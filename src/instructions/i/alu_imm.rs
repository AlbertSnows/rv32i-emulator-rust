use crate::definitions::cpu_definition::RegisterFile;

#[derive(Debug, PartialEq)]
pub enum AluImmOp {
    Addi,
    Slti,
    Sltiu,
    Xori,
    Ori,
    Andi
}

pub fn execute_i_alu_imm_type(op: &AluImmOp, rd: usize, rs1: usize, rs2: usize, register: &mut RegisterFile) {
    
}