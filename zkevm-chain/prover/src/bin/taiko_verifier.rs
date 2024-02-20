// use env_logger::Env;
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

    let mut new_args = vec!["dummy".to_string()];
    for a in task.args.clone() {
        new_args.push(a);
    }

    // Display program arguments we received. These could be used for
    // e.g. parsing CLI arguments with clap.
    println!("taiko verifier: new_args: {:?}", &new_args);

    let result = verifier_cmd(&new_args);
    let result: String = match result {
        true => "Taiko verifier result: success".to_string(),
        false => "Taiko verifier result: fail".to_string(),
    };
    // -----------------------------------------------------------------------
    // Here would be the control logic to run the prover with given arguments.
    // -----------------------------------------------------------------------

    println!("done with taiko verification, result is {:?}", result);

    // Write generated proof to a file.
    // std::fs::write("/workspace/verify.dat", result)?;
    println!("exit verifier run_task");

    // Return TaskResult
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

    println!("file entries at directory . :: {:?}", entries);

    let entries = fs::read_dir("/workspace")
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    println!("file entries at directory /workspace :: {:?}", entries);

    // let derived_proof =
    //     entries.get(0).unwrap().to_str().unwrap().to_string() + &proof_path.clone().unwrap();

    // println!("derived_proof: {:?}", derived_proof);

    // let jproof = std::fs::read_to_string(derived_proof).unwrap();
    println!("use proof_path!");
    let jproof = std::fs::read_to_string(proof_path.unwrap()).unwrap();

    let proofs: Proofs = serde_json::from_str(&jproof).unwrap();
    let result = verify(proofs);

    result
}
