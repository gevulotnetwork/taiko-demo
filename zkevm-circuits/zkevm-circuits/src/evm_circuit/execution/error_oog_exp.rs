use crate::{
    evm_circuit::{
        execution::ExecutionGadget,
        param::N_BYTES_GAS,
        step::ExecutionState,
        util::{
            common_gadget::CommonErrorGadget,
            constraint_builder::{ConstrainBuilderCommon, EVMConstraintBuilder},
            math_gadget::{ByteSizeGadget, LtGadget},
            CachedRegion, Cell, Word,
        },
        witness::{Block, Call, ExecStep, Transaction},
    },
    util::Expr,
};
use eth_types::{
    evm_types::{GasCost, OpcodeId},
    Field, ToLittleEndian,
};
use halo2_proofs::{circuit::Value, plonk::Error};

/// Gadget to implement the corresponding out of gas errors for
/// [`OpcodeId::EXP`].
#[derive(Clone, Debug)]
pub(crate) struct ErrorOOGExpGadget<F> {
    opcode: Cell<F>,
    base: Word<F>,
    exponent: Word<F>,
    exponent_byte_size: ByteSizeGadget<F>,
    insufficient_gas_cost: LtGadget<F, N_BYTES_GAS>,
    common_error_gadget: CommonErrorGadget<F>,
}

impl<F: Field> ExecutionGadget<F> for ErrorOOGExpGadget<F> {
    const NAME: &'static str = "ErrorOutOfGasEXP";

    const EXECUTION_STATE: ExecutionState = ExecutionState::ErrorOutOfGasEXP;

    fn configure(cb: &mut EVMConstraintBuilder<F>) -> Self {
        let opcode = cb.query_cell();

        cb.require_equal(
            "ErrorOutOfGasEXP opcode must be EXP",
            opcode.expr(),
            OpcodeId::EXP.expr(),
        );

        let base = cb.query_word_rlc();
        let exponent = cb.query_word_rlc();
        cb.stack_pop(base.expr());
        cb.stack_pop(exponent.expr());

        let exponent_byte_size = ByteSizeGadget::construct(
            cb,
            exponent
                .cells
                .iter()
                .map(Expr::expr)
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        );

        let insufficient_gas_cost = LtGadget::construct(
            cb,
            cb.curr.state.gas_left.expr(),
            // static_gas = 10
            // dynamic_gas = exponent_byte_size * 50
            // gas_cost = dynamic_gas + static_gas
            exponent_byte_size.byte_size() * GasCost::EXP_BYTE_TIMES.0.expr()
                + OpcodeId::EXP.constant_gas_cost().expr(),
        );

        cb.require_equal(
            "Gas left is less than gas cost",
            insufficient_gas_cost.expr(),
            1.expr(),
        );

        let common_error_gadget = CommonErrorGadget::construct(cb, opcode.expr(), 4.expr());
        Self {
            opcode,
            base,
            exponent,
            exponent_byte_size,
            insufficient_gas_cost,
            common_error_gadget,
        }
    }

    fn assign_exec_step(
        &self,
        region: &mut CachedRegion<'_, '_, F>,
        offset: usize,
        block: &Block<F>,
        _tx: &Transaction,
        call: &Call,
        step: &ExecStep,
    ) -> Result<(), Error> {
        let opcode = step.opcode().unwrap();
        let [base, exponent] = [0, 1].map(|index| block.get_rws(step, index).stack_value());

        log::debug!(
            "ErrorOutOfGasEXP: gas_left = {}, gas_cost = {}",
            step.gas_left,
            step.gas_cost,
        );

        self.opcode
            .assign(region, offset, Value::known(F::from(opcode.as_u64())))?;
        self.base.assign(region, offset, Some(base.to_le_bytes()))?;
        self.exponent
            .assign(region, offset, Some(exponent.to_le_bytes()))?;
        self.exponent_byte_size.assign(region, offset, exponent)?;
        self.insufficient_gas_cost.assign_value(
            region,
            offset,
            Value::known(F::from(step.gas_left.0)),
            Value::known(F::from(step.gas_cost.0)),
        )?;
        self.common_error_gadget
            .assign(region, offset, block, call, step, 4)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        evm_circuit::test::{rand_bytes, rand_word},
        test_util::CircuitTestBuilder,
    };
    use eth_types::{
        bytecode,
        evm_types::{GasCost, OpcodeId},
        Bytecode, U256,
    };
    use mock::{
        eth, generate_mock_call_bytecode, test_ctx::helpers::account_0_code_account_1_no_code,
        MockCallBytecodeParams, TestContext, MOCK_ACCOUNTS,
    };

    #[test]
    fn test_oog_exp() {
        [
            U256::zero(),
            U256::one(),
            1023.into(),
            U256::MAX,
            rand_word(),
        ]
        .into_iter()
        .for_each(|exponent| {
            let testing_data = TestingData::new(exponent);

            test_root(&testing_data);
            test_internal(&testing_data);
        })
    }

    struct TestingData {
        bytecode: Bytecode,
        gas_cost: u64,
    }

    impl TestingData {
        pub fn new(exponent: U256) -> Self {
            let bytecode = bytecode! {
                PUSH32(exponent)
                PUSH32(rand_word())
                EXP
            };

            let gas_cost = OpcodeId::PUSH32.constant_gas_cost().0 * 2
                + OpcodeId::EXP.constant_gas_cost().0
                + ((exponent.bits() as u64 + 7) / 8) * GasCost::EXP_BYTE_TIMES.0;

            Self { bytecode, gas_cost }
        }
    }

    fn test_root(testing_data: &TestingData) {
        let ctx = TestContext::<2, 1>::new(
            None,
            account_0_code_account_1_no_code(testing_data.bytecode.clone()),
            |mut txs, accs| {
                // Decrease expected gas cost (by 1) to trigger out of gas error.
                txs[0]
                    .from(accs[1].address)
                    .to(accs[0].address)
                    .gas((GasCost::TX.0 + testing_data.gas_cost - 1).into());
            },
            |block, _tx| block.number(0xcafe_u64),
        )
        .unwrap();

        CircuitTestBuilder::new_from_test_ctx(ctx).run();
    }

    fn test_internal(testing_data: &TestingData) {
        let (addr_a, addr_b) = (MOCK_ACCOUNTS[0], MOCK_ACCOUNTS[1]);

        // code B gets called by code A, so the call is an internal call.
        let code_b = testing_data.bytecode.clone();

        // code A calls code B.
        // Decrease expected gas cost (by 1) to trigger out of gas error.
        let code_a = generate_mock_call_bytecode(MockCallBytecodeParams {
            address: addr_b,
            pushdata: rand_bytes(32),
            call_data_length: 0x00usize,
            call_data_offset: 0x20usize,
            gas: testing_data.gas_cost - 1,
            ..MockCallBytecodeParams::default()
        });

        let ctx = TestContext::<3, 1>::new(
            None,
            |accs| {
                accs[0].address(addr_b).code(code_b);
                accs[1].address(addr_a).code(code_a);
                accs[2].address(MOCK_ACCOUNTS[2]).balance(eth(10));
            },
            |mut txs, accs| {
                txs[0].from(accs[2].address).to(accs[1].address);
            },
            |block, _tx| block,
        )
        .unwrap();

        CircuitTestBuilder::new_from_test_ctx(ctx).run();
    }
}
