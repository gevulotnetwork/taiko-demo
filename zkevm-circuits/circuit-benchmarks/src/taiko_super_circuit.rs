//! TaikoSuperCircuit circuit benchmarks

use eth_types::{Bytes, U256};
use halo2_proofs::{
    halo2curves::bn256::{Bn256, Fq, Fr, G1Affine},
    plonk::{ProvingKey, VerifyingKey},
    poly::{
        commitment::ParamsProver,
        kzg::commitment::{KZGCommitmentScheme, ParamsKZG},
    },
};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use snark_verifier::{
    loader::evm::{deploy_and_call, encode_calldata, EvmLoader},
    pcs::kzg::*,
    system::halo2::{compile, transcript::evm::EvmTranscript, Config},
    verifier::SnarkVerifier,
};
use snark_verifier_sdk::{GWC, SHPLONK};
use std::rc::Rc;
use zkevm_circuits::root_circuit::{taiko_aggregation::AccumulationSchemeType, KzgDk, KzgSvk};

/// Number of limbs to decompose a elliptic curve base field element into.
pub const LIMBS: usize = 4;
/// Number of bits of each decomposed limb.
pub const BITS: usize = 68;

pub type ProverParams = ParamsKZG<Bn256>;
pub type ProverCommitmentScheme = KZGCommitmentScheme<Bn256>;
pub type ProverKey = ProvingKey<G1Affine>;

/// KZG accumulation scheme with GWC19 multiopen.
pub type PlonkVerifierGWC =
    snark_verifier::verifier::plonk::PlonkVerifier<GWC, LimbsEncoding<LIMBS, BITS>>;

pub type PlonkVerifierSHPLONK =
    snark_verifier::verifier::plonk::PlonkVerifier<SHPLONK, LimbsEncoding<LIMBS, BITS>>;

use rand::rngs::StdRng;

#[derive(Serialize, Deserialize, Debug)]
struct BlockProofData {
    instances: Vec<U256>,
    proof: Bytes,
}

/// Fixed rng for testing purposes
pub fn fixed_rng() -> StdRng {
    StdRng::seed_from_u64(9)
}

/// Returns [<len>, ...] of `instance`
pub fn gen_num_instance(instance: &[Vec<Fr>]) -> Vec<usize> {
    instance.iter().map(|v| v.len()).collect()
}

/// for chain to generate verifier contract
pub fn gen_verifier(
    params: &ProverParams,
    vk: &VerifyingKey<G1Affine>,
    config: Config,
    num_instance: Vec<usize>,
    aggregation_type: AccumulationSchemeType,
) -> String {
    let protocol = compile(params, vk, config);
    let svk = KzgSvk::<Bn256>::new(params.get_g()[0]);
    let dk = KzgDk::<Bn256>::new(svk, params.g2(), params.s_g2());

    let loader = EvmLoader::new::<Fq, Fr>();
    let protocol = protocol.loaded(&loader);
    let mut transcript = EvmTranscript::<_, Rc<EvmLoader>, _, _>::new(&loader);

    let instances = transcript.load_instances(num_instance);
    match aggregation_type {
        AccumulationSchemeType::GwcType => {
            let proof =
                PlonkVerifierGWC::read_proof(&dk, &protocol, &instances, &mut transcript).unwrap();
            PlonkVerifierGWC::verify(&dk, &protocol, &instances, &proof).unwrap();
        }
        AccumulationSchemeType::ShplonkType => {
            let proof =
                PlonkVerifierSHPLONK::read_proof(&dk, &protocol, &instances, &mut transcript)
                    .unwrap();
            PlonkVerifierSHPLONK::verify(&dk, &protocol, &instances, &proof).unwrap();
        }
    };

    let sol = loader.solidity_code();
    // fs::write(Path::new("./aggregation_plonk.sol"), &sol).unwrap();
    sol
}

/// for chain to verify
pub fn evm_verify(deployment_code: Vec<u8>, instances: Vec<Vec<Fr>>, proof: Vec<u8>) {
    let calldata = encode_calldata(&instances, &proof);
    println!(
        "deploy code size: {} bytes, instances size: [{}][{}], calldata: {}",
        deployment_code.len(),
        instances.len(),
        instances[0].len(),
        calldata.len(),
    );

    match deploy_and_call(deployment_code, calldata) {
        Ok(_) => println!("Verification success"),
        Err(e) => {
            // println!("Verification failed due to {:?}", e);
            panic!("Verification failed due to {:?}", e);
        }
    }
}

/// for chain to verify
pub fn gevulot_evm_verify(
    deployment_code: Vec<u8>,
    instances: Vec<Vec<Fr>>,
    proof: Vec<u8>,
) -> Result<u64, String> {
    let calldata = encode_calldata(&instances, &proof);
    println!(
        "deploy code size: {} bytes, instances size: [{}][{}], calldata: {}",
        deployment_code.len(),
        instances.len(),
        instances[0].len(),
        calldata.len(),
    );

    deploy_and_call(deployment_code, calldata)
}

#[cfg(test)]
mod tests {
    use ark_std::{end_timer, start_timer};
    use bus_mapping::circuit_input_builder::{CircuitsParams, ProtocolInstance};
    use eth_types::{address, bytecode, geth_types::GethData, Word, U256};
    use ethers_signers::{LocalWallet, Signer};
    use halo2_proofs::{
        halo2curves::{
            bn256::{Bn256, Fr, G1Affine},
            ff::PrimeField,
        },
        plonk::{create_proof, keygen_pk, keygen_vk, verify_proof},
        poly::{
            commitment::{Params, ParamsProver},
            kzg::{
                commitment::{KZGCommitmentScheme, ParamsKZG, ParamsVerifierKZG},
                multiopen::{ProverSHPLONK, VerifierSHPLONK},
                strategy::SingleStrategy,
            },
        },
        transcript::{
            Blake2bRead, Blake2bWrite, Challenge255, TranscriptReadBuffer, TranscriptWriterBuffer,
        },
    };
    use itertools::Itertools;
    use mock::{TestContext, MOCK_CHAIN_ID};
    use rand::SeedableRng;
    use rand_chacha::{ChaCha20Rng, ChaChaRng};
    use snark_verifier::loader::evm;
    use snark_verifier_sdk::{
        evm::{evm_verify, gen_evm_proof_gwc, gen_evm_proof_shplonk},
        halo2::{
            aggregation::AccumulationSchemeSDK, gen_snark_gwc, gen_snark_shplonk, gen_srs,
            read_or_create_srs,
        },
        Snark, GWC,
    };
    use std::{collections::HashMap, env::var, fs, io::Read, path::Path};
    use zkevm_circuits::{
        root_circuit::{
            taiko_aggregation::AccumulationSchemeType, Config, TaikoAggregationCircuit,
        },
        taiko_super_circuit::{test::block_1tx, SuperCircuit},
    };

    use crate::taiko_super_circuit::{gen_verifier, BlockProofData};

    const MIN_APP_DEGREE: u32 = 21;
    const MIN_AGG_DEGREE: u32 = 26;

    fn gen_application_snark(
        params: &ParamsKZG<Bn256>,
        aggregation_type: AccumulationSchemeType,
    ) -> Snark {
        println!("gen app snark");
        let circuits_params = CircuitsParams {
            max_txs: 2,
            max_calldata: 200,
            max_rws: 256,
            max_copy_rows: 256,
            max_exp_steps: 256,
            max_bytecode: 512,
            max_evm_rows: 0,
            max_keccak_rows: 0,
        };
        let protocol_instance = ProtocolInstance::default();
        let (_, super_circuit, _, _) =
            SuperCircuit::<_>::build(block_1tx(), circuits_params, protocol_instance).unwrap();

        // let pk = gen_pk(params, &super_circuit, Some(Path::new("./examples/app.pk")),
        // super_circuit.params());
        let vk = keygen_vk(params, &super_circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(params, vk, &super_circuit).expect("keygen_pk should not fail");
        match aggregation_type {
            AccumulationSchemeType::GwcType => {
                gen_snark_gwc(params, &pk, super_circuit, None::<&str>)
            }
            AccumulationSchemeType::ShplonkType => {
                gen_snark_shplonk(params, &pk, super_circuit, None::<&str>)
            }
        }
    }

    fn create_root_super_circuit_prover_sdk<const T: u64, AS: AccumulationSchemeSDK>() {
        let params_app = gen_srs(MIN_APP_DEGREE);
        let aggregation_type = T.into();
        let snarks = [(); 1].map(|_| gen_application_snark(&params_app, aggregation_type));

        let params = gen_srs(MIN_AGG_DEGREE);
        let mut snark_roots = Vec::new();
        for snark in snarks {
            let agg_circuit = TaikoAggregationCircuit::<AS>::new(&params, [snark]).unwrap();

            let start0 = start_timer!(|| "gen vk & pk");
            // let pk = gen_pk(
            // &params,
            // &agg_circuit.without_witnesses(),
            // Some(Path::new("./examples/agg.pk")),
            // agg_circuit.params(),
            // );
            let vk = keygen_vk(&params, &agg_circuit).expect("keygen_vk should not fail");
            let pk = keygen_pk(&params, vk, &agg_circuit).expect("keygen_pk should not fail");
            end_timer!(start0);

            let _root = match aggregation_type {
                AccumulationSchemeType::GwcType => {
                    gen_snark_gwc(
                        &params,
                        &pk,
                        agg_circuit.clone(),
                        // Some(Path::new("./examples/agg.snark"))
                        None::<&str>,
                    )
                }
                AccumulationSchemeType::ShplonkType => {
                    gen_snark_shplonk(
                        &params,
                        &pk,
                        agg_circuit.clone(),
                        // Some(Path::new("./examples/agg.snark"))
                        None::<&str>,
                    )
                }
            };

            snark_roots.push(_root);
        }

        println!("gen blocks agg snark");
        let params = gen_srs(MIN_AGG_DEGREE);
        let agg_circuit = TaikoAggregationCircuit::<AS>::new(&params, snark_roots).unwrap();
        println!("new root agg circuit {}", agg_circuit);

        let start0 = start_timer!(|| "gen vk & pk");
        // let pk = gen_pk(
        // &params,
        // &agg_circuit.without_witnesses(),
        // Some(Path::new("./examples/agg.pk")),
        // agg_circuit.params(),
        // );
        let vk = keygen_vk(&params, &agg_circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&params, vk.clone(), &agg_circuit).expect("keygen_pk should not fail");
        end_timer!(start0);

        println!("gen evm snark");
        // do one more time to verify
        let num_instances = agg_circuit.num_instance();
        let instances = agg_circuit.instance();
        let accumulator_indices = Some(agg_circuit.accumulator_indices());
        let proof_calldata = match aggregation_type {
            AccumulationSchemeType::GwcType => {
                gen_evm_proof_gwc(&params, &pk, agg_circuit, instances.clone())
            }
            AccumulationSchemeType::ShplonkType => {
                gen_evm_proof_shplonk(&params, &pk, agg_circuit, instances.clone())
            }
        };

        let block_proof_data = BlockProofData {
            instances: instances
                .iter()
                .flatten()
                .map(|v| U256::from_little_endian(v.to_repr().as_ref()))
                .collect(),
            proof: proof_calldata.clone().into(),
        };

        fs::write(
            Path::new("./proof.json"),
            serde_json::to_vec(&block_proof_data).unwrap(),
        )
        .unwrap();

        let deployment_sol = gen_verifier(
            &params,
            &vk,
            Config::kzg()
                .with_num_instance(num_instances.clone())
                .with_accumulator_indices(accumulator_indices),
            num_instances,
            aggregation_type,
        );
        let evm_verifier_bytecode = evm::compile_solidity(&deployment_sol);

        evm_verify(evm_verifier_bytecode, instances, proof_calldata);
    }

    // for N super circuit -> 1 root circuit integration
    fn create_1_level_root_super_circuit_prover_sdk<const T: u64, AS: AccumulationSchemeSDK>() {
        let agg_type = T.into();
        let mut params_app = gen_srs(MIN_AGG_DEGREE);
        params_app.downsize(MIN_APP_DEGREE);
        let snarks = [(); 1].map(|_| gen_application_snark(&params_app, agg_type));

        println!("gen blocks agg snark");
        let params = gen_srs(MIN_AGG_DEGREE);
        let agg_circuit = TaikoAggregationCircuit::<AS>::new(&params, snarks).unwrap();
        let start0 = start_timer!(|| "gen vk & pk");
        // let pk = gen_pk(
        // &params,
        // &agg_circuit.without_witnesses(),
        // Some(Path::new("./examples/agg.pk")),
        // agg_circuit.params(),
        // );
        let vk = keygen_vk(&params, &agg_circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&params, vk.clone(), &agg_circuit).expect("keygen_pk should not fail");
        end_timer!(start0);

        println!("gen evm snark");
        // do one more time to verify
        let num_instances = agg_circuit.num_instance();
        let instances = agg_circuit.instance();
        let accumulator_indices = Some(agg_circuit.accumulator_indices());
        let proof_calldata = match agg_type {
            AccumulationSchemeType::GwcType => {
                gen_evm_proof_gwc(&params, &pk, agg_circuit, instances.clone())
            }
            AccumulationSchemeType::ShplonkType => {
                gen_evm_proof_shplonk(&params, &pk, agg_circuit, instances.clone())
            }
        };

        let block_proof_data = BlockProofData {
            instances: instances
                .iter()
                .flatten()
                .map(|v| U256::from_little_endian(v.to_repr().as_ref()))
                .collect(),
            proof: proof_calldata.clone().into(),
        };

        fs::write(
            Path::new("./proof.json"),
            serde_json::to_vec(&block_proof_data).unwrap(),
        )
        .unwrap();

        let deployment_code = gen_verifier(
            &params,
            &vk,
            Config::kzg()
                .with_num_instance(num_instances.clone())
                .with_accumulator_indices(accumulator_indices),
            num_instances,
            agg_type,
        );
        let evm_verifier_bytecode = evm::compile_solidity(&deployment_code);

        evm_verify(evm_verifier_bytecode, instances, proof_calldata);
    }

    #[test]
    fn test_local_verify() {
        let file_path = "./aggregation_plonk.yul";
        let mut file = fs::File::open(file_path).expect("yul file open ok");
        let mut deployment_code = String::new();
        file.read_to_string(&mut deployment_code)
            .expect("yul file read ok");
        let evm_verifier_bytecode = evm::compile_solidity(&deployment_code);

        let proof_path = "./proof.json";
        let proof_file = fs::File::open(proof_path).expect("proof file open ok");
        let block_proof_data: BlockProofData =
            serde_json::from_reader(proof_file).expect("read proof json ok");

        let instances = block_proof_data
            .instances
            .iter()
            .map(|v| Fr::from_u128(v.as_u128()))
            .collect_vec();
        println!("instances = {:?}", instances);
        println!("proof = {:?}", block_proof_data.proof);

        evm_verify(
            evm_verifier_bytecode,
            vec![instances],
            block_proof_data.proof.to_vec(),
        );
    }

    #[cfg_attr(not(feature = "benches"), ignore)]
    #[test]
    fn test_setup_random_params() {
        let k = 22;
        read_or_create_srs::<G1Affine, _>(k, move |k| {
            ParamsKZG::<Bn256>::setup(k, ChaCha20Rng::from_entropy())
        });
    }

    #[cfg_attr(not(feature = "benches"), ignore)]
    #[test]
    fn bench_taiko_super_circuit_root_prover_sdk() {
        // New version, cleanest using the new sdk
        const AGG_TYPE: u64 = AccumulationSchemeType::GwcType as u64;
        create_root_super_circuit_prover_sdk::<AGG_TYPE, GWC>();
    }

    #[cfg_attr(not(feature = "benches"), ignore)]
    #[test]
    fn bench_taiko_super_circuit_n_to_1_root_prover() {
        // for N->1 aggregation using new sdk
        const AGG_TYPE: u64 = AccumulationSchemeType::GwcType as u64;
        create_1_level_root_super_circuit_prover_sdk::<AGG_TYPE, GWC>();
    }

    #[cfg_attr(not(feature = "benches"), ignore)]
    #[test]
    fn bench_taiko_super_circuit_prover() {
        let setup_prfx = crate::constants::SETUP_PREFIX;
        let proof_gen_prfx = crate::constants::PROOFGEN_PREFIX;
        let proof_ver_prfx = crate::constants::PROOFVER_PREFIX;
        // Unique string used by bench results module for parsing the result
        const BENCHMARK_ID: &str = "Taiko Super Circuit";

        let degree: u32 = var("DEGREE")
            .expect("No DEGREE env var was provided")
            .parse()
            .expect("Cannot parse DEGREE env var as u32");

        let mut rng = ChaChaRng::seed_from_u64(2);

        let chain_id = (*MOCK_CHAIN_ID).as_u64();

        let bytecode = bytecode! {
            STOP
        };

        let wallet_a = LocalWallet::new(&mut rng).with_chain_id(chain_id);

        let addr_a = wallet_a.address();
        let addr_b = address!("0x000000000000000000000000000000000000BBBB");

        let mut wallets = HashMap::new();
        wallets.insert(wallet_a.address(), wallet_a);

        let mut block: GethData = TestContext::<2, 1>::new(
            None,
            |accs| {
                accs[0]
                    .address(addr_b)
                    .balance(Word::from(1u64 << 20))
                    .code(bytecode);
                accs[1].address(addr_a).balance(Word::from(1u64 << 20));
            },
            |mut txs, accs| {
                txs[0]
                    .from(accs[1].address)
                    .to(accs[0].address)
                    .gas(Word::from(1_000_000u64));
            },
            |block, _tx| block.number(0xcafeu64),
        )
        .unwrap()
        .into();

        block.sign(&wallets);

        let circuits_params = CircuitsParams {
            max_txs: 1,
            max_calldata: 32,
            max_rws: 256,
            max_copy_rows: 256,
            max_exp_steps: 256,
            max_bytecode: 512,
            max_evm_rows: 0,
            max_keccak_rows: 0,
        };
        let (_, circuit, instance, _) =
            SuperCircuit::build(block, circuits_params, ProtocolInstance::default()).unwrap();
        let instance_refs: Vec<&[Fr]> = instance.iter().map(|v| &v[..]).collect();

        // Bench setup generation
        let setup_message = format!("{} {} with degree = {}", BENCHMARK_ID, setup_prfx, degree);
        let start1 = start_timer!(|| setup_message);
        let general_params = ParamsKZG::<Bn256>::setup(degree, &mut rng);
        let verifier_params: ParamsVerifierKZG<Bn256> = general_params.verifier_params().clone();
        end_timer!(start1);

        // Initialize the proving key
        let vk = keygen_vk(&general_params, &circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&general_params, vk, &circuit).expect("keygen_pk should not fail");
        // Create a proof
        let mut transcript = Blake2bWrite::<_, G1Affine, Challenge255<_>>::init(vec![]);

        // Bench proof generation time
        let proof_message = format!(
            "{} {} with degree = {}",
            BENCHMARK_ID, proof_gen_prfx, degree
        );
        let start2 = start_timer!(|| proof_message);
        create_proof::<
            KZGCommitmentScheme<Bn256>,
            ProverSHPLONK<'_, Bn256>,
            Challenge255<G1Affine>,
            ChaChaRng,
            Blake2bWrite<Vec<u8>, G1Affine, Challenge255<G1Affine>>,
            SuperCircuit<Fr>,
        >(
            &general_params,
            &pk,
            &[circuit],
            &[&instance_refs],
            rng,
            &mut transcript,
        )
        .expect("proof generation should not fail");
        let proof = transcript.finalize();
        end_timer!(start2);

        // Bench verification time
        let start3 = start_timer!(|| format!("{} {}", BENCHMARK_ID, proof_ver_prfx));
        let mut verifier_transcript = Blake2bRead::<_, G1Affine, Challenge255<_>>::init(&proof[..]);
        let strategy = SingleStrategy::new(&general_params);

        verify_proof::<
            KZGCommitmentScheme<Bn256>,
            VerifierSHPLONK<'_, Bn256>,
            Challenge255<G1Affine>,
            Blake2bRead<&[u8], G1Affine, Challenge255<G1Affine>>,
            SingleStrategy<'_, Bn256>,
        >(
            &verifier_params,
            pk.get_vk(),
            strategy,
            &[&instance_refs],
            &mut verifier_transcript,
        )
        .expect("failed to verify bench circuit");
        end_timer!(start3);
    }
}
