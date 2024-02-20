use crate::{
    circuit_input_builder::{CircuitInputStateRef, ExecStep},
    evm::Opcode,
    Error,
};
use eth_types::GethExecStep;

#[derive(Debug, Copy, Clone)]
pub(crate) struct ErrorSimple;

// ErrorSimple is to deal with errors with general common ops as below
// - added error to current `ExecStep`
// - restore call context
// no extra ops e.g. stack read etc...
impl Opcode for ErrorSimple {
    fn gen_associated_ops(
        state: &mut CircuitInputStateRef,
        geth_steps: &[GethExecStep],
    ) -> Result<Vec<ExecStep>, Error> {
        let geth_step = &geth_steps[0];
        let mut exec_step = state.new_step(geth_step)?;
        let next_step = geth_steps.get(1);
        exec_step.error = state.get_step_err(geth_step, next_step).unwrap();

        state.handle_return(&mut exec_step, geth_steps, true)?;
        Ok(vec![exec_step])
    }
}
