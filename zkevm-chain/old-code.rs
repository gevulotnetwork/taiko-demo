fn read_block(block_path: String) -> Result<Block<Transaction>, GevulotError> {
    let jblock = std::fs::read_to_string(block_path).map_err(|_| GevulotError::ErrorIo)?;
    let block: Block<Transaction> =
        serde_json::from_str(&jblock).map_err(|_| GevulotError::CanonicalDeserializeError)?;
    Ok(block)
}

#[tokio::main]
async fn main() {
    println!("Let's go!");
    let result = read_block("block.json".to_string());
    println!(
        "The block we read in looks like this:\n{:?}",
        result.unwrap()
    );


// deserialize
let jproofinputs = std::fs::read_to_string(inputs_path).unwrap();
let fil_proof_info: FilProofInfo = serde_json::from_str(&jproofinputs).unwrap();

// serialize
let prover_response = on_prove(ps, &program_id, &Some(witness_file), proof_file).unwrap();
let output = json!(prover_response.proof_info).to_string();
fs::write(proof_file, output).unwrap();
