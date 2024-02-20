use crate::Fr;
use bus_mapping::circuit_input_builder::Block;
use bus_mapping::circuit_input_builder::BuilderClient;
use bus_mapping::circuit_input_builder::CircuitsParams;
use bus_mapping::circuit_input_builder::ProtocolInstance;
use bus_mapping::mock::BlockData;
use bus_mapping::rpc::GethClient;
use eth_types::geth_types;
use eth_types::geth_types::GethData;
use eth_types::Address;
use eth_types::ToBigEndian;
use eth_types::Word;
use eth_types::H256;
use ethers_providers::Http;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use zkevm_circuits::evm_circuit;
use zkevm_circuits::pi_circuit::PublicData;
use zkevm_common::prover::ProofRequestOptions;
use zkevm_common::prover::{CircuitConfig, RequestExtraInstance};

/// Wrapper struct for circuit witness data.
#[derive(Serialize, Deserialize)]

pub struct CircuitWitness {
    pub circuit_config: CircuitConfig,
    pub eth_block: eth_types::Block<eth_types::Transaction>,
    pub block: bus_mapping::circuit_input_builder::Block,
    // dummy block for real data
    pub dummy_block: Option<bus_mapping::circuit_input_builder::Block>,
    pub code_db: bus_mapping::state_db::CodeDB,
    pub protocol_instance: ProtocolInstance,
}

impl CircuitWitness {
    pub fn dummy(circuit_config: CircuitConfig) -> Result<Self, String> {
        let history_hashes = vec![Word::zero(); 256];
        let mut eth_block: eth_types::Block<eth_types::Transaction> = eth_types::Block::default();
        eth_block.mix_hash = Some(H256::zero());
        eth_block.author = Some(Address::zero());
        eth_block.number = Some(history_hashes.len().into());
        eth_block.base_fee_per_gas = Some(0.into());
        eth_block.hash = Some(eth_block.parent_hash);
        eth_block.gas_limit = circuit_config.block_gas_limit.into();

        let circuit_params = CircuitsParams {
            max_txs: circuit_config.max_txs,
            max_calldata: circuit_config.max_calldata,
            max_bytecode: circuit_config.max_bytecode,
            max_rws: circuit_config.max_rws,
            max_copy_rows: circuit_config.max_copy_rows,
            max_exp_steps: circuit_config.max_exp_steps,
            max_evm_rows: circuit_config.pad_to,
            max_keccak_rows: circuit_config.keccak_padding,
        };
        let empty_data = GethData {
            chain_id: Word::from(99),
            history_hashes: vec![Word::zero(); 256],
            eth_block,
            geth_traces: Vec::new(),
            accounts: Vec::new(),
        };
        let mut builder =
            BlockData::new_from_geth_data_with_params(empty_data.clone(), circuit_params)
                .new_circuit_input_builder();
        builder
            .handle_block(&empty_data.eth_block, &empty_data.geth_traces)
            .unwrap();
        Ok(Self {
            circuit_config,
            eth_block: empty_data.eth_block,
            block: builder.block,
            dummy_block: None,
            code_db: builder.code_db,
            protocol_instance: ProtocolInstance::default(),
        })
    }

    pub async fn dummy_with_request(request: &ProofRequestOptions) -> Result<Self, String> {
        let url = Http::from_str(&request.rpc).map_err(|e| e.to_string())?;
        let geth_client = GethClient::new(url);
        let chain_id = geth_client
            .get_chain_id()
            .await
            .map_err(|e| e.to_string())?;
        let block = geth_client
            .get_block_by_number((request.block).into())
            .await
            .map_err(|e| e.to_string())?;
        let circuit_config =
            crate::match_circuit_params!(block.gas_used.as_usize(), CIRCUIT_CONFIG, {
                return Err(format!(
                    "No circuit parameters found for block with gas used={}",
                    block.gas_used
                )
                .into());
            });

        let pi: ProtocolInstance = request.protocol_instance.clone().into();
        let circuits_params = CircuitsParams {
            max_txs: circuit_config.max_txs,
            max_calldata: circuit_config.max_calldata,
            max_bytecode: circuit_config.max_bytecode,
            max_rws: circuit_config.max_rws,
            max_copy_rows: circuit_config.max_copy_rows,
            max_exp_steps: circuit_config.max_exp_steps,
            max_evm_rows: circuit_config.pad_to,
            max_keccak_rows: circuit_config.keccak_padding,
        };
        let builder = BuilderClient::new(geth_client, circuits_params, Some(pi.clone()))
            .await
            .map_err(|e| e.to_string())?;

        let (eth_block, _, history_hashes, prev_state_root) = builder
            .get_block(request.block.into())
            .await
            .map_err(|e| e.to_string())?;
        let (builder, _eth_block) = builder
            .gen_inputs(request.block)
            .await
            .map_err(|e| e.to_string())?;
        let mut w = Self::dummy(circuit_config)?;
        w.protocol_instance = request.protocol_instance.clone().into();
        w.block = builder.block;
        w.code_db = builder.code_db;
        w.eth_block = eth_block;

        let dummy_block = Block::new(
            chain_id.into(),
            history_hashes,
            prev_state_root,
            &w.eth_block,
            circuits_params,
            Some(pi.clone()),
        )
        .map_err(|e| e.to_string())?;
        w.dummy_block = Some(dummy_block);
        Ok(w)
    }

    pub async fn from_request(
        request: &mut ProofRequestOptions,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut w =
            Self::from_rpc(&request.block, &request.rpc, &mut request.protocol_instance).await?;
        w.protocol_instance = request.protocol_instance.clone().into();
        Ok(w)
    }

    /// Gathers debug trace(s) from `rpc_url` for block `block_num`.
    /// Expects a go-ethereum node with debug & archive capabilities on `rpc_url`.
    pub async fn from_rpc(
        block_num: &u64,
        rpc_url: &str,
        pi: &mut RequestExtraInstance,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let url = Http::from_str(rpc_url)?;
        let geth_client = GethClient::new(url);
        // TODO: add support for `eth_getHeaderByNumber`
        let block = geth_client.get_block_by_number((*block_num).into()).await?;

        let hash = format!("{:?}", block.hash.unwrap()).as_str()[2..].to_string();
        let parent_hash = format!("{:?}", block.parent_hash).as_str()[2..].to_string();

        pi.block_hash = hash;
        pi.parent_hash = parent_hash;

        #[cfg(feature = "eip-1559-only")]
        Self::validate_proverable_block(&block)?;

        let circuit_config =
            crate::match_circuit_params!(block.gas_used.as_usize(), CIRCUIT_CONFIG, {
                return Err(format!(
                    "No circuit parameters found for block with gas used={}",
                    block.gas_used
                )
                .into());
            });
        let circuit_params = CircuitsParams {
            max_txs: circuit_config.max_txs,
            max_calldata: circuit_config.max_calldata,
            max_bytecode: circuit_config.max_bytecode,
            max_rws: circuit_config.max_rws,
            max_copy_rows: circuit_config.max_copy_rows,
            max_exp_steps: circuit_config.max_exp_steps,
            max_evm_rows: circuit_config.pad_to,
            max_keccak_rows: circuit_config.keccak_padding,
        };
        // println!("*** CircuitsParams {:?}", circuit_config);
        // println!("*** pi {:?}", pi);

        let pi: ProtocolInstance = pi.clone().into();
        let builder = BuilderClient::new(geth_client, circuit_params, Some(pi.clone())).await?;
        let (builder, eth_block) = builder.gen_inputs(*block_num).await?;

        Ok(Self {
            circuit_config,
            eth_block,
            block: builder.block,
            dummy_block: None,
            code_db: builder.code_db,
            protocol_instance: pi,
        })
    }

    pub fn evm_witness(&self) -> zkevm_circuits::witness::Block<Fr> {
        let mut block =
            evm_circuit::witness::block_convert(&self.block, &self.code_db).expect("block_convert");
        block.exp_circuit_pad_to = self.circuit_config.pad_to;
        // fixed randomness used in PublicInput contract and SuperCircuit
        block.randomness = Fr::from(0x100);

        // fill protocol instance
        block.protocol_instance = Some(self.protocol_instance.clone());
        block
    }

    pub fn dummy_evm_witness(&self) -> zkevm_circuits::witness::Block<Fr> {
        let mut block =
            evm_circuit::witness::block_convert(&self.block, &self.code_db).expect("block_convert");
        block.exp_circuit_pad_to = self.circuit_config.pad_to;
        // fixed randomness used in PublicInput contract and SuperCircuit
        block.randomness = Fr::from(0x100);

        if let Some(block_data) = &self.dummy_block {
            block.context = (block_data).into();
        };

        // fill protocol instance
        block.protocol_instance = Some(self.protocol_instance.clone());
        block
    }

    pub fn gas_used(&self) -> u64 {
        self.eth_block.gas_used.as_u64()
    }

    pub fn txs(&self) -> Vec<geth_types::Transaction> {
        let txs = self
            .eth_block
            .transactions
            .iter()
            .map(geth_types::Transaction::from)
            .collect();

        txs
    }

    pub fn public_data(&self) -> PublicData {
        let chain_id = self.block.chain_id;
        let eth_block = self.eth_block.clone();
        let history_hashes = self.block.history_hashes.clone();
        let block_constants = geth_types::BlockConstants {
            coinbase: eth_block.author.expect("coinbase"),
            timestamp: eth_block.timestamp,
            number: eth_block.number.expect("number"),
            mix_hash: eth_block.mix_hash.expect("mix_hash"),
            gas_limit: eth_block.gas_limit,
            base_fee: eth_block.base_fee_per_gas.unwrap_or_default(),
        };
        let prev_state_root = H256::from(self.block.prev_state_root.to_be_bytes());

        PublicData {
            chain_id,
            history_hashes,
            block_constants,
            prev_state_root,
            transactions: eth_block.transactions.clone(),
            // block_hash: eth_block.hash.unwrap_or_default(),
            state_root: eth_block.state_root,
        }
    }

    #[cfg(feature = "eip-1559-only")]
    fn validate_proverable_block(
        block: &eth_types::Block<eth_types::Transaction>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if block.transactions.iter().any(|tx| {
            tx.transaction_type != Some(2u64.into())
                || (tx.access_list.is_some() && !tx.access_list.as_ref().unwrap().0.is_empty())
        }) {
            return Err("unsupported block".into());
        }

        Ok(())
    }
}

mod test {
    #[allow(unused_imports)]
    use super::*;

    #[tokio::test]
    async fn test_geth_client() {
        let urlstr = "http://localhost:8545";
        let url = Http::from_str(urlstr).unwrap();
        let geth_client = GethClient::new(url);
        let block = geth_client
            .get_block_by_number(102296.into())
            .await
            .unwrap();
        println!("{:?}", block);
    }
}
