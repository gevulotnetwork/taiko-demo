use bus_mapping::circuit_input_builder::{
    protocol_instance::{BlockMetadata, Transition},
    ProtocolInstance,
};
use eth_types::{Address, Bytes, H256};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ProofResult {
    /// The halo2 transcript
    pub proof: Bytes,
    /// Public inputs for the proof
    pub instance: Vec<String>,
    /// k of circuit parameters
    pub k: u8,
    /// Randomness used
    pub randomness: Bytes,
    /// Circuit name / identifier
    pub label: String,
    /// Auxiliary
    pub aux: ProofResultInstrumentation,
}

impl std::fmt::Debug for ProofResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Proof")
            .field("proof", &format!("{}", &self.proof))
            .field("instance", &self.instance)
            .field("k", &self.k)
            .field("randomness", &format!("{}", &self.randomness))
            .field("aux", &format!("{:#?}", self.aux))
            .finish()
    }
}

/// Timing information in milliseconds.
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ProofResultInstrumentation {
    /// keygen_vk
    pub vk: u32,
    /// keygen_pk
    pub pk: u32,
    /// create_proof
    pub proof: u32,
    /// verify_proof
    pub verify: u32,
    /// MockProver.verify_par
    pub mock: u32,
    /// Circuit::new
    pub circuit: u32,
    /// RootCircuit::compile
    pub protocol: u32,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Proofs {
    /// Circuit configuration used
    pub config: CircuitConfig,
    // Proof result for circuit
    pub circuit: ProofResult,
    /// Aggregation proof for circuit, if requested
    pub aggregation: ProofResult,
    /// Gas used. Determines the upper ceiling for circuit parameters
    pub gas: u64,
    /// byte code used for evm verifier
    pub bytecode: Bytes,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct VerificationKey {
    /// Circuit configuration used
    pub bytecode: Vec<u8>,
    // Public inputs
    pub instance: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
// request extra instance corresponding to ProtocolInstance
pub struct RequestExtraInstance {
    /// l1 signal service address
    pub l1_signal_service: String,
    /// l2 signal service address
    pub l2_signal_service: String,
    /// l2 contract address
    pub l2_contract: String,
    /// meta hash
    pub request_meta_data: RequestMetaData,
    /// block hash value
    pub block_hash: String,
    /// the parent block hash
    pub parent_hash: String,
    /// signal root
    pub signal_root: String,
    /// extra message
    pub graffiti: String,
    /// Prover address
    pub prover: String,
    /// treasury
    pub treasury: String,
    /// gas used
    pub gas_used: u32,
    /// parent gas used
    pub parent_gas_used: u32,
    /// blockMaxGasLimit
    pub block_max_gas_limit: u64,
    /// maxTransactionsPerBlock
    pub max_transactions_per_block: u64,
    /// maxBytesPerTxList
    pub max_bytes_per_tx_list: u64,
    /// anchor_gas_limit
    pub anchor_gas_limit: u64,
}

/// l1 meta hash
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestMetaData {
    /// meta id
    pub id: u64,
    /// meta timestamp
    pub timestamp: u64,
    /// l1 block height
    pub l1_height: u64,
    /// l1 block hash
    pub l1_hash: String,
    /// deposits processed
    pub deposits_hash: String,
    /// tx list hash
    pub blob_hash: String,
    /// tx list byte start
    pub tx_list_byte_offset: u32, // u24
    /// tx list byte end
    pub tx_list_byte_size: u32, // u24
    /// gas limit
    pub gas_limit: u32,
    /// coinbase
    pub coinbase: String,
    // difficulty
    pub difficulty: String,
    // extraData
    pub extra_data: String,
    // minTier
    pub min_tier: u16,
    // blobUsed
    pub blob_used: bool,
    /// previous meta hash
    pub parent_metahash: String,
}

impl PartialEq for RequestExtraInstance {
    fn eq(&self, other: &Self) -> bool {
        self.l1_signal_service == other.l1_signal_service
            && self.l2_signal_service == other.l2_signal_service
            && self.l2_contract == other.l2_contract
            && self.request_meta_data == other.request_meta_data
            && self.block_hash == other.block_hash
            && self.parent_hash == other.parent_hash
            && self.signal_root == other.signal_root
            && self.graffiti == other.graffiti
            && self.prover == other.prover
            && self.gas_used == other.gas_used
            && self.parent_gas_used == other.parent_gas_used
            && self.block_max_gas_limit == other.block_max_gas_limit
            && self.max_transactions_per_block == other.max_transactions_per_block
            && self.max_bytes_per_tx_list == other.max_bytes_per_tx_list
    }
}

fn parse_hash(input: &str) -> [u8; 32] {
    H256::from_slice(&hex::decode(input).expect("parse_hash"))
        .as_fixed_bytes()
        .clone()
}

fn parse_address(input: &str) -> Address {
    Address::from_slice(&hex::decode(input).expect("parse_address"))
}

impl From<RequestExtraInstance> for ProtocolInstance {
    fn from(instance: RequestExtraInstance) -> Self {
        ProtocolInstance {
            transition: Transition {
                parentHash: parse_hash(&instance.parent_hash).into(),
                blockHash: parse_hash(&instance.block_hash).into(), // constrain: l2 block hash
                signalRoot: parse_hash(&instance.signal_root).into(), // constrain: ??l2 service account storage root??
                graffiti: parse_hash(&instance.graffiti).into(),
            },
            block_metadata: BlockMetadata {
                l1Hash: parse_hash(&instance.request_meta_data.l1_hash).into(),
                difficulty: parse_hash(&instance.request_meta_data.difficulty).into(),
                blobHash: parse_hash(&instance.request_meta_data.blob_hash).into(),
                extraData: parse_hash(&instance.request_meta_data.extra_data).into(),
                depositsHash: parse_hash(&instance.request_meta_data.deposits_hash).into(),
                coinbase: parse_address(&instance.request_meta_data.coinbase)
                    .to_fixed_bytes()
                    .into(),
                id: instance.request_meta_data.id,
                gasLimit: instance.request_meta_data.gas_limit,
                timestamp: instance.request_meta_data.timestamp,
                l1Height: instance.request_meta_data.l1_height,
                txListByteOffset: instance.request_meta_data.tx_list_byte_offset,
                txListByteSize: instance.request_meta_data.tx_list_byte_size,
                minTier: instance.request_meta_data.min_tier,
                blobUsed: instance.request_meta_data.blob_used,
                parentMetaHash: parse_hash(&instance.request_meta_data.parent_metahash).into(),
            },
            prover: parse_address(&instance.prover),
        }
    }
}

#[derive(PartialEq, Clone, Debug, Copy, Serialize, Deserialize, Default)]
pub enum ProverMode {
    WitnessCapture,
    OfflineProver,
    #[default]
    LegacyProver,
    Verifier,
}
impl From<&str> for ProverMode {
    fn from(input: &str) -> ProverMode {
        match input {
            "witness_capture" => ProverMode::WitnessCapture,
            "offline_prover" => ProverMode::OfflineProver,
            "legacy_prover" => ProverMode::LegacyProver,
            "verifier" => ProverMode::Verifier,
            _ => panic!("invalid mode string: {input}"),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProofRequestOptions {
    /// The name of the circuit.
    /// "super", "pi"
    pub circuit: String,
    /// the block number
    pub block: u64,
    /// prover mode
    pub prover_mode: ProverMode,
    /// the l2 rpc url
    pub rpc: String,
    /// the protocol instance data
    pub protocol_instance: RequestExtraInstance,
    /// retry proof computation if error
    pub retry: bool,
    /// Parameters file or directory to use.
    /// Otherwise generates them on the fly.
    pub param: Option<String>,
    /// Witness file to serialize
    pub witness_path: Option<String>,
    /// Proof file to serialize
    pub proof_path: Option<String>,
    /// Only use MockProver if true.
    #[serde(default = "default_bool")]
    pub mock: bool,
    /// Additionaly aggregates the circuit proof if true
    #[serde(default = "default_bool")]
    pub aggregate: bool,
    /// Runs the MockProver if proofing fails.
    #[serde(default = "default_bool")]
    pub mock_feedback: bool,
    /// Verifies the proof after computation.
    #[serde(default = "default_bool")]
    pub verify_proof: bool,
}

impl PartialEq for ProofRequestOptions {
    fn eq(&self, other: &Self) -> bool {
        self.block == other.block
            && self.protocol_instance == other.protocol_instance
            && self.rpc == other.rpc
            && self.param == other.param
            && self.circuit == other.circuit
            && self.mock == other.mock
            && self.aggregate == other.aggregate
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRequest {
    pub options: ProofRequestOptions,
    pub result: Option<Result<Proofs, String>>,
    /// A counter to keep track of changes of the `result` field
    pub edition: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInformation {
    pub id: String,
    pub tasks: Vec<ProofRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStatus {
    pub id: String,
    /// The current active task this instance wants to obtain or is working on.
    pub task: Option<ProofRequestOptions>,
    /// `true` if this instance started working on `task`
    pub obtained: bool,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct CircuitConfig {
    pub block_gas_limit: usize,
    pub max_txs: usize,
    pub max_calldata: usize,
    pub max_bytecode: usize,
    pub max_rws: usize,
    pub max_copy_rows: usize,
    pub max_exp_steps: usize,
    pub min_k: usize,
    pub pad_to: usize,
    pub min_k_aggregation: usize,
    pub keccak_padding: usize,
}

fn default_bool() -> bool {
    false
}
