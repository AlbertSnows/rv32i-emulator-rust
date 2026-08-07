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

use crate::cpu_definition::build_register_file;
use crate::cpu_definition::RegisterFile;

enum AluOp {
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

pub fn parse_r_type(raw_word: InstructionWord) -> Instruction {
    
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