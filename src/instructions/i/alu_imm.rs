use crate::definitions::cpu_definition::RegisterFile;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;

#[derive(Debug, PartialEq)]
pub enum AluImmOp {
    Addi,
    Slti,
    Sltiu,
    Xori,
    Ori,
    Andi
}

pub fn parse_alu_imm_inst(raw_word: InstructionWord) -> Format {
    Format::AluImmType {
        op: AluImmOp::Addi,
        imm: 1,
        rd: 1,
        rs1: 1
    }
}

pub fn execute_i_alu_imm_type(op: &AluImmOp, rd: usize, rs1: usize, rs2: usize, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    ExecutionSignal::Continue
}