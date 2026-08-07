// J-type
//
//  31                                         12 11     7 6      0
// |       imm[20|10:1|11|19:12]                |   rd    | opcode |
// |                    20                      |    5    |   7    |
//
// rd <- pc + 4; pc <- pc + imm
// same field shape as U-type, but the immediate means "jump target offset"
// (PC-relative, always even, bit 0 implied zero) instead of raw upper bits.
// immediate bits arrive scrambled: [20][10:1][11][19:12].
// e.g. jal
use crate::instructions::Format;
use crate::fetcher::InstructionWord;

#[derive(Debug, PartialEq)]
pub enum JOp {
    Foo
}

pub fn parse_j_inst(raw_word: InstructionWord) -> Format {
    Format::JType { 
        op: JOp::Foo,
        rd: 1,
        imm: 1
    }
}
