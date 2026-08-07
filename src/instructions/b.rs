// B-type
//
//  31            25 24    20 19    15 14   12 11          7 6      0
// | imm[12|10:5]  |  rs2   |  rs1   | funct3 | imm[4:1|11]  | opcode |
// |     7         |   5    |   5    |   3    |     5        |   7    |
//
// pc <- pc + ((rs1 CMP rs2) ? imm : 4)   (no rd -- branches never write a register)
// same field shape as S-type, but the immediate means "branch offset" and is
// always even, so bit 0 is implied zero and isn't stored -- one extra bit of
// range for free. immediate bits arrive scrambled: [12][10:5] ... [4:1][11].
// e.g. beq, bne, blt, bge, bltu, bgeu
use crate::instructions::Format;
use crate::fetcher::InstructionWord;

#[derive(Debug, PartialEq)]
pub enum BOp {
    Foo
}

pub fn parse_b_inst(raw_word: InstructionWord) -> Format {
    Format::BType { 
        op: BOp::Foo,
        imm: 1,
        rs1: 1,
        rs2: 1
    }
}
