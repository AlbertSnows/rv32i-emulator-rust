use crate::definitions::cpu_definition::RegisterFile;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;

#[derive(Debug, PartialEq)]
pub enum LoadOp {
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu
}

pub fn parse_load_inst(raw_word: InstructionWord) -> Format {
    Format::LoadType {
        imm: 1,
        op: LoadOp::Lb,
        rd: 1,
        rs1: 1
    }
}


pub fn execute_i_load_type(op: &LoadOp, rd: usize, rs1: usize, rs2: usize, register: &mut RegisterFile) {
    
}