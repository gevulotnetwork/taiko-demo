use crate::circuit_witness::CircuitWitness;
use crate::Fr;
use ethers_core::abi::AbiEncode;
use rand::Rng;
// use zkevm_circuits::bytecode_circuit::circuit::BytecodeCircuit;
// use zkevm_circuits::copy_circuit::CopyCircuit;
// use zkevm_circuits::evm_circuit::EvmCircuit;
// use zkevm_circuits::exp_circuit::ExpCircuit;
// use zkevm_circuits::keccak_circuit::KeccakCircuit;
// use zkevm_circuits::pi_circuit::PiCircuit;
// use zkevm_circuits::state_circuit::StateCircuit;
// use zkevm_circuits::tx_circuit::TxCircuit;
use zkevm_circuits::taiko_super_circuit::SuperCircuit;
use zkevm_circuits::util::SubCircuit;

/// Returns a instance of the `SuperCircuit`.
pub fn gen_super_circuit<
    const MAX_TXS: usize,
    const MAX_CALLDATA: usize,
    const MAX_RWS: usize,
    const MAX_COPY_ROWS: usize,
    RNG: Rng,
>(
    witness: &CircuitWitness,
    mut _rng: RNG,
) -> Result<SuperCircuit<Fr>, String> {
    let block = witness.dummy_evm_witness();
    assert_eq!(
        block.context.block_hash.encode_hex(),
        block
            .protocol_instance
            .as_ref()
            .unwrap()
            .transition.blockHash.to_string()
    );
    assert_eq!(
        block.context.history_hashes[block.context.history_hashes.len() - 1].encode_hex(),
        block
            .protocol_instance
            .as_ref()
            .unwrap()
            .transition.parentHash.to_string()
    );
    let circuit = SuperCircuit::new_from_block(&block);
    Ok(circuit)
}

// /// Returns a instance of the `PiCircuit`.
// pub fn gen_pi_circuit<
//     const MAX_TXS: usize,
//     const MAX_CALLDATA: usize,
//     const MAX_RWS: usize,
//     const MAX_COPY_ROWS: usize,
//     RNG: Rng,
// >(
//     witness: &CircuitWitness,
//     mut _rng: RNG,
// ) -> Result<PiCircuit<Fr>, String> {
//     let block = witness.evm_witness();
//     let circuit = PiCircuit::new_from_block(&block);

//     Ok(circuit)
// }

// /// Returns a instance of the `EvmCircuit`.
// pub fn gen_evm_circuit<
//     const MAX_TXS: usize,
//     const MAX_CALLDATA: usize,
//     const MAX_RWS: usize,
//     const MAX_COPY_ROWS: usize,
//     RNG: Rng,
// >(
//     witness: &CircuitWitness,
//     mut _rng: RNG,
// ) -> Result<EvmCircuit<Fr>, String> {
//     let block = witness.evm_witness();
//     Ok(EvmCircuit::new_from_block(&block))
// }

// /// Returns a instance of the `StateCircuit`.
// pub fn gen_state_circuit<
//     const MAX_TXS: usize,
//     const MAX_CALLDATA: usize,
//     const MAX_RWS: usize,
//     const MAX_COPY_ROWS: usize,
//     RNG: Rng,
// >(
//     witness: &CircuitWitness,
//     mut _rng: RNG,
// ) -> Result<StateCircuit<Fr>, String> {
//     let block = witness.evm_witness();
//     Ok(StateCircuit::new_from_block(&block))
// }

// /// Returns a instance of the `TxCircuit`.
// pub fn gen_tx_circuit<
//     const MAX_TXS: usize,
//     const MAX_CALLDATA: usize,
//     const MAX_RWS: usize,
//     const MAX_COPY_ROWS: usize,
//     RNG: Rng,
// >(
//     witness: &CircuitWitness,
//     mut _rng: RNG,
// ) -> Result<TxCircuit<Fr>, String> {
//     let block = witness.evm_witness();
//     Ok(TxCircuit::new_from_block(&block))
// }

// /// Returns a instance of the `BytecodeCircuit`.
// pub fn gen_bytecode_circuit<
//     const MAX_TXS: usize,
//     const MAX_CALLDATA: usize,
//     const MAX_RWS: usize,
//     const MAX_COPY_ROWS: usize,
//     RNG: Rng,
// >(
//     witness: &CircuitWitness,
//     mut _rng: RNG,
// ) -> Result<BytecodeCircuit<Fr>, String> {
//     let block = witness.evm_witness();
//     Ok(BytecodeCircuit::new_from_block(&block))
// }

// /// Returns a instance of the `CopyCircuit`.
// pub fn gen_copy_circuit<
//     const MAX_TXS: usize,
//     const MAX_CALLDATA: usize,
//     const MAX_RWS: usize,
//     const MAX_COPY_ROWS: usize,
//     RNG: Rng,
// >(
//     witness: &CircuitWitness,
//     mut _rng: RNG,
// ) -> Result<CopyCircuit<Fr>, String> {
//     let block = witness.evm_witness();
//     Ok(CopyCircuit::new_from_block(&block))
// }

// /// Returns a instance of the `ExpCircuit`.
// pub fn gen_exp_circuit<
//     const MAX_TXS: usize,
//     const MAX_CALLDATA: usize,
//     const MAX_RWS: usize,
//     const MAX_COPY_ROWS: usize,
//     RNG: Rng,
// >(
//     witness: &CircuitWitness,
//     mut _rng: RNG,
// ) -> Result<ExpCircuit<Fr>, String> {
//     let block = witness.evm_witness();
//     Ok(ExpCircuit::new_from_block(&block))
// }

// /// Returns a instance of the `KeccakCircuit`.
// pub fn gen_keccak_circuit<
//     const MAX_TXS: usize,
//     const MAX_CALLDATA: usize,
//     const MAX_RWS: usize,
//     const MAX_COPY_ROWS: usize,
//     RNG: Rng,
// >(
//     witness: &CircuitWitness,
//     mut _rng: RNG,
// ) -> Result<KeccakCircuit<Fr>, String> {
//     let block = witness.evm_witness();
//     Ok(KeccakCircuit::new_from_block(&block))
// }
