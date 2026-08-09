use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::codes::ExecutionSignal;
use crate::instructions::i::alu_imm_or_shift::IShOp;

pub fn execute_i_shift_type(op: &IShOp, rd: usize, rs1: usize, shamt: usize, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    Ok(ExecutionSignal::Continue)
}
