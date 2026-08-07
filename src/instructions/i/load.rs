use crate::definitions::cpu_definition::RegisterFile;

#[derive(Debug, PartialEq)]
pub enum LoadOp {
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu
}

pub fn execute_i_load_type(op: &LoadOp, rd: usize, rs1: usize, rs2: usize, register: &mut RegisterFile) {
    
}