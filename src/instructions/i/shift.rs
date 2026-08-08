use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::codes::ExecutionSignal;

#[derive(Debug, PartialEq)]
pub enum IShOp {
    Slli,
    Srli,
    Srai
}

pub fn execute_i_shift_type(op: &IShOp, rd: usize, rs1: usize, shamt: usize, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    Ok(ExecutionSignal::Continue)
}
