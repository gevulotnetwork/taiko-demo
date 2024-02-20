use clap::Parser;
use gevulot_shim::{Task, TaskResult};
use prover::shared_state::verify;
use std::{error::Error, fs, io, result::Result};
use zkevm_common::prover::*;

fn main() -> Result<(), Box<dyn Error>> {
    println!("main()");
    gevulot_shim::run(run_task)
}

// The main function that executes the prover program.
fn run_task(task: &Task) -> Result<TaskResult, Box<dyn Error>> {
    println!("run_task()");

    let mut args = vec!["dummy".to_string()];
    for a in task.args.clone() {
        args.push(a);
    }
    println!("taiko verifier: args: {:?}", &args);

    let result = verifier_cmd(&args);
    let result: String = match result {
        true => "Taiko verifier result: success".to_string(),
        false => "Taiko verifier result: fail".to_string(),
    };

    println!("done with taiko verification, result is {:?}", result);
    println!("exit taiko_verifier task");
    task.result(vec![], vec![])
}

#[derive(Parser, Debug)]
#[clap(author = "Taiko Verifier", version, about, long_about = None)]
pub struct TaikoVerifierConfig {
    /// Required for offline_prover, legacy_prover, and verifier
    #[clap(short, long, value_parser, verbatim_doc_comment)]
    pub proof_path: Option<String>,
}

// #[tokio::main]
fn verifier_cmd(args: &Vec<String>) -> bool {
    println!("verifier_cmd");
    let arg_conf = TaikoVerifierConfig::parse_from(args);

    let proof_path = arg_conf.proof_path;

    let entries = fs::read_dir(".")
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    println!("file entries in root directory :: {:?}", entries);

    let entries = fs::read_dir("/workspace")
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    println!("file entries in /workspace :: {:?}", entries);

    let jproof = std::fs::read_to_string(proof_path.unwrap()).unwrap();
    let proofs: Proofs = serde_json::from_str(&jproof).unwrap();
    let result = verify(proofs);

    result
}
