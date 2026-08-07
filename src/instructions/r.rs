// R-type
//
//  31        25 24    20 19    15 14   12 11     7 6      0
// |  funct7   |  rs2   |  rs1   | funct3 |   rd    | opcode |
// |    7      |   5    |   5    |   3    |    5    |   7    |
//
// rd <- rs1 OP rs2   (funct3 + funct7 together select OP)
// two register operands in, one register operand out, no immediate.
// e.g. add, sub, and, or, xor, sll, srl, sra, slt, sltu
// rs is an index

use crate::definitions::cpu_definition::build_register_file;
use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::op_codes;
use crate::definitions::masks;
use crate::fetcher::InstructionWord;
use crate::instructions::Instruction;
use crate::utility::bit_operations::mask_and_shift;

#[derive(Debug)]
pub enum AluOp {
    Add, 
    Sub, 
    Sll, 
    Slt, 
    Sltu, 
    Xor, 
    Srl, 
    Sra, 
    Or, 
    And
}

pub fn parse_r_inst(raw_word: InstructionWord) -> Instruction {
    let content = raw_word.0;
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    let funct_three = mask_and_shift(content, masks::FUNCT_3);
    let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    let reg_source_two = mask_and_shift(content, masks::REG_SOURCE_TWO);
    let funct_seven = mask_and_shift(content, masks::FUNCT_7);
    let instruction_name = match (funct_seven, funct_three) {
        (0b0000000, 0b000) => AluOp::Add,
        (0b0100000, 0b000) => AluOp::Sub,
        (0b0000000, 0b001) => AluOp::Sll,
        (0b0000000, 0b010) => AluOp::Slt,
        (0b0000000, 0b011) => AluOp::Sltu,
        (0b0000000, 0b100) => AluOp::Xor,
        (0b0000000, 0b101) => AluOp::Srl,
        (0b0100000, 0b101) => AluOp::Sra,
        (0b0000000, 0b110) => AluOp::Or,
        (0b0000000, 0b111) => AluOp::And,
        _ => panic!("undefined R type")
    };
    Instruction::RType { 
        op: instruction_name, 
        rd: reg_dest as usize, 
        rs1: reg_source_one as usize, 
        rs2: reg_source_two as usize 
    }
}

pub fn inst_r_add(rs1: usize, rs2: usize, rd: usize, reg_file: &mut RegisterFile) -> &mut RegisterFile {
    let storage = reg_file.storage;
    let left = storage[rs1];
    let right = storage[rs2];
    let sum = left + right;
    reg_file.storage[rd] = sum;
    reg_file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let rs1 = 1;
        let rs2 = 3;
        let rd = 30;
        let mut rf = build_register_file();
        rf.storage[1] = 3;
        rf.storage[3] = 4;
        inst_r_add(rs1, rs2, rd, &mut rf);
        assert_eq!(rf.storage[rd], 7)
    }
}