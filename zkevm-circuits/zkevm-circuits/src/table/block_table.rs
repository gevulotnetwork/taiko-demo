use super::*;

/// Tag to identify the field in a Block Table row
// Keep the sequence consistent with OpcodeId for scalar
#[derive(Clone, Copy, Debug)]
pub enum BlockContextFieldTag {
    /// Coinbase field
    Coinbase = 1,
    /// Timestamp field
    Timestamp,
    /// Number field
    Number,
    /// Difficulty field
    Difficulty,
    /// Gas Limit field
    GasLimit,
    /// Base Fee field
    BaseFee = 8,
    /// Block Hash field
    BlockHash,
    /// Chain ID field.  Although this is not a field in the block header, we
    /// add it here for convenience.
    ChainId,
    /// Treasury accounts for receiving block base fee
    Treasury,
}
impl_expr!(BlockContextFieldTag);

/// Table with Block header fields
#[derive(Clone, Debug)]
pub struct BlockTable {
    /// Tag
    pub tag: Column<Advice>,
    /// Index
    pub index: Column<Advice>,
    /// Value
    pub value: Column<Advice>,
}

impl BlockTable {
    /// Construct a new BlockTable
    pub fn construct<F: Field>(meta: &mut ConstraintSystem<F>) -> Self {
        Self {
            tag: meta.advice_column(),
            index: meta.advice_column(),
            value: meta.advice_column_in(SecondPhase),
        }
    }

    /// Assign the `BlockTable` from a `BlockContext`.
    pub fn load<F: Field>(
        &self,
        layouter: &mut impl Layouter<F>,
        block: &BlockContext,
        randomness: Value<F>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "block table",
            |mut region| {
                let mut offset = 0;
                for column in <BlockTable as LookupTable<F>>::advice_columns(self) {
                    region.assign_advice(
                        || "block table all-zero row",
                        column,
                        offset,
                        || Value::known(F::ZERO),
                    )?;
                }
                offset += 1;

                let block_table_columns = <BlockTable as LookupTable<F>>::advice_columns(self);
                for row in block.table_assignments(randomness) {
                    for (&column, value) in block_table_columns.iter().zip_eq(row) {
                        region.assign_advice(
                            || format!("block table row {}", offset),
                            column,
                            offset,
                            || value,
                        )?;
                    }
                    offset += 1;
                }

                Ok(())
            },
        )
    }
}

impl<F: Field> LookupTable<F> for BlockTable {
    fn columns(&self) -> Vec<Column<Any>> {
        vec![self.tag.into(), self.index.into(), self.value.into()]
    }

    fn annotations(&self) -> Vec<String> {
        vec![
            String::from("tag"),
            String::from("index"),
            String::from("value"),
        ]
    }
}
