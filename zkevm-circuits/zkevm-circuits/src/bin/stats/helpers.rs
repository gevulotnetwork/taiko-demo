use std::cmp::Ordering;

use bus_mapping::{
    circuit_input_builder::{self, CircuitsParams, ExecState},
    mock::BlockData,
};
use cli_table::{
    format::{Justify, Separator},
    print_stdout, Table, WithTitle,
};
use eth_types::{bytecode, evm_types::OpcodeId, geth_types::GethData, Address, Bytecode, ToWord};
use mock::{eth, test_ctx::TestContext, MOCK_ACCOUNTS};
use strum::IntoEnumIterator;
use zkevm_circuits::evm_circuit::step::ExecutionState;

/// Generate the prefix bytecode to trigger a big amount of rw operations
pub(crate) fn bytecode_prefix_op_big_rws(opcode: OpcodeId) -> Bytecode {
    match opcode {
        OpcodeId::CODECOPY | OpcodeId::CALLDATACOPY => {
            bytecode! {
                PUSH4(0x1000) // size
                PUSH2(0x00) // offset
                PUSH2(0x00) // destOffset
            }
        }
        OpcodeId::RETURNDATACOPY => {
            bytecode! {
                PUSH1(0x00) // retLength
                PUSH1(0x00) // retOffset
                PUSH1(0x00) // argsLength
                PUSH1(0x00) // argsOffset
                PUSH1(0x00) // value
                PUSH32(MOCK_ACCOUNTS[3].to_word())
                PUSH32(0x1_0000) // gas
                CALL
                PUSH4(0x1000) // size
                PUSH2(0x00) // offset
                PUSH2(0x00) // destOffset
            }
        }
        OpcodeId::LOG0
        | OpcodeId::LOG1
        | OpcodeId::LOG2
        | OpcodeId::LOG3
        | OpcodeId::LOG4
        | OpcodeId::SHA3
        | OpcodeId::RETURN
        | OpcodeId::REVERT => bytecode! {
            PUSH4(0x1000) // size
            PUSH2(0x00) // offset
        },
        OpcodeId::EXTCODECOPY => bytecode! {
            PUSH4(0x1000) // size
            PUSH2(0x00) // offset
            PUSH2(0x00) // destOffset
            PUSH2(0x00) // address
        },
        _ => bytecode! {
            PUSH2(0x40)
            PUSH2(0x50)
        },
    }
}

/// Wrap f64 for both sorting and pretty formatting
#[derive(PartialEq, PartialOrd)]
struct PrettyF64(f64);

impl std::fmt::Display for PrettyF64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:1.3}", self.0)
    }
}

impl From<f64> for PrettyF64 {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

#[derive(Table)]
struct Row {
    #[table(title = "Execution State")]
    state: ExecutionState,
    #[table(title = "Opcode")]
    opcode: OpcodeId,
    #[table(title = "Height", justify = "Justify::Right")]
    height: usize,
    #[table(title = "Gas Cost", justify = "Justify::Right")]
    gas_cost: u64,
    #[table(title = "Height per Gas", justify = "Justify::Right")]
    height_per_gas: PrettyF64,
}

/// This function prints to stdout a table with all the implemented states
/// and their responsible opcodes with the following stats:
/// - height: number of rows in a circuit used by the execution state
/// - gas: gas value used for the opcode execution
/// - height/gas: ratio between circuit cost and gas cost
///
/// The TestContext is as follows:
/// - `MOCK_ACCOUNTS[0]` calls `MOCK_ACCOUNTS[1]` which has a proxy code that calls
///   `MOCK_ACCOUNT[2]` which has the main code
/// - `0x0` account has a copy of the main code
/// - `MOCK_ACCOUNTS[3]` has a small code that returns a 0-memory chunk
pub(crate) fn print_circuit_stats_by_states(
    // Function to select which opcodes to analyze.  When this returns false,
    // the opcode is skipped.
    fn_filter: impl Fn(ExecutionState) -> bool,
    // Function to generate bytecode that will be prefixed to the opcode,
    // useful to set up arguments that cause worst height/gas case.
    fn_bytecode_prefix_op: impl Fn(OpcodeId) -> Bytecode,
    // Function that calculates the circuit height used by an opcode.  This function takes the
    // circuit input builder Block, the current execution state, and the step index in circuit
    // input builder tx.
    fn_height: impl Fn(&circuit_input_builder::Block, ExecutionState, usize) -> usize,
) {
    let mut implemented_states = Vec::new();
    for state in ExecutionState::iter() {
        let height = state.get_step_height_option(false);
        if height.is_some() {
            implemented_states.push(state);
        }
    }
    let smallcode = bytecode! {
        PUSH4(0x1000) // size
        PUSH2(0x00) // offset
        RETURN
    };
    let proxy_code = bytecode! {
        PUSH2(0x1000) // retLength
        PUSH1(0x00) // retOffset
        PUSH1(0x00) // argsLength
        PUSH1(0x00) // argsOffset
        PUSH1(0x00) // value
        PUSH32(MOCK_ACCOUNTS[2].to_word())
        PUSH32(800_000) // gas
        CALL
        STOP
    };

    let mut rows = vec![];
    for state in implemented_states {
        if !fn_filter(state) {
            continue;
        }
        for responsible_op in state.responsible_opcodes() {
            let opcode = responsible_op.opcode();
            let mut code = bytecode! {
                PUSH2(0x00)
                EXTCODESIZE // Warm up 0x0 address
                PUSH1(0x00)
                PUSH1(0x00)
                PUSH1(0x00)
                PUSH1(0x00)
                PUSH1(0x00)
                PUSH1(0x00)
                PUSH1(0x00)
                PUSH1(0x00)
                PUSH1(0x00)
                PUSH1(0x00)
                PUSH2(0x00)
                PUSH2(0x10)
                PUSH2(0x20)
                PUSH2(0x30)
            };
            let bytecode_prefix_op = fn_bytecode_prefix_op(opcode);
            code.append(&bytecode_prefix_op);
            code.write_op(opcode);
            let opcode_pc = code.code.len() - 1;
            // let opcode_step_index = (proxy_code.num_opcodes - 1 + code.num_opcodes) - 1;
            code.op_stop();
            let block: GethData = TestContext::<10, 1>::new(
                None,
                |accs| {
                    accs[0].address(MOCK_ACCOUNTS[0]).balance(eth(10));
                    accs[1]
                        .address(MOCK_ACCOUNTS[1])
                        .balance(eth(10))
                        .code(proxy_code.clone());
                    accs[2]
                        .address(MOCK_ACCOUNTS[2])
                        .balance(eth(10))
                        .code(code.clone());
                    accs[3].address(MOCK_ACCOUNTS[3]).code(smallcode.clone());
                    accs[4].address(Address::zero()).balance(eth(10)).code(code);
                },
                |mut txs, accs| {
                    txs[0]
                        .from(accs[0].address)
                        .to(accs[1].address)
                        .input(vec![1, 2, 3, 4, 5, 6, 7].into());
                },
                |block, _tx| block.number(0xcafeu64),
            )
            .unwrap()
            .into();
            let mut builder = BlockData::new_from_geth_data_with_params(
                block.clone(),
                CircuitsParams {
                    max_rws: 16_000,
                    max_copy_rows: 8_000,
                    ..CircuitsParams::default()
                },
            )
            .new_circuit_input_builder();
            builder
                .handle_block(&block.eth_block, &block.geth_traces)
                .unwrap();
            // Find the step that executed our opcode by filtering on second call (because
            // we run it via proxy) and the PC where we wrote the opcode.
            let (step_index, step) = builder.block.txs[0]
                .steps()
                .iter()
                .enumerate()
                .find(|(_, s)| s.call_index == 1 && s.pc.0 == opcode_pc)
                .unwrap();
            assert_eq!(ExecState::Op(opcode), step.exec_state);
            let height = fn_height(&builder.block, state, step_index);

            // Substract 1 to step_index to remove the `BeginTx` step, which doesn't appear
            // in the geth trace.
            let geth_step = &block.geth_traces[0].struct_logs[step_index - 1];
            assert_eq!(opcode, geth_step.op);
            let gas_cost = geth_step.gas_cost.0;
            rows.push(Row {
                state,
                opcode,
                height,
                gas_cost,
                height_per_gas: (height as f64 / gas_cost as f64).into(),
            });
        }
    }
    rows.sort_by(|a, b| {
        b.height_per_gas
            .partial_cmp(&a.height_per_gas)
            .unwrap_or(Ordering::Greater)
    });

    print_stdout(rows.with_title().separator(Separator::builder().build()))
        .expect("the table renders");
}
