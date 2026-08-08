// S-type
//
//  31        25 24    20 19    15 14   12 11      7 6      0
// | imm[11:5] |  rs2   |  rs1   | funct3 | imm[4:0] | opcode |
// |    7      |   5    |   5    |   3    |    5     |   7    |
//
// mem[rs1 + imm] <- rs2   (no rd -- the destination is memory, not a register)
// two register operands in, no register operand out, one 12-bit immediate
// split across two non-adjacent chunks.
// e.g. sb, sh, sw
use crate::instructions::Format;
use crate::fetcher::InstructionWord;
use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::codes::ExecutionSignal;

#[derive(Debug, PartialEq)]
pub enum SOp {
    Foo
}

pub fn parse_s_inst(raw_word: InstructionWord) -> Format {
    Format::SType { 
        op: SOp::Foo,
        imm: 1,
        rs1: 1,
        rs2: 1
    }
}

pub fn execute_s_type(op: &SOp, imm: i32, rs1: usize, rs2: usize, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    Ok(ExecutionSignal::Continue)
}