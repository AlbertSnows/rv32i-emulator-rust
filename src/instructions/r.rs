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
use crate::instructions::Format;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::codes::ExecutionSignal;

#[derive(Debug, PartialEq)]
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

pub fn parse_r_inst(raw_word: InstructionWord) -> Result<Format, String> {
    let content = raw_word.0;
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    let funct_three = mask_and_shift(content, masks::FUNCT_THREE);
    let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    let reg_source_two = mask_and_shift(content, masks::REG_SOURCE_TWO);
    let funct_seven = mask_and_shift(content, masks::FUNCT_SEVEN);
    let instruction_name = match (funct_seven, funct_three) {
        (0b0000000, 0b000) => Ok(AluOp::Add),
        (0b0100000, 0b000) => Ok(AluOp::Sub),
        (0b0000000, 0b001) => Ok(AluOp::Sll),
        (0b0000000, 0b010) => Ok(AluOp::Slt),
        (0b0000000, 0b011) => Ok(AluOp::Sltu),
        (0b0000000, 0b100) => Ok(AluOp::Xor),
        (0b0000000, 0b101) => Ok(AluOp::Srl),
        (0b0100000, 0b101) => Ok(AluOp::Sra),
        (0b0000000, 0b110) => Ok(AluOp::Or),
        (0b0000000, 0b111) => Ok(AluOp::And),
        _ => Err(format!("undefined R type"))
    }?;
    Ok(Format::RType { 
        op: instruction_name, 
        rd: reg_dest as usize, 
        rs1: reg_source_one as usize, 
        rs2: reg_source_two as usize 
    })
}

pub fn execute_r_type(op: &AluOp, rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    match op {
        AluOp::Add => {
            inst_r_add(rs1, rs2, rd, reg_file);
        }
        AluOp::Sub => inst_r_sub(),
        AluOp::Sll => inst_r_sll(),
        AluOp::Slt => inst_r_slt(),
        AluOp::Sltu => inst_r_sltu(),
        AluOp::Xor => inst_r_xor(),
        AluOp::Srl => inst_r_srl(),
        AluOp::Sra => inst_r_sra(),
        AluOp::Or => inst_r_or(),
        AluOp::And => inst_r_and(),
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_r_sub() {
    // rd <- rs1 - rs2
    let left = reg_file.read(rs1);
    let right = reg_file.read(rs2);
    let outcome = left.wrapping_sub(right);
    reg_file.write(rd, outcome);
}

pub fn inst_r_sll() {
    // rd <- rs1 << rs2[4:0]
    let left = reg_file.read(rs1);
    let right = reg_file.read(rs2);
    let right_nibble = right & 0b1_1111;
    let outcome = left << right_nibble;
    reg_file.write(rd, outcome);
}

pub fn inst_r_slt() {
    // rd <- (rs1 <s rs2) ? 1 : 0
    let left = reg_file.read(rs1) as i32;
    let right = reg_file.read(rs2) as i32;
    let comparison = left < right;
    let bit = if comparison { 1 } else { 0 };
    reg_file.write(rd, bit);
}

pub fn inst_r_sltu() {
    // rd <- (rs1 <u rs2) ? 1 : 0
    let left = reg_file.read(rs1);
    let right = reg_file.read(rs2);
    let comparison = left < right;
    let bit = if comparison { 1 } else { 0 };
    reg_file.write(rd, bit);
}

pub fn inst_r_xor() {
    // rd <- rs1 ^ rs2
    let left = reg_file.read(rs1) as i32;
    let right = reg_file.read(rs2) as i32;
    let exponential_outcome = left ^ right;
    reg_file.write(rd, exponential_outcome);
}

pub fn inst_r_srl() {
    // rd <- rs1 >>u rs2[4:0]
    let left = reg_file.read(rs1) as u32;
    let right = reg_file.read(rs2) as u32;
    let right_nibble = right & 0b1_1111;
    let shifted_left = left >> right_nibble;
    reg_file.write(rd, shifted_left);
}

pub fn inst_r_sra() {
    // rd <- rs1 >>s rs2[4:0]
    // shift by the first 5 bits of rs2
    let left = reg_file.read(rs1) as i32;
    let right = reg_file.read(rs2) as i32;
    let right_nibble = right & 0b1_1111;
    let shifted_left = left >> right_nibble;
    reg_file.write(rd, shifted_left);
}

pub fn inst_r_or() {
    // rd <- rs1 | rs2
    let left = reg_file.read(rs1) as i32;
    let right = reg_file.read(rs2) as i32;
    let rs_or = left | right;
    reg_file.write(rd, rs_or);
}

pub fn inst_r_and() {
    // rd <- rs1 & rs2
    let left = reg_file.read(rs1) as i32;
    let right = reg_file.read(rs2) as i32;
    let rs_and = left & right;
    reg_file.write(rd, rs_and);
}

pub fn inst_r_add(rs1: usize, rs2: usize, rd: usize, reg_file: &mut RegisterFile) -> &mut RegisterFile {
    let left = reg_file.read(rs1);
    let right = reg_file.read(rs2);
    // the hardware wraps by defalut
    // https://docs.riscv.org/reference/isa/_attachments/riscv-unprivileged.pdf
    // "We did not include special instruction-set support for overflow checks on integer arithmetic
    // operations in the base instruction set, as many overflow checks can be cheaply implemented
    // using RISC-V branches"
    let sum = left.wrapping_add(right);
    reg_file.write(rd, sum);
    Ok(ExecutionSignal::Continue)
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
        rf.write(1, 3);
        rf.write(3, 4);
        inst_r_add(rs1, rs2, rd, &mut rf);
        assert_eq!(rf.read(rd), 7)
    }

    #[test]
    fn test_parse_r_inst_add() {
        use crate::programs::instructions::ADD_X3_X1_X2;

        // add x3, x1, x2
        let raw_word = InstructionWord(ADD_X3_X1_X2);
        let result = parse_r_inst(raw_word);
        assert_eq!(result.unwrap(), Format::RType { op: AluOp::Add, rd: 3, rs1: 1, rs2: 2 });
    }

    #[test]
    fn test_parse_r_inst_sub() {
        // sub x5, x1, x2 -- funct7=0b0100000, funct3=0b000, rd=5, rs1=1, rs2=2,
        // opcode=0b0110011. Same field-packing process as the add x3,x1,x2
        // walkthrough, just with sub's funct7 instead of add's.
        let raw_word = InstructionWord(0x402082B3);
        let result = parse_r_inst(raw_word);
        assert_eq!(result.unwrap(), Format::RType { op: AluOp::Sub, rd: 5, rs1: 1, rs2: 2 });
    }

    #[test]
    fn test_parse_r_inst_invalid_combo_panics() {
        // funct7=0b0000001, funct3=0b000 -- not a real combination for any
        // R-type instruction (only 0b0000000 and 0b0100000 are valid funct7
        // values), so this should hit the catch-all err
        let raw_word = InstructionWord(0x02000033);
        let outcome = parse_r_inst(raw_word);
        assert!(outcome.is_err());
    }

    #[test]
    fn test_inst_r_sub() {
        let mut reg = build_register_file();
        reg.write(2, 3);
        reg.write(3, 8);
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_sub(rs1, rs2, rd, reg);
        assert_eq!(reg.read(5), -5);
    }

    #[test]
    fn test_inst_r_sll() {
        let mut reg = build_register_file();
        reg.write(2, 5); // 101
        reg.write(3, 0b0_0011);
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_sll(rs1, rs2, rd, reg);
        assert_eq!(reg.read(5), 0b10_1000);
    }

    #[test]
    fn test_inst_r_slt() {
        let mut reg = build_register_file();
        reg.write(2, -22);
        reg.write(3, 33);
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_slt(rs1, rs2, rd, reg);
        assert_eq!(reg.read(5), 1);
        reg.write(2, 33);
        reg.write(3, -22);
        inst_r_slt(rs1, rs2, rd, reg);
        assert_eq!(reg.read(5), 0);

    }

    #[test]
    fn test_inst_r_sltu() {
        let mut reg = build_register_file();
        reg.write(2, 33);
        reg.write(3, 18);
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_sltu(rs1, rs2, rd, reg);
        assert_eq!(reg.read(5), 0);
        reg.write(2, 18);
        reg.write(3, 33);
        inst_r_sltu(rs1, rs2, rd, reg);
        assert_eq!(reg.read(5), 1);
    }

    #[test]
    fn test_inst_r_xor() {
        let mut reg = build_register_file();
        reg.write(2, 3); // 011
        reg.write(3, 5); // 101
        // 011 xor 101 = 110
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_xor(rs1, rs2, rd, reg);
        assert_eq!(reg.read(5), 6);
    }

    #[test]
    fn test_inst_r_srl() {
        let mut reg = build_register_file();
        reg.write(2, 31); // 1_1111
        reg.write(3, 2); // 00_0010
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_srl(rs1, rs2, rd, reg);
        assert_eq!(reg.read(5), 7) 
    }

    #[test]
    fn test_inst_r_sra() {
        let mut reg = build_register_file();
        reg.write(2, 12);
        // 30 = 0001_1110
        // -30 = 30 -> flip = 1110_0001 + 1 = 1110_0010
        // take 5 -> 0_0010 -> shift amount is 2
        // rs1 = 12 = 0b1100 -> shift = 11 = 3 
        reg.write(3, -30);
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_sra(rs1, rs2, rd, reg);
        assert_eq!(reg.read(5), 3);
    }

    #[test]
    fn test_inst_r_or() {
        let mut reg = build_register_file();
        reg.write(2, 3); // 11
        reg.write(3, 8); // 1000
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_or(rs1, rs2, rd, reg);
        assert_eq!(reg.read(5), 11); // 1011
    }

    #[test]
    fn test_inst_r_and() {
        let mut reg = build_register_file();
        reg.write(2, 3); // 0011
        reg.write(3, 9); // 1001
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_and(rs1, rs2, rd, reg);
        assert_eq!(reg.read(5), 1);
    }
}