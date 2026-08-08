use crate::definitions::cpu_definition::RegisterFile;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::definitions::codes::ExecutionSignal;

// jalr -- the only instruction under its opcode, so no op enum needed
// (same reasoning as JType). Not yet implemented.

pub fn parse_jalr_inst(raw_word: InstructionWord) -> Format {
    Format::JalrType {
        imm: 1,
        rd: 1,
        rs1: 1
    }
}

pub fn execute_i_jalr_type(rd: usize, rs1: usize, imm: i32, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    Ok(ExecutionSignal::Continue)
}