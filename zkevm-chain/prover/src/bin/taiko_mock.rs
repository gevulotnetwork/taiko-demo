// use env_logger::Env;
use clap::Parser;
use gevulot_shim::{Task, TaskResult};
use std::fs;
use std::fs::write;
use std::io;
use std::{error::Error, result::Result};

fn main() -> Result<(), Box<dyn Error>> {
    println!("main()");
    gevulot_shim::run(run_task)
}

fn run_task(task: &Task) -> Result<TaskResult, Box<dyn Error>> {
    println!("run_task()");

    let mut args = vec!["dummy".to_string()];
    for a in task.args.clone() {
        args.push(a);
    }

    println!("taiko prover: args: {:?}", args);

    let proof_path = prover_mock(&args);

    println!("exit taiko_mock task");

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
fn prover_mock(args: &Vec<String>) -> String {
    let arg_conf = TaikoProverConfig::parse_from(args);
    println!("prover_mock");

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

    println!("file entries at directory /workspace :: {:?}", entries);

    // set our arguments, use defaults as applicable
    let params_path = arg_conf.kparams_path;
    let proof_path = arg_conf.proof_path;
    let witness_path = arg_conf.witness_path;

    assert!(params_path.is_some(), "pass in a kparams file");
    assert!(proof_path.is_some(), "pass in a proof file for output");
    assert!(witness_path.is_some(), "pass in a witness file for input");

    println!("params_path: {:?}", params_path);
    println!("proof_path: {:?}", proof_path);
    println!("witness_path: {:?}", witness_path);

    let jproof = std::fs::read_to_string(witness_path.unwrap()).unwrap();

    println!("mock taiko prover, proof len = {:?} bytes", jproof.len());
    println!("mock taiko prover, proof_path = {:?}", proof_path);
    println!("use proof_path!");
    write(proof_path.clone().unwrap(), jproof).unwrap();

    proof_path.unwrap()
}
