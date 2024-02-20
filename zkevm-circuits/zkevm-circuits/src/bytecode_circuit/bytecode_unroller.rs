use crate::{
    table::BytecodeFieldTag,
    util::{get_push_size, keccak},
};
use eth_types::{Field, Word};
use std::vec;

/// Public data for the bytecode
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BytecodeRow<F: Field> {
    pub(crate) code_hash: Word,
    pub(crate) tag: F,
    pub(crate) index: F,
    pub(crate) is_code: F,
    pub(crate) value: F,
}

/// Unrolled bytecode
#[derive(Clone, Debug, PartialEq, Default)]
pub struct UnrolledBytecode<F: Field> {
    pub(crate) bytes: Vec<u8>,
    pub(crate) rows: Vec<BytecodeRow<F>>,
}

/// Get unrolled bytecode from raw bytes
pub fn unroll<F: Field>(bytes: Vec<u8>) -> UnrolledBytecode<F> {
    let code_hash = keccak(&bytes[..]);
    let mut rows = vec![BytecodeRow::<F> {
        code_hash,
        tag: F::from(BytecodeFieldTag::Header as u64),
        index: F::ZERO,
        is_code: F::ZERO,
        value: F::from(bytes.len() as u64),
    }];
    // Run over all the bytes
    let mut push_rindex = 0;
    for (index, byte) in bytes.iter().enumerate() {
        // Track which byte is an opcode and which is push data
        let is_code = push_rindex == 0;
        push_rindex = if is_code {
            get_push_size(*byte)
        } else {
            push_rindex - 1
        };

        rows.push(BytecodeRow::<F> {
            code_hash,
            tag: F::from(BytecodeFieldTag::Byte as u64),
            index: F::from(index as u64),
            is_code: F::from(is_code as u64),
            value: F::from(*byte as u64),
        });
    }
    UnrolledBytecode { bytes, rows }
}
