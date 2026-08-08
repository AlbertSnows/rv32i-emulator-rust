use crate::definitions::cpu_definition::RegisterFile;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::definitions::codes::ExecutionSignal;

#[derive(Debug, PartialEq)]
pub enum AluImmOp {
    Addi,
    Slti,
    Sltiu,
    Xori,
    Ori,
    Andi
}

pub fn parse_alu_imm_inst(raw_word: InstructionWord) -> Result<Format, String> {
    Ok(Format::AluImmType {
        op: AluImmOp::Addi,
        imm: 1,
        rd: 1,
        rs1: 1
    })
}

pub fn execute_i_alu_imm_type(op: &AluImmOp, rd: usize, rs1: usize, imm: i32, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    Ok(ExecutionSignal::Continue)
}