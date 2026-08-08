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
        AluOp::Sub => todo!(),
        AluOp::Sll => todo!(),
        AluOp::Slt => todo!(),
        AluOp::Sltu => todo!(),
        AluOp::Xor => todo!(),
        AluOp::Srl => todo!(),
        AluOp::Sra => todo!(),
        AluOp::Or => todo!(),
        AluOp::And => todo!(),
    }
    Ok(ExecutionSignal::Continue)
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
        rf.write(1, 3);
        rf.write(3, 4);
        inst_r_add(rs1, rs2, rd, &mut rf);
        assert_eq!(rf.read(rd), 7)
    }

    #[test]
    fn test_parse_r_inst_add() {
        // add x3, x1, x2
        let raw_word = InstructionWord(0x002081B3);
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
        // values), so this should hit the catch-all and panic.
        let raw_word = InstructionWord(0x02000033);
        let outcome = parse_r_inst(raw_word);
        assert!(outcome.is_err());
    }
}