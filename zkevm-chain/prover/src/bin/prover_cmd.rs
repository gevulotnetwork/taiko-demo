use clap::Parser;
use prover::shared_state::SharedState;
use zkevm_common::prover::*;

#[derive(Parser, Debug)]
#[clap(author = "Taiko Prover", version, about, long_about = None)]
pub struct ArgConfiguration {
    /// witness_capture | offline_prover | legacy_prover | verifier
    #[clap(value_parser)]
    pub mode: ProverMode,
    /// Required for witness_capture and legacy_prover
    #[clap(short, long, value_parser)]
    pub block_num: Option<u64>,
    /// Url of L2 Taiko node, required for witness_capture and legacy_prover
    #[clap(short, long, value_parser)]
    pub rpc_url: Option<String>,
    /// Required for offline_prover, legacy_prover, and verifier
    #[clap(short, long, value_parser, verbatim_doc_comment)]
    pub proof_path: Option<String>,
    /// Required for witness_capture and offline_prover
    #[clap(short, long, value_parser)]
    pub witness_path: Option<String>,
    /// Required for witness_capture, offline_prover, legacy_prover
    #[clap(short, long, value_parser)]
    pub kparams_path: Option<String>,
}

#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    let arg_conf = ArgConfiguration::parse_from(&args);

    // set our arguments, use defaults as applicable
    let block_num = arg_conf.block_num;
    let params_path = arg_conf.kparams_path;
    let proof_path = arg_conf.proof_path;
    let prover_mode = arg_conf.mode;
    let rpc_url = arg_conf.rpc_url;
    let witness_path = arg_conf.witness_path;

    println!("block_num: {:?}", block_num);
    println!("params_path: {:?}", params_path);
    println!("prover_mode: {:?}", prover_mode);
    println!("proof_path: {:?}", proof_path);
    println!("rpc_url: {:?}", rpc_url);
    println!("witness_path: {:?}", witness_path);

    // check args for each mode
    match prover_mode {
        ProverMode::WitnessCapture => {
            assert!(block_num.is_some(), "pass in a block number");
            assert!(params_path.is_some(), "pass in a kparams file");
            assert!(rpc_url.is_some(), "pass in an L2 RPC url");
            assert!(witness_path.is_some(), "pass in a witness file for output");
        }
        ProverMode::OfflineProver => {
            assert!(params_path.is_some(), "pass in a kparams file");
            assert!(proof_path.is_some(), "pass in a proof file for output");
            assert!(witness_path.is_some(), "pass in a witness file for input");
        }
        ProverMode::LegacyProver => {
            assert!(block_num.is_some(), "pass in a block_num");
            assert!(params_path.is_some(), "pass in a kparams file");
            assert!(rpc_url.is_some(), "pass in an L2 RPC url");
        }
        ProverMode::Verifier => {
            assert!(proof_path.is_some(), "pass in a proof file for input");
        }
    }

    // now set dummy RPC url and block number which will not be used.
    let rpc_url = rpc_url.unwrap_or("http://dummy.com".to_string());
    let block_num = block_num.unwrap_or(0);

    // mock a RequestExtraInstance struct
    // the block_hash and parent_hash will get overwritten with real values
    // when the eth_block is first read in.
    let protocol_instance = RequestExtraInstance {
        l1_signal_service: "7a2088a1bFc9d81c55368AE168C2C02570cB814F".to_string(),
        l2_signal_service: "1000777700000000000000000000000000000007".to_string(),
        l2_contract: "1000777700000000000000000000000000000001".to_string(),
        request_meta_data: RequestMetaData {
            id: 57437,
            timestamp: 1706084004,
            l1_height: 800044,
            l1_hash: "db92d81a16dcd5b684bdf420ef71e3e31b40cda81a43563f9638c9c0390790e4".to_string(),
            deposits_hash: "569e75fc77c1a856f6daaf9e69d8a9566ca34aa47f9133711ce065a571af0cfd"
                .to_string(),
            blob_hash: "94e0d3a174ecf52f531eba85ee1a01f77fccae3f352b4be7ed68bb2cac4969ee"
                .to_string(),
            tx_list_byte_offset: 0,
            tx_list_byte_size: 28706,
            gas_limit: 15000000,
            coinbase: "e1e210594771824dad216568b91c9cb4ceed361c".to_string(),
            difficulty: "055c829a0e081185e57b66ff8fbb0fbf06f5fc58e226764419224495d3036a00"
                .to_string(),
            extra_data: "302e31382e302d64657600000000000000000000000000000000000000000000"
                .to_string(),
            parent_metahash: "e5c39fecba3dca4aec370e5005fbecac568705f859871794e4f4090c276936fa"
                .to_string(),
            ..Default::default()
        },
        block_hash: "930e1b7bc4c8354614b0c76aea5c5dc6b6797d6e21ccda43e228cd0cef773490".to_string(),
        parent_hash: "d6cf6f0c98d11e9e955d97ebd477282831d2f11f55ee13354f2134afc7f85429".to_string(),
        signal_root: "5c9572d9ec31784e01a393dc17b7ff0786b6534bcfa14715effd02d29222dbf9".to_string(),
        graffiti: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        prover: "ee85e2fe0e26891882a8CD744432d2BBFbe140dd".to_string(),
        treasury: "0x1670080000000000000000000000000000010001".to_string(),
        gas_used: 0,
        parent_gas_used: 0,
        block_max_gas_limit: 6000000,
        max_transactions_per_block: 79,
        max_bytes_per_tx_list: 120000,
        anchor_gas_limit: 250000,
    };

    let state = SharedState::new(String::new(), None);
    let request = ProofRequestOptions {
        circuit: "super".to_string(),
        block: block_num,
        prover_mode,
        rpc: rpc_url,
        retry: false,
        param: params_path,
        witness_path,
        proof_path,
        protocol_instance,
        mock: false,
        aggregate: true,
        verify_proof: true,
        ..Default::default()
    };

    state.get_or_enqueue(&request).await;
    state.duty_cycle().await;
    let _result = state.get_or_enqueue(&request).await;
}
