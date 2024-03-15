// use env_logger::Env;
use clap::Parser;
use gevulot_shim::{Task, TaskResult};
use prover::shared_state::SharedState;
use serde_json::json;
use std::fs;
use std::fs::write;
use std::io;
use std::{error::Error, result::Result};
use zkevm_common::prover::*;

fn main() -> Result<(), Box<dyn Error>> {
    println!("taiko_prover main()");
    gevulot_shim::run(run_task)
}

// The main function that executes the prover program.
fn run_task(task: Task) -> Result<TaskResult, Box<dyn Error>> {
    println!("run_task()");

    // to synchronize argument parsing
    let mut args = vec!["dummy".to_string()];
    for a in task.args.clone() {
        args.push(a);
    }

    println!("taiko prover: args: {:?}", args);

    let proof_path = taiko_prover(&args)?;

    println!("exit prover run_task");

    // Return TaskResult with reference to the generated proof file.
    task.result(vec![], vec![proof_path])
}

#[derive(Parser, Debug)]
#[clap(author = "Taiko Prover", version, about, long_about = None)]
pub struct TaikoProverConfig {
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

// #[tokio::main]
fn taiko_prover(args: &Vec<String>) -> Result<String, Box<dyn Error>> {
    let arg_conf = TaikoProverConfig::parse_from(args);

    let entries = fs::read_dir(".")
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    println!(
        "taiko_prover file entries in root directory :: {:?}",
        entries
    );

    let entries = fs::read_dir("/workspace")
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    println!(
        "taiko_prover file entries at directory /workspace :: {:?}",
        entries
    );

    let entries = fs::read_dir("/gevulot")
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    println!(
        "taiko_prover file entries at directory /gevulot :: {:?}",
        entries
    );

    // set our arguments, use defaults as applicable
    let params_path = arg_conf.kparams_path;
    let proof_path = arg_conf.proof_path;
    let witness_path = arg_conf.witness_path;

    println!("params_path: {:?}", params_path);
    println!("proof_path: {:?}", proof_path);
    println!("witness_path: {:?}", witness_path);

    if witness_path.is_none() {
        return Err(String::from("no witness file parameter").into());
    }
    if params_path.is_none() {
        return Err(String::from("no parameters file parameter").into());
    }
    if proof_path.is_none() {
        return Err(String::from("no proof file parameter").into());
    }

    // now set dummy RPC url and block number which will not be used.
    let rpc_url = "http://dummy.com".to_string();
    let block_num: u64 = 0;

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
        prover_mode: ProverMode::OfflineProver,
        rpc: rpc_url,
        retry: false,
        param: params_path,
        witness_path,
        proof_path: proof_path.clone(),
        protocol_instance,
        mock: false,
        aggregate: true,
        verify_proof: true,
        ..Default::default()
    };

    let proofs = state.prove(&request)?;
    let proof_path = proof_path.unwrap();

    let jproof = json!(proofs).to_string();
    println!("taiko prover, write proof string, {:?} bytes", jproof.len());
    println!("taiko prover, proof_path, {:?} bytes", proof_path);
    write(proof_path.clone(), jproof).unwrap();
    Ok(proof_path)
}
