use crate::evm_circuit::{
    step::ExecutionState,
    table::Table,
    util::{constraint_builder::EVMConstraintBuilder, CellType},
};
use eth_types::Field;
use itertools::Itertools;

type StepSize = Vec<(CellType, ColumnSize)>;
/// Contains (width, height, num_cells)
type ColumnSize = (usize, usize, usize);

/// Instrument captures metrics during the compilation of a circuit.
#[derive(Clone, Debug, Default)]
pub struct Instrument {
    // States -> Cell Types -> (width, height, num_cells)
    states: Vec<(ExecutionState, StepSize)>,
}

impl Instrument {
    /// Collects `CellManager` stats from a compiled EVMCircuit in order to
    /// extract metrics.
    pub(crate) fn on_gadget_built<F: Field>(
        &mut self,
        execution_state: ExecutionState,
        cb: &EVMConstraintBuilder<F>,
    ) {
        let sizes = cb
            .curr
            .cell_manager
            .get_stats()
            .into_iter()
            .sorted()
            .collect::<Vec<_>>();

        self.states.push((execution_state, sizes));
    }

    /// Dissasembles the instrumentation data and returns a collection of
    /// `ExecStateReport`s. One for each EVM `ExecutionState`.
    pub fn analyze(&self) -> Vec<ExecStateReport> {
        let mut report_collection = vec![];
        for (state, sizes) in &self.states {
            // Create a state report
            let mut report = ExecStateReport::from(state);
            // Compute max_height required for any kind of CellType for the current
            // `ExecutionState`.
            let top_height: usize = sizes.iter().map(|(_, (_, h, _))| *h).max().unwrap();

            // Obtain `ExecutionState` metrics per column type.
            for (cell_type, (width, _, cells)) in sizes {
                let unused_cells = width * top_height - cells;
                let total_available_cells = width * top_height;
                let utilization =
                    ((*cells as f64) / (*width as f64 * top_height as f64) * 100f64).round();

                let data_entry = StateReportRow {
                    available_cells: total_available_cells,
                    unused_cells,
                    used_cells: *cells,
                    top_height,
                    used_columns: cells / top_height,
                    utilization,
                };

                match cell_type {
                    CellType::StoragePhase1 => {
                        report.storage_1 = data_entry;
                    }
                    CellType::StoragePhase2 => {
                        report.storage_2 = data_entry;
                    }
                    CellType::StoragePermutation => {
                        report.storage_perm = data_entry;
                    }
                    CellType::LookupByte => {
                        report.byte_lookup = data_entry;
                    }
                    CellType::Lookup(Table::Fixed) => {
                        report.fixed_table = data_entry;
                    }
                    CellType::Lookup(Table::Tx) => {
                        report.tx_table = data_entry;
                    }
                    CellType::Lookup(Table::Rw) => {
                        report.rw_table = data_entry;
                    }
                    CellType::Lookup(Table::Bytecode) => {
                        report.bytecode_table = data_entry;
                    }
                    CellType::Lookup(Table::Block) => {
                        report.block_table = data_entry;
                    }
                    CellType::Lookup(Table::Copy) => {
                        report.copy_table = data_entry;
                    }
                    CellType::Lookup(Table::Keccak) => {
                        report.keccak_table = data_entry;
                    }
                    CellType::Lookup(Table::Exp) => {
                        report.exp_table = data_entry;
                    }
                }
            }
            report_collection.push(report);
        }
        report_collection
    }
}

/// Struct which contains a Cost/ColumnType report for a particular EVM
/// `ExecutionStep`.
#[derive(Clone, Debug, Default)]
pub struct ExecStateReport {
    pub state: ExecutionState,
    pub storage_1: StateReportRow,
    pub storage_2: StateReportRow,
    pub storage_perm: StateReportRow,
    pub byte_lookup: StateReportRow,
    pub fixed_table: StateReportRow,
    pub tx_table: StateReportRow,
    pub rw_table: StateReportRow,
    pub bytecode_table: StateReportRow,
    pub block_table: StateReportRow,
    pub copy_table: StateReportRow,
    pub keccak_table: StateReportRow,
    pub exp_table: StateReportRow,
}

impl From<ExecutionState> for ExecStateReport {
    fn from(state: ExecutionState) -> Self {
        ExecStateReport {
            state,
            ..Default::default()
        }
    }
}

impl From<&ExecutionState> for ExecStateReport {
    fn from(state: &ExecutionState) -> Self {
        ExecStateReport {
            state: *state,
            ..Default::default()
        }
    }
}

/// Struct that contains all of the measurament values required to evaluate the
/// costs of a particular `ColumnType` of an `ExecStateReport`
#[derive(Debug, Clone, Default)]
pub struct StateReportRow {
    // Given a rigion of x columns and y rows, we have x * y cells available for computation.
    pub available_cells: usize,
    // The cells not used in the computation in the x*y region. These are the wasted cells.
    pub unused_cells: usize,
    // The cells used in the computation in the x*y region.
    pub used_cells: usize,
    // The largest y within all the `CellType`.
    pub top_height: usize,
    // If we fully utilize y, how large is the x really needed?
    pub used_columns: usize,
    // The percentage of cells used in computation in the x * y region.
    pub utilization: f64,
}
