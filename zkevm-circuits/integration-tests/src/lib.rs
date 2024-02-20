//! Integration testing

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]

use bus_mapping::{
    circuit_input_builder::{
        build_state_code_db, get_state_accesses, BuilderClient, CircuitsParams,
    },
    rpc::GethClient,
};
use env_logger::Env;
use eth_types::Address;
use ethers::{
    abi,
    core::{k256::ecdsa::SigningKey, types::Bytes},
    providers::{Http, Provider},
    signers::{coins_bip39::English, MnemonicBuilder, Signer, Wallet},
};
use lazy_static::lazy_static;
use log::trace;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env::{self, VarError},
    fs::File,
    sync::Once,
    time::Duration,
};
use url::Url;

/// Geth dev chain ID
pub const CHAIN_ID: u64 = 1337;
/// Path to the test contracts
pub const CONTRACTS_PATH: &str = "contracts";
/// List of contracts as (ContractName, ContractSolidityFile)
pub const CONTRACTS: &[(&str, &str)] = &[
    ("Greeter", "greeter/Greeter.sol"),
    (
        "OpenZeppelinERC20TestToken",
        "ERC20/OpenZeppelinERC20TestToken.sol",
    ),
];
/// Path to gen_blockchain_data output file
pub const GENDATA_OUTPUT_PATH: &str = "gendata_output.json";

const GETH0_URL_DEFAULT: &str = "http://localhost:8545";

lazy_static! {
    /// URL of the integration test geth0 instance, which contains blocks for which proofs will be
    /// generated.
    pub static ref GETH0_URL: String = match env::var("GETH0_URL") {
        Ok(val) => val,
        Err(VarError::NotPresent) => GETH0_URL_DEFAULT.to_string(),
        Err(e) => panic!("Error in GETH0_URL env var: {:?}", e),
    };

    /// create GEN_DATA
    pub static ref GEN_DATA: GenDataOutput = GenDataOutput::load();
}

static LOG_INIT: Once = Once::new();

/// Initialize log
pub fn log_init() {
    LOG_INIT.call_once(|| {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    });
}

/// Get the integration test [`GethClient`]
pub fn get_client() -> GethClient<Http> {
    let transport = Http::new(Url::parse(&GETH0_URL).expect("invalid url"));
    GethClient::new(transport)
}

/// Get the integration test [`Provider`]
pub fn get_provider() -> Provider<Http> {
    let transport = Http::new(Url::parse(&GETH0_URL).expect("invalid url"));
    Provider::new(transport).interval(Duration::from_millis(100))
}

/// Get the chain id by querying the geth client.
pub async fn get_chain_id() -> u64 {
    let client = get_client();
    client.get_chain_id().await.unwrap()
}

const PHRASE: &str =
    "work man father plunge mystery proud hollow address reunion sauce theory bonus";

/// Get a wallet by index
pub fn get_wallet(index: u32) -> Wallet<SigningKey> {
    // Access mnemonic phrase.
    // Child key at derivation path: m/44'/60'/0'/0/{index}
    MnemonicBuilder::<English>::default()
        .phrase(PHRASE)
        .index(index)
        .expect("invalid index")
        .build()
        .expect("cannot build wallet from mnemonic")
        .with_chain_id(CHAIN_ID)
}

/// Output information of the blockchain data generated by
/// `gen_blockchain_data`.
#[derive(Serialize, Deserialize)]
pub struct GenDataOutput {
    /// Coinbase of the blockchain
    pub coinbase: Address,
    /// Wallets used by `gen_blockchain_data`
    pub wallets: Vec<Address>,
    /// Block map: BlockContent -> BlockNum
    pub blocks: HashMap<String, u64>,
    /// Contracts deployed map: ContractName -> (BlockNum, Address)
    pub deployments: HashMap<String, (u64, Address)>,
}

impl GenDataOutput {
    /// Load [`GenDataOutput`] from the json file.
    pub fn load() -> Self {
        serde_json::from_reader(File::open(GENDATA_OUTPUT_PATH).expect("cannot read file"))
            .expect("cannot deserialize json from file")
    }

    /// Store [`GenDataOutput`] into the json file.
    pub fn store(&self) {
        serde_json::to_writer(
            &File::create(GENDATA_OUTPUT_PATH).expect("cannot create file"),
            self,
        )
        .expect("cannot serialize json into file");
    }
}

/// Solc-compiled contract output
#[derive(Serialize, Deserialize)]
pub struct CompiledContract {
    /// Contract path
    pub path: String,
    /// Contract name
    pub name: String,
    /// ABI
    pub abi: abi::Contract,
    /// Bytecode
    pub bin: Bytes,
    /// Runtime Bytecode
    pub bin_runtime: Bytes,
}

/// Build circuit input builder for a block
pub async fn build_circuit_input_builder_block(block_num: u64) {
    let cli = get_client();
    let cli = BuilderClient::new(
        cli,
        CircuitsParams {
            max_rws: 800000,
            max_txs: 10,
            max_calldata: 4000,
            max_bytecode: 4000,
            max_copy_rows: 800000,
            max_evm_rows: 0,
            max_exp_steps: 1000,
            max_keccak_rows: 0,
        },
        Default::default(),
    )
    .await
    .unwrap();

    // 1. Query geth for Block, Txs and TxExecTraces
    let (eth_block, geth_trace, history_hashes, prev_state_root) =
        cli.get_block(block_num).await.unwrap();

    // 2. Get State Accesses from TxExecTraces
    let access_set = get_state_accesses(&eth_block, &geth_trace, &None).unwrap();
    trace!("AccessSet: {:#?}", access_set);

    // 3. Query geth for all accounts, storage keys, and codes from Accesses
    let (proofs, codes) = cli.get_state(block_num, access_set).await.unwrap();

    // 4. Build a partial StateDB from step 3
    let (state_db, code_db) = build_state_code_db(proofs, codes);
    trace!("StateDB: {:#?}", state_db);

    // 5. For each step in TxExecTraces, gen the associated ops and state
    // circuit inputs
    let builder = cli
        .gen_inputs_from_state(
            state_db,
            code_db,
            &eth_block,
            &geth_trace,
            history_hashes,
            prev_state_root,
        )
        .unwrap();

    trace!("CircuitInputBuilder: {:#?}", builder);
}

/// Block explorer URL is https://explorer.internal.taiko.xyz
/// The block that has only one anchor
pub const TAIKO_BLOCK_ANCHOR_ONLY: u64 = 5368;
/// The block that has ERC20 transfer
pub const TAIKO_BLOCK_TRANSFER_SUCCEED: u64 = 1270;

/// Common code for integration tests of circuits.
pub mod integration_test_circuits;

/// Common code for integration public data tests of circuits.
pub mod integration_public_data_circuits;