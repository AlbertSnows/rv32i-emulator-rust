use crate::definitions::cpu_definition::RegisterFile;

// jalr -- the only instruction under its opcode, so no op enum needed
// (same reasoning as JType). Not yet implemented.
pub fn execute_i_jalr_type(rd: usize, rs1: usize, rs2: usize, register: &mut RegisterFile) {
    
}