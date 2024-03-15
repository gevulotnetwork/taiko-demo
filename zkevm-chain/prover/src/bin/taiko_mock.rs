// use env_logger::Env;
use clap::Parser;
use gevulot_shim::{Task, TaskResult};
use std::fs;
use std::fs::write;
use std::io;
use std::{error::Error, result::Result};

fn main() -> Result<(), Box<dyn Error>> {
    println!("taiko_mock main()");
    gevulot_shim::run(run_task)
}

fn run_task(task: Task) -> Result<TaskResult, Box<dyn Error>> {
    println!("run_task()");

    // to synchronize argument parsing
    let mut args = vec!["dummy".to_string()];
    for a in task.args.clone() {
        args.push(a);
    }

    println!("taiko prover: args: {:?}", args);

    let proof_result = prover_mock(&args)?;

    println!("prover_mock result: {:?}", proof_result);

    task.result(vec![], vec![proof_result])
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
fn prover_mock(args: &Vec<String>) -> Result<String, Box<dyn Error>> {
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

    let entries = fs::read_dir("/gevulot")
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    println!("file entries at directory /gevulot :: {:?}", entries);

    // set our arguments, use defaults as applicable
    let params_path = arg_conf.kparams_path;
    let proof_path = arg_conf.proof_path;
    let witness_path = arg_conf.witness_path;

    if witness_path.is_none() {
        return Err(String::from("no witness file parameter").into());
    }
    if params_path.is_none() {
        return Err(String::from("no parameters file parameter").into());
    }
    if proof_path.is_none() {
        return Err(String::from("no proof file parameter").into());
    }

    println!("params_path: {:?}", params_path);
    println!("proof_path: {:?}", proof_path);
    println!("witness_path: {:?}", witness_path);

    let jproof = std::fs::read_to_string(witness_path.unwrap())?;

    println!("mock taiko prover, proof len = {:?} bytes", jproof.len());
    println!("mock taiko prover, proof_path = {:?}", proof_path);
    println!("use proof_path!");
    write(
        proof_path.clone().expect("pass in a proof file for output"),
        jproof,
    )?;

    Ok(proof_path.unwrap())
}
