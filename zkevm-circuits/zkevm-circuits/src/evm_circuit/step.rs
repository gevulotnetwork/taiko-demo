use super::{
    param::EXECUTION_STATE_HEIGHT_MAP_WITH_TAIKO,
    util::{CachedRegion, CellManager, CellType},
};
use crate::{
    evm_circuit::{
        param::{EXECUTION_STATE_HEIGHT_MAP, MAX_STEP_HEIGHT, STEP_STATE_HEIGHT, STEP_WIDTH},
        util::Cell,
        witness::{Block, Call, ExecStep},
    },
    util::Expr,
};
use bus_mapping::{
    circuit_input_builder::ExecState,
    error::{ExecError, OogError},
    evm::OpcodeId,
    precompile::PrecompileCalls,
};
use eth_types::{evm_unimplemented, Field, ToWord};
use halo2_proofs::{
    circuit::Value,
    plonk::{Advice, Column, ConstraintSystem, Error, Expression},
};
use std::{fmt::Display, iter, marker::ConstParamTy};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

impl From<PrecompileCalls> for ExecutionState {
    fn from(value: PrecompileCalls) -> Self {
        match value {
            PrecompileCalls::ECRecover => ExecutionState::PrecompileEcRecover,
            PrecompileCalls::Sha256 => ExecutionState::PrecompileSha256,
            PrecompileCalls::Ripemd160 => ExecutionState::PrecompileRipemd160,
            PrecompileCalls::Identity => ExecutionState::PrecompileIdentity,
            PrecompileCalls::Modexp => ExecutionState::PrecompileBigModExp,
            PrecompileCalls::Bn128Add => ExecutionState::PrecompileBn256Add,
            PrecompileCalls::Bn128Mul => ExecutionState::PrecompileBn256ScalarMul,
            PrecompileCalls::Bn128Pairing => ExecutionState::PrecompileBn256Pairing,
            PrecompileCalls::Blake2F => ExecutionState::PrecompileBlake2f,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, EnumIter, ConstParamTy)]
pub enum ExecutionState {
    // Internal state
    BeginTx,
    EndTx,
    EndBlock,
    // Opcode successful cases
    STOP,
    ADD_SUB,     // ADD, SUB
    MUL_DIV_MOD, // MUL, DIV, MOD
    SDIV_SMOD,   // SDIV, SMOD
    SHL_SHR,     // SHL, SHR
    ADDMOD,
    MULMOD,
    EXP,
    SIGNEXTEND,
    CMP,  // LT, GT, EQ
    SCMP, // SLT, SGT
    ISZERO,
    BITWISE, // AND, OR, XOR
    NOT,
    BYTE,
    SAR,
    SHA3,
    ADDRESS,
    BALANCE,
    ORIGIN,
    CALLER,
    CALLVALUE,
    CALLDATALOAD,
    CALLDATASIZE,
    CALLDATACOPY,
    CODESIZE,
    CODECOPY,
    GASPRICE,
    EXTCODESIZE,
    EXTCODECOPY,
    RETURNDATASIZE,
    RETURNDATACOPY,
    EXTCODEHASH,
    BLOCKHASH,
    BLOCKCTXU64,  // TIMESTAMP, NUMBER, GASLIMIT
    BLOCKCTXU160, // COINBASE
    BLOCKCTXU256, // DIFFICULTY, BASEFEE
    CHAINID,
    SELFBALANCE,
    POP,
    MEMORY, // MLOAD, MSTORE, MSTORE8
    SLOAD,
    SSTORE,
    JUMP,
    JUMPI,
    PC,
    MSIZE,
    GAS,
    JUMPDEST,
    PUSH0,
    PUSH, // PUSH1, PUSH2, ..., PUSH32
    DUP,  // DUP1, DUP2, ..., DUP16
    SWAP, // SWAP1, SWAP2, ..., SWAP16
    LOG,  // LOG0, LOG1, ..., LOG4
    CREATE,
    CALL_OP,       // CALL, CALLCODE, DELEGATECALL, STATICCALL
    RETURN_REVERT, // RETURN, REVERT
    CREATE2,
    SELFDESTRUCT,
    // Error cases
    ErrorInvalidOpcode,
    ErrorStack,
    ErrorWriteProtection,
    ErrorDepth,
    ErrorInsufficientBalance,
    ErrorContractAddressCollision,
    ErrorInvalidCreationCode,
    ErrorMaxCodeSizeExceeded,
    ErrorInvalidJump,
    ErrorReturnDataOutOfBound,
    ErrorOutOfGasConstant,
    ErrorOutOfGasStaticMemoryExpansion,
    ErrorOutOfGasDynamicMemoryExpansion,
    ErrorOutOfGasMemoryCopy,
    ErrorOutOfGasAccountAccess,
    ErrorOutOfGasCodeStore,
    ErrorOutOfGasLOG,
    ErrorOutOfGasEXP,
    ErrorOutOfGasSHA3,
    ErrorOutOfGasEXTCODECOPY,
    ErrorOutOfGasCall,
    ErrorOutOfGasSloadSstore,
    ErrorOutOfGasCREATE2,
    ErrorOutOfGasSELFDESTRUCT,
    // Precompiles
    PrecompileEcRecover,
    PrecompileSha256,
    PrecompileRipemd160,
    PrecompileIdentity,
    PrecompileBigModExp,
    PrecompileBn256Add,
    PrecompileBn256ScalarMul,
    PrecompileBn256Pairing,
    PrecompileBlake2f,
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self::STOP
    }
}

impl Display for ExecutionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl From<&ExecError> for ExecutionState {
    fn from(error: &ExecError) -> Self {
        match error {
            ExecError::InvalidOpcode => ExecutionState::ErrorInvalidOpcode,
            ExecError::StackOverflow | ExecError::StackUnderflow => ExecutionState::ErrorStack,
            ExecError::WriteProtection => ExecutionState::ErrorWriteProtection,
            ExecError::Depth => ExecutionState::ErrorDepth,
            ExecError::InsufficientBalance => ExecutionState::ErrorInsufficientBalance,
            ExecError::ContractAddressCollision => ExecutionState::ErrorContractAddressCollision,
            ExecError::InvalidCreationCode => ExecutionState::ErrorInvalidCreationCode,
            ExecError::InvalidJump => ExecutionState::ErrorInvalidJump,
            ExecError::ReturnDataOutOfBounds => ExecutionState::ErrorReturnDataOutOfBound,
            ExecError::CodeStoreOutOfGas => ExecutionState::ErrorOutOfGasCodeStore,
            ExecError::MaxCodeSizeExceeded => ExecutionState::ErrorMaxCodeSizeExceeded,
            ExecError::OutOfGas(oog_error) => match oog_error {
                OogError::Constant => ExecutionState::ErrorOutOfGasConstant,
                OogError::StaticMemoryExpansion => {
                    ExecutionState::ErrorOutOfGasStaticMemoryExpansion
                }
                OogError::DynamicMemoryExpansion => {
                    ExecutionState::ErrorOutOfGasDynamicMemoryExpansion
                }
                OogError::MemoryCopy => ExecutionState::ErrorOutOfGasMemoryCopy,
                OogError::AccountAccess => ExecutionState::ErrorOutOfGasAccountAccess,
                OogError::CodeStore => ExecutionState::ErrorOutOfGasCodeStore,
                OogError::Log => ExecutionState::ErrorOutOfGasLOG,
                OogError::Exp => ExecutionState::ErrorOutOfGasEXP,
                OogError::Sha3 => ExecutionState::ErrorOutOfGasSHA3,
                OogError::Call => ExecutionState::ErrorOutOfGasCall,
                OogError::SloadSstore => ExecutionState::ErrorOutOfGasSloadSstore,
                OogError::Create2 => ExecutionState::ErrorOutOfGasCREATE2,
                OogError::SelfDestruct => ExecutionState::ErrorOutOfGasSELFDESTRUCT,
            },
        }
    }
}
impl From<&ExecStep> for ExecutionState {
    fn from(step: &ExecStep) -> Self {
        if let Some(error) = step.error.as_ref() {
            return error.into();
        }
        match step.exec_state {
            ExecState::Op(op) => {
                if op.is_dup() {
                    return ExecutionState::DUP;
                }
                if op.is_push0() {
                    return ExecutionState::PUSH0;
                }
                if op.is_push() {
                    return ExecutionState::PUSH;
                }
                if op.is_swap() {
                    return ExecutionState::SWAP;
                }
                if op.is_log() {
                    return ExecutionState::LOG;
                }

                macro_rules! dummy {
                    ($name:expr) => {{
                        evm_unimplemented!("{:?} is implemented with DummyGadget", $name);
                        $name
                    }};
                }

                match op {
                    OpcodeId::ADD | OpcodeId::SUB => ExecutionState::ADD_SUB,
                    OpcodeId::ADDMOD => ExecutionState::ADDMOD,
                    OpcodeId::ADDRESS => ExecutionState::ADDRESS,
                    OpcodeId::BALANCE => ExecutionState::BALANCE,
                    OpcodeId::MUL | OpcodeId::DIV | OpcodeId::MOD => ExecutionState::MUL_DIV_MOD,
                    OpcodeId::MULMOD => ExecutionState::MULMOD,
                    OpcodeId::SDIV | OpcodeId::SMOD => ExecutionState::SDIV_SMOD,
                    OpcodeId::EQ | OpcodeId::LT | OpcodeId::GT => ExecutionState::CMP,
                    OpcodeId::SLT | OpcodeId::SGT => ExecutionState::SCMP,
                    OpcodeId::SIGNEXTEND => ExecutionState::SIGNEXTEND,
                    OpcodeId::STOP => ExecutionState::STOP,
                    OpcodeId::AND => ExecutionState::BITWISE,
                    OpcodeId::XOR => ExecutionState::BITWISE,
                    OpcodeId::OR => ExecutionState::BITWISE,
                    OpcodeId::NOT => ExecutionState::NOT,
                    OpcodeId::EXP => ExecutionState::EXP,
                    OpcodeId::POP => ExecutionState::POP,
                    OpcodeId::PUSH32 => ExecutionState::PUSH,
                    OpcodeId::BYTE => ExecutionState::BYTE,
                    OpcodeId::MLOAD => ExecutionState::MEMORY,
                    OpcodeId::MSTORE => ExecutionState::MEMORY,
                    OpcodeId::MSTORE8 => ExecutionState::MEMORY,
                    OpcodeId::JUMPDEST => ExecutionState::JUMPDEST,
                    OpcodeId::JUMP => ExecutionState::JUMP,
                    OpcodeId::JUMPI => ExecutionState::JUMPI,
                    OpcodeId::GASPRICE => ExecutionState::GASPRICE,
                    OpcodeId::PC => ExecutionState::PC,
                    OpcodeId::MSIZE => ExecutionState::MSIZE,
                    OpcodeId::CALLER => ExecutionState::CALLER,
                    OpcodeId::CALLVALUE => ExecutionState::CALLVALUE,
                    OpcodeId::EXTCODEHASH => ExecutionState::EXTCODEHASH,
                    OpcodeId::EXTCODESIZE => ExecutionState::EXTCODESIZE,
                    OpcodeId::BLOCKHASH => ExecutionState::BLOCKHASH,
                    OpcodeId::TIMESTAMP | OpcodeId::NUMBER | OpcodeId::GASLIMIT => {
                        ExecutionState::BLOCKCTXU64
                    }
                    OpcodeId::COINBASE => ExecutionState::BLOCKCTXU160,
                    OpcodeId::DIFFICULTY | OpcodeId::BASEFEE => ExecutionState::BLOCKCTXU256,
                    OpcodeId::GAS => ExecutionState::GAS,
                    OpcodeId::SAR => ExecutionState::SAR,
                    OpcodeId::SELFBALANCE => ExecutionState::SELFBALANCE,
                    OpcodeId::SHA3 => ExecutionState::SHA3,
                    OpcodeId::SHL | OpcodeId::SHR => ExecutionState::SHL_SHR,
                    OpcodeId::SLOAD => ExecutionState::SLOAD,
                    OpcodeId::SSTORE => ExecutionState::SSTORE,
                    OpcodeId::CALLDATASIZE => ExecutionState::CALLDATASIZE,
                    OpcodeId::CALLDATACOPY => ExecutionState::CALLDATACOPY,
                    OpcodeId::CHAINID => ExecutionState::CHAINID,
                    OpcodeId::ISZERO => ExecutionState::ISZERO,
                    OpcodeId::CALL
                    | OpcodeId::CALLCODE
                    | OpcodeId::DELEGATECALL
                    | OpcodeId::STATICCALL => ExecutionState::CALL_OP,
                    OpcodeId::ORIGIN => ExecutionState::ORIGIN,
                    OpcodeId::CODECOPY => ExecutionState::CODECOPY,
                    OpcodeId::CALLDATALOAD => ExecutionState::CALLDATALOAD,
                    OpcodeId::CODESIZE => ExecutionState::CODESIZE,
                    OpcodeId::EXTCODECOPY => ExecutionState::EXTCODECOPY,
                    OpcodeId::RETURN | OpcodeId::REVERT => ExecutionState::RETURN_REVERT,
                    OpcodeId::RETURNDATASIZE => ExecutionState::RETURNDATASIZE,
                    OpcodeId::RETURNDATACOPY => ExecutionState::RETURNDATACOPY,
                    // dummy ops
                    OpcodeId::CREATE => dummy!(ExecutionState::CREATE),
                    OpcodeId::CREATE2 => dummy!(ExecutionState::CREATE2),
                    OpcodeId::SELFDESTRUCT => dummy!(ExecutionState::SELFDESTRUCT),
                    _ => unimplemented!("unimplemented opcode {:?}", op),
                }
            }
            ExecState::Precompile(precompile) => match precompile {
                PrecompileCalls::ECRecover => ExecutionState::PrecompileEcRecover,
                PrecompileCalls::Sha256 => ExecutionState::PrecompileSha256,
                PrecompileCalls::Ripemd160 => ExecutionState::PrecompileRipemd160,
                PrecompileCalls::Identity => ExecutionState::PrecompileIdentity,
                PrecompileCalls::Modexp => ExecutionState::PrecompileBigModExp,
                PrecompileCalls::Bn128Add => ExecutionState::PrecompileBn256Add,
                PrecompileCalls::Bn128Mul => ExecutionState::PrecompileBn256ScalarMul,
                PrecompileCalls::Bn128Pairing => ExecutionState::PrecompileBn256Pairing,
                PrecompileCalls::Blake2F => ExecutionState::PrecompileBlake2f,
            },
            ExecState::BeginTx => ExecutionState::BeginTx,
            ExecState::EndTx => ExecutionState::EndTx,
            ExecState::EndBlock => ExecutionState::EndBlock,
        }
    }
}

pub trait HasExecutionState {
    fn execution_state(&self) -> ExecutionState;
}

impl HasExecutionState for ExecStep {
    fn execution_state(&self) -> ExecutionState {
        ExecutionState::from(self)
    }
}

impl ExecutionState {
    pub(crate) const fn as_u64(&self) -> u64 {
        *self as u64
    }

    pub(crate) fn amount() -> usize {
        Self::iter().count()
    }

    pub(crate) fn halts_in_exception(&self) -> bool {
        matches!(
            self,
            Self::ErrorInvalidOpcode
                | Self::ErrorStack
                | Self::ErrorWriteProtection
                | Self::ErrorInvalidCreationCode
                | Self::ErrorMaxCodeSizeExceeded
                | Self::ErrorInvalidJump
                | Self::ErrorReturnDataOutOfBound
                | Self::ErrorOutOfGasConstant
                | Self::ErrorOutOfGasStaticMemoryExpansion
                | Self::ErrorOutOfGasDynamicMemoryExpansion
                | Self::ErrorOutOfGasMemoryCopy
                | Self::ErrorOutOfGasAccountAccess
                | Self::ErrorOutOfGasCodeStore
                | Self::ErrorOutOfGasLOG
                | Self::ErrorOutOfGasEXP
                | Self::ErrorOutOfGasSHA3
                | Self::ErrorOutOfGasEXTCODECOPY
                | Self::ErrorOutOfGasCall
                | Self::ErrorOutOfGasSloadSstore
                | Self::ErrorOutOfGasCREATE2
                | Self::ErrorOutOfGasSELFDESTRUCT
        )
    }

    pub(crate) fn halts(&self) -> bool {
        matches!(self, Self::STOP | Self::RETURN_REVERT | Self::SELFDESTRUCT)
            || self.halts_in_exception()
    }

    pub fn responsible_opcodes(&self) -> Vec<ResponsibleOp> {
        if matches!(self, Self::ErrorStack) {
            return OpcodeId::valid_opcodes()
                .into_iter()
                .flat_map(|op| {
                    op.invalid_stack_ptrs()
                        .into_iter()
                        .map(move |stack_ptr| ResponsibleOp::InvalidStackPtr(op, stack_ptr))
                })
                .collect();
        }

        match self {
            Self::STOP => vec![OpcodeId::STOP],
            Self::ADD_SUB => vec![OpcodeId::ADD, OpcodeId::SUB],
            Self::MUL_DIV_MOD => vec![OpcodeId::MUL, OpcodeId::DIV, OpcodeId::MOD],
            Self::SDIV_SMOD => vec![OpcodeId::SDIV, OpcodeId::SMOD],
            Self::SHL_SHR => vec![OpcodeId::SHL, OpcodeId::SHR],
            Self::ADDMOD => vec![OpcodeId::ADDMOD],
            Self::MULMOD => vec![OpcodeId::MULMOD],
            Self::EXP => vec![OpcodeId::EXP],
            Self::SIGNEXTEND => vec![OpcodeId::SIGNEXTEND],
            Self::CMP => vec![OpcodeId::LT, OpcodeId::GT, OpcodeId::EQ],
            Self::SCMP => vec![OpcodeId::SLT, OpcodeId::SGT],
            Self::ISZERO => vec![OpcodeId::ISZERO],
            Self::BITWISE => vec![OpcodeId::AND, OpcodeId::OR, OpcodeId::XOR],
            Self::NOT => vec![OpcodeId::NOT],
            Self::BYTE => vec![OpcodeId::BYTE],
            Self::SAR => vec![OpcodeId::SAR],
            Self::SHA3 => vec![OpcodeId::SHA3],
            Self::ADDRESS => vec![OpcodeId::ADDRESS],
            Self::BALANCE => vec![OpcodeId::BALANCE],
            Self::ORIGIN => vec![OpcodeId::ORIGIN],
            Self::CALLER => vec![OpcodeId::CALLER],
            Self::CALLVALUE => vec![OpcodeId::CALLVALUE],
            Self::CALLDATALOAD => vec![OpcodeId::CALLDATALOAD],
            Self::CALLDATASIZE => vec![OpcodeId::CALLDATASIZE],
            Self::CALLDATACOPY => vec![OpcodeId::CALLDATACOPY],
            Self::CODESIZE => vec![OpcodeId::CODESIZE],
            Self::CODECOPY => vec![OpcodeId::CODECOPY],
            Self::GASPRICE => vec![OpcodeId::GASPRICE],
            Self::EXTCODESIZE => vec![OpcodeId::EXTCODESIZE],
            Self::EXTCODECOPY => vec![OpcodeId::EXTCODECOPY],
            Self::RETURNDATASIZE => vec![OpcodeId::RETURNDATASIZE],
            Self::RETURNDATACOPY => vec![OpcodeId::RETURNDATACOPY],
            Self::EXTCODEHASH => vec![OpcodeId::EXTCODEHASH],
            Self::BLOCKHASH => vec![OpcodeId::BLOCKHASH],
            Self::BLOCKCTXU64 => vec![OpcodeId::TIMESTAMP, OpcodeId::NUMBER, OpcodeId::GASLIMIT],
            Self::BLOCKCTXU160 => vec![OpcodeId::COINBASE],
            Self::BLOCKCTXU256 => vec![OpcodeId::DIFFICULTY, OpcodeId::BASEFEE],
            Self::CHAINID => vec![OpcodeId::CHAINID],
            Self::SELFBALANCE => vec![OpcodeId::SELFBALANCE],
            Self::POP => vec![OpcodeId::POP],
            Self::MEMORY => {
                vec![OpcodeId::MLOAD, OpcodeId::MSTORE, OpcodeId::MSTORE8]
            }
            Self::SLOAD => vec![OpcodeId::SLOAD],
            Self::SSTORE => vec![OpcodeId::SSTORE],
            Self::JUMP => vec![OpcodeId::JUMP],
            Self::JUMPI => vec![OpcodeId::JUMPI],
            Self::PC => vec![OpcodeId::PC],
            Self::MSIZE => vec![OpcodeId::MSIZE],
            Self::GAS => vec![OpcodeId::GAS],
            Self::JUMPDEST => vec![OpcodeId::JUMPDEST],
            Self::PUSH0 => vec![OpcodeId::PUSH0],
            Self::PUSH => vec![
                OpcodeId::PUSH1,
                OpcodeId::PUSH2,
                OpcodeId::PUSH3,
                OpcodeId::PUSH4,
                OpcodeId::PUSH5,
                OpcodeId::PUSH6,
                OpcodeId::PUSH7,
                OpcodeId::PUSH8,
                OpcodeId::PUSH9,
                OpcodeId::PUSH10,
                OpcodeId::PUSH11,
                OpcodeId::PUSH12,
                OpcodeId::PUSH13,
                OpcodeId::PUSH14,
                OpcodeId::PUSH15,
                OpcodeId::PUSH16,
                OpcodeId::PUSH17,
                OpcodeId::PUSH18,
                OpcodeId::PUSH19,
                OpcodeId::PUSH20,
                OpcodeId::PUSH21,
                OpcodeId::PUSH22,
                OpcodeId::PUSH23,
                OpcodeId::PUSH24,
                OpcodeId::PUSH25,
                OpcodeId::PUSH26,
                OpcodeId::PUSH27,
                OpcodeId::PUSH28,
                OpcodeId::PUSH29,
                OpcodeId::PUSH30,
                OpcodeId::PUSH31,
                OpcodeId::PUSH32,
            ],
            Self::DUP => vec![
                OpcodeId::DUP1,
                OpcodeId::DUP2,
                OpcodeId::DUP3,
                OpcodeId::DUP4,
                OpcodeId::DUP5,
                OpcodeId::DUP6,
                OpcodeId::DUP7,
                OpcodeId::DUP8,
                OpcodeId::DUP9,
                OpcodeId::DUP10,
                OpcodeId::DUP11,
                OpcodeId::DUP12,
                OpcodeId::DUP13,
                OpcodeId::DUP14,
                OpcodeId::DUP15,
                OpcodeId::DUP16,
            ],
            Self::SWAP => vec![
                OpcodeId::SWAP1,
                OpcodeId::SWAP2,
                OpcodeId::SWAP3,
                OpcodeId::SWAP4,
                OpcodeId::SWAP5,
                OpcodeId::SWAP6,
                OpcodeId::SWAP7,
                OpcodeId::SWAP8,
                OpcodeId::SWAP9,
                OpcodeId::SWAP10,
                OpcodeId::SWAP11,
                OpcodeId::SWAP12,
                OpcodeId::SWAP13,
                OpcodeId::SWAP14,
                OpcodeId::SWAP15,
                OpcodeId::SWAP16,
            ],
            Self::LOG => vec![
                OpcodeId::LOG0,
                OpcodeId::LOG1,
                OpcodeId::LOG2,
                OpcodeId::LOG3,
                OpcodeId::LOG4,
            ],
            Self::CREATE => vec![OpcodeId::CREATE],
            Self::CALL_OP => vec![
                OpcodeId::CALL,
                OpcodeId::CALLCODE,
                OpcodeId::DELEGATECALL,
                OpcodeId::STATICCALL,
            ],
            Self::RETURN_REVERT => vec![OpcodeId::RETURN, OpcodeId::REVERT],
            Self::CREATE2 => vec![OpcodeId::CREATE2],
            Self::SELFDESTRUCT => vec![OpcodeId::SELFDESTRUCT],
            Self::ErrorInvalidOpcode => OpcodeId::invalid_opcodes(),
            _ => vec![],
        }
        .into_iter()
        .map(Into::into)
        .collect()
    }

    pub fn get_step_height_option(&self, is_taiko: bool) -> Option<usize> {
        if is_taiko {
            EXECUTION_STATE_HEIGHT_MAP_WITH_TAIKO.get(self).copied()
        } else {
            EXECUTION_STATE_HEIGHT_MAP.get(self).copied()
        }
    }

    pub fn get_step_height(&self, is_taiko: bool) -> usize {
        self.get_step_height_option(is_taiko)
            .unwrap_or_else(|| panic!("Execution state unknown: {:?}", self))
    }
}

/// Enum of Responsible opcode mapping to execution state.
#[derive(Debug)]
pub enum ResponsibleOp {
    /// Raw opcode
    Op(OpcodeId),
    /// Corresponding to ExecutionState::ErrorStack
    InvalidStackPtr(OpcodeId, u32),
}

/// Helper for easy transform from a raw OpcodeId to ResponsibleOp.
impl From<OpcodeId> for ResponsibleOp {
    fn from(opcode: OpcodeId) -> Self {
        Self::Op(opcode)
    }
}

impl ResponsibleOp {
    pub fn opcode(&self) -> OpcodeId {
        *match self {
            ResponsibleOp::Op(opcode) => opcode,
            ResponsibleOp::InvalidStackPtr(opcode, _) => opcode,
        }
    }
}

/// Dynamic selector that generates expressions of degree 2 to select from N
/// possible targets using N/2 + 1 cells.
#[derive(Clone, Debug)]
pub(crate) struct DynamicSelectorHalf<F> {
    /// N value: how many possible targets this selector supports.
    count: usize,
    /// Whether the target is odd.  `target % 2 == 1`.
    pub(crate) target_odd: Cell<F>,
    /// Whether the target belongs to each consecutive pair of targets.
    /// `in [0, 1], in [2, 3], in [4, 5], ...`
    pub(crate) target_pairs: Vec<Cell<F>>,
}

impl<F: Field> DynamicSelectorHalf<F> {
    pub(crate) fn new(cell_manager: &mut CellManager<F>, count: usize) -> Self {
        let target_pairs = cell_manager.query_cells(CellType::StoragePhase1, (count + 1) / 2);
        let target_odd = cell_manager.query_cell(CellType::StoragePhase1);
        Self {
            count,
            target_pairs,
            target_odd,
        }
    }

    /// Return the list of constraints that configure this "gadget".
    pub(crate) fn configure(&self) -> Vec<(&'static str, Expression<F>)> {
        // Only one of target_pairs should be enabled
        let sum_to_one = (
            "Only one of target_pairs should be enabled",
            self.target_pairs
                .iter()
                .fold(1u64.expr(), |acc, cell| acc - cell.expr()),
        );
        // Cells representation for target_pairs and target_odd should be bool.
        let bool_checks = iter::once(&self.target_odd)
            .chain(&self.target_pairs)
            .map(|cell| {
                (
                    "Representation for target_pairs and target_odd should be bool",
                    cell.expr() * (1u64.expr() - cell.expr()),
                )
            });
        let mut constraints: Vec<(&'static str, Expression<F>)> =
            iter::once(sum_to_one).chain(bool_checks).collect();
        // In case count is odd, we must forbid selecting N+1 with (odd = 1,
        // target_pairs[-1] = 1)
        if self.count % 2 == 1 {
            constraints.push((
                "Forbid N+1 target when N is odd",
                self.target_odd.expr() * self.target_pairs[self.count / 2].expr(),
            ));
        }
        constraints
    }

    pub(crate) fn selector(&self, targets: impl IntoIterator<Item = usize>) -> Expression<F> {
        targets
            .into_iter()
            .map(|target| {
                let odd = target % 2 == 1;
                let pair_index = target / 2;
                (if odd {
                    self.target_odd.expr()
                } else {
                    1.expr() - self.target_odd.expr()
                }) * self.target_pairs[pair_index].expr()
            })
            .reduce(|acc, expr| acc + expr)
            .expect("Select some Targets")
    }

    pub(crate) fn assign(
        &self,
        region: &mut CachedRegion<'_, '_, F>,
        offset: usize,
        target: usize,
    ) -> Result<(), Error> {
        let odd = target % 2 == 1;
        let pair_index = target / 2;
        self.target_odd.assign(
            region,
            offset,
            Value::known(if odd { F::ONE } else { F::ZERO }),
        )?;
        for (index, cell) in self.target_pairs.iter().enumerate() {
            cell.assign(
                region,
                offset,
                Value::known(if index == pair_index { F::ONE } else { F::ZERO }),
            )?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct StepState<F> {
    /// The execution state selector for the step
    pub(crate) execution_state: DynamicSelectorHalf<F>,
    /// The Read/Write counter
    pub(crate) rw_counter: Cell<F>,
    /// The unique identifier of call in the whole proof, using the
    /// `rw_counter` at the call step.
    pub(crate) call_id: Cell<F>,
    /// Whether the call is root call
    pub(crate) is_root: Cell<F>,
    /// Whether the call is a create call
    pub(crate) is_create: Cell<F>,
    /// Denotes the hash of the bytecode for the current call.
    /// In the case of a contract creation root call, this denotes the hash of
    /// the tx calldata.
    /// In the case of a contract creation internal call, this denotes the hash
    /// of the chunk of bytes from caller's memory that represent the
    /// contract init code.
    pub(crate) code_hash: Cell<F>,
    /// The program counter
    pub(crate) program_counter: Cell<F>,
    /// The stack pointer
    pub(crate) stack_pointer: Cell<F>,
    /// The amount of gas left
    pub(crate) gas_left: Cell<F>,
    /// Memory size in words (32 bytes)
    pub(crate) memory_word_size: Cell<F>,
    /// The counter for reversible writes
    pub(crate) reversible_write_counter: Cell<F>,
    /// The counter for log index
    pub(crate) log_id: Cell<F>,
}

#[derive(Clone, Debug)]
pub(crate) struct Step<F> {
    pub(crate) state: StepState<F>,
    pub(crate) cell_manager: CellManager<F>,
}

impl<F: Field> Step<F> {
    pub(crate) fn new(
        meta: &mut ConstraintSystem<F>,
        advices: [Column<Advice>; STEP_WIDTH],
        offset: usize,
        is_next: bool,
    ) -> Self {
        let height = if is_next {
            STEP_STATE_HEIGHT // Query only the state of the next step.
        } else {
            MAX_STEP_HEIGHT // Query the entire current step.
        };
        let mut cell_manager = CellManager::new(meta, height, &advices, offset);
        let state = {
            StepState {
                execution_state: DynamicSelectorHalf::new(
                    &mut cell_manager,
                    ExecutionState::amount(),
                ),
                rw_counter: cell_manager.query_cell(CellType::StoragePhase1),
                call_id: cell_manager.query_cell(CellType::StoragePhase1),
                is_root: cell_manager.query_cell(CellType::StoragePhase1),
                is_create: cell_manager.query_cell(CellType::StoragePhase1),
                code_hash: cell_manager.query_cell(CellType::StoragePhase2),
                program_counter: cell_manager.query_cell(CellType::StoragePhase1),
                stack_pointer: cell_manager.query_cell(CellType::StoragePhase1),
                gas_left: cell_manager.query_cell(CellType::StoragePhase1),
                memory_word_size: cell_manager.query_cell(CellType::StoragePhase1),
                reversible_write_counter: cell_manager.query_cell(CellType::StoragePhase1),
                log_id: cell_manager.query_cell(CellType::StoragePhase1),
            }
        };
        Self {
            state,
            cell_manager,
        }
    }

    pub(crate) fn execution_state_selector(
        &self,
        execution_states: impl IntoIterator<Item = ExecutionState>,
    ) -> Expression<F> {
        self.state
            .execution_state
            .selector(execution_states.into_iter().map(|s| s as usize))
    }

    pub(crate) fn assign_exec_step(
        &self,
        region: &mut CachedRegion<'_, '_, F>,
        offset: usize,
        _block: &Block<F>,
        call: &Call,
        step: &ExecStep,
    ) -> Result<(), Error> {
        self.state
            .execution_state
            .assign(region, offset, step.execution_state() as usize)?;
        self.state
            .rw_counter
            .assign(region, offset, Value::known(F::from(step.rwc.into())))?;
        self.state
            .call_id
            .assign(region, offset, Value::known(F::from(call.call_id as u64)))?;
        self.state
            .is_root
            .assign(region, offset, Value::known(F::from(call.is_root as u64)))?;
        self.state.is_create.assign(
            region,
            offset,
            Value::known(F::from(call.is_create() as u64)),
        )?;
        self.state
            .code_hash
            .assign(region, offset, region.word_rlc(call.code_hash.to_word()))?;
        self.state.program_counter.assign(
            region,
            offset,
            Value::known(F::from(step.program_counter())),
        )?;
        self.state.stack_pointer.assign(
            region,
            offset,
            Value::known(F::from(step.stack_pointer())),
        )?;
        self.state
            .gas_left
            .assign(region, offset, Value::known(F::from(step.gas_left.0)))?;
        self.state.memory_word_size.assign(
            region,
            offset,
            Value::known(F::from(step.memory_word_size())),
        )?;
        self.state.reversible_write_counter.assign(
            region,
            offset,
            Value::known(F::from(step.reversible_write_counter as u64)),
        )?;
        self.state
            .log_id
            .assign(region, offset, Value::known(F::from(step.log_id as u64)))?;
        Ok(())
    }
}
