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

use crate::cpu::definitions::cpu::cpu_definition::{build_register_file, RegisterFile};
use crate::cpu::definitions::{op_codes, masks};
use crate::cpu::fetcher::InstructionWord;
use crate::cpu::instructions::Format;
use crate::cpu::utility::bit_operations::mask_and_shift;
use crate::cpu::definitions::codes::ExecutionSignal;
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::cpu::utility::bit_operations::{extract_sub_bytes};
use crate::cpu::utility::types::{ByteType};

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
    And,
    Mul,
    Mulh,
    Mulhsu,
    Mulhu,
    Div,
    Divu,
    Rem,
    Remu
}

pub fn parse_r_inst(raw_word: InstructionWord) -> Result<Format, TrapCause> {
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

        (0b0000001, 0b000) => Ok(AluOp::Mul),
        (0b0000001, 0b001) => Ok(AluOp::Mulh),
        (0b0000001, 0b010) => Ok(AluOp::Mulhsu),
        (0b0000001, 0b011) => Ok(AluOp::Mulhu),
        (0b0000001, 0b100) => Ok(AluOp::Div),
        (0b0000001, 0b101) => Ok(AluOp::Divu),
        (0b0000001, 0b110) => Ok(AluOp::Rem),
        (0b0000001, 0b111) => Ok(AluOp::Remu),
        _ => Err(TrapCause::IllegalInstruction { instruction: Some(content) })
    }?;
    Ok(Format::RType { 
        op: instruction_name, 
        rd: reg_dest as usize, 
        rs1: reg_source_one as usize, 
        rs2: reg_source_two as usize 
    })
}

pub fn execute_r_type(op: &AluOp, rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) -> Result<ExecutionSignal, TrapCause> {
    let inst_fn = match op {
        AluOp::Add => inst_r_add,
        AluOp::Sub => inst_r_sub,
        AluOp::Sll => inst_r_sll,
        AluOp::Slt => inst_r_slt,
        AluOp::Sltu => inst_r_sltu,
        AluOp::Xor => inst_r_xor,
        AluOp::Srl => inst_r_srl,
        AluOp::Sra => inst_r_sra,
        AluOp::Or => inst_r_or,
        AluOp::And => inst_r_and,
        AluOp::Mul => inst_r_mul,
        AluOp::Mulh => inst_r_mulh,
        AluOp::Mulhsu => inst_r_mulsu,
        AluOp::Mulhu => inst_r_mulhu,
        AluOp::Div => inst_r_div,
        AluOp::Divu => inst_r_divu,
        AluOp::Rem => inst_r_rem,
        AluOp::Remu => inst_r_remu
    };
    inst_fn(rd, rs1, rs2, reg_file);
    Ok(ExecutionSignal::Continue)
}

pub fn inst_r_sub(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 - rs2
    let left = reg_file.read(rs1);
    let right = reg_file.read(rs2);
    let outcome = left.wrapping_sub(right);
    reg_file.write(rd, outcome);
}

pub fn inst_r_sll(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 << rs2[4:0]
    let left = reg_file.read(rs1);
    let right = reg_file.read(rs2);
    let right_nibble = right & 0b1_1111;
    let outcome = left << right_nibble;
    reg_file.write(rd, outcome);
}

pub fn inst_r_slt(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- (rs1 <s rs2) ? 1 : 0
    let left = reg_file.read(rs1) as i32;
    let right = reg_file.read(rs2) as i32;
    let comparison = left < right;
    let bit = if comparison { 1 } else { 0 };
    reg_file.write(rd, bit);
}

pub fn inst_r_sltu(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- (rs1 <u rs2) ? 1 : 0
    let left = reg_file.read(rs1);
    let right = reg_file.read(rs2);
    let comparison = left < right;
    let bit = if comparison { 1 } else { 0 };
    reg_file.write(rd, bit);
}

pub fn inst_r_xor(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 ^ rs2
    let left = reg_file.read(rs1) as i32;
    let right = reg_file.read(rs2) as i32;
    let exponential_outcome = left ^ right;
    reg_file.write(rd, exponential_outcome as u32);
}

pub fn inst_r_srl(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 >>u rs2[4:0]
    let left = reg_file.read(rs1) as u32;
    let right = reg_file.read(rs2) as u32;
    let right_nibble = right & 0b1_1111;
    let shifted_left = left >> right_nibble;
    reg_file.write(rd, shifted_left as u32);
}

pub fn inst_r_sra(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 >>s rs2[4:0]
    // shift by the first 5 bits of rs2
    let left = reg_file.read(rs1) as i32;
    let right = reg_file.read(rs2) as i32;
    let right_nibble = right & 0b1_1111;
    let shifted_left = left >> right_nibble;
    reg_file.write(rd, shifted_left as u32);
}

pub fn inst_r_or(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 | rs2
    let left = reg_file.read(rs1) as i32;
    let right = reg_file.read(rs2) as i32;
    let rs_or = left | right;
    reg_file.write(rd, rs_or as u32);
}

pub fn inst_r_and(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 & rs2
    let left = reg_file.read(rs1) as i32;
    let right = reg_file.read(rs2) as i32;
    let rs_and = left & right;
    reg_file.write(rd, rs_and as u32);
}

pub fn inst_r_add(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    let left = reg_file.read(rs1);
    let right = reg_file.read(rs2);
    // the hardware wraps by defalut
    // https://docs.riscv.org/reference/isa/_attachments/riscv-unprivileged.pdf
    // "We did not include special instruction-set support for overflow checks on integer arithmetic
    // operations in the base instruction set, as many overflow checks can be cheaply implemented
    // using RISC-V branches"
    let sum = left.wrapping_add(right);
    reg_file.write(rd, sum);
}

pub fn inst_r_mul(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- (rs1 * rs2)[31:0] -- low 32 bits of the full product.
    // Overflow (the product not fitting in 32 bits) is ignored, same as
    // ADD/SUB -- only the low bits are kept regardless of sign.
    let rs1_val = reg_file.read(rs1);
    let rs2_val = reg_file.read(rs2);
    let mult_val = rs1_val.wrapping_mul(rs2_val);
    reg_file.write(rd, mult_val);
}

pub fn inst_r_mulh(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- (rs1 * rs2)[63:32] -- upper 32 bits of the full 64-bit
    // product, both operands treated as signed.

    // assume rs1 holds 0xFF_FF_FF_FF, call this V
    // that reads as: 1111_1111_1111_1111_1111_1111_1111_1111
    // .read() returns u32. what happens when you cast u32 as i64?
    // answer: V becomes ..._0000_1111_... @ the 31 bit location.
    // but if we want to maintain V as sign, the 31nd bit needs to extend to the 63rd.
    // thus, we must tell rust it's signed first, then recast it
    // hence, as i32 as i64
    let rs1_val = reg_file.read(rs1) as i32 as i64;
    let rs2_val = reg_file.read(rs2) as i32 as i64;
    let mult_val = (rs1_val.wrapping_mul(rs2_val));
    let high_of_mult = extract_sub_bytes(mult_val as u64, 4, ByteType::Word);
    reg_file.write(rd, high_of_mult as u32);
}

pub fn inst_r_mulsu(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- (rs1 * rs2)[63:32] -- upper 32 bits of the full 64-bit
    // product, rs1 treated as signed, rs2 treated as unsigned.
    let rs1_val = reg_file.read(rs1) as i32 as i64;
    let rs2_val = reg_file.read(rs2) as i64;
    let mult_val = (rs1_val.wrapping_mul(rs2_val));
    let high_of_mult = extract_sub_bytes(mult_val as u64, 4, ByteType::Word);
    reg_file.write(rd, high_of_mult as u32);
}

pub fn inst_r_mulhu(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- (rs1 * rs2)[63:32] -- upper 32 bits of the full 64-bit
    // product, both operands treated as unsigned.
    let rs1_val = reg_file.read(rs1) as u64;
    let rs2_val = reg_file.read(rs2) as u64;
    let mult_val = (rs1_val.wrapping_mul(rs2_val));
    let high_of_mult = extract_sub_bytes(mult_val, 4, ByteType::Word);
    reg_file.write(rd, high_of_mult as u32);
}

pub fn inst_r_div(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 / rs2, signed, rounding toward zero (not floor).
    // Never traps, fully-defined results for the degenerate cases:
    //   rs2 == 0            -> rd = -1 (all bits set)
    //   rs1 == i32::MIN &&
    //   rs2 == -1 (overflow) -> rd = i32::MIN (the dividend, unchanged)
    let rs1_val = reg_file.read(rs1) as i32;
    let rs2_val = reg_file.read(rs2) as i32;
    let div_val = if rs2_val == 0 {
        -1
    } else if rs1_val == i32::MIN && rs2_val == -1 {
        i32::MIN
    } else {
        (rs1_val / rs2_val) as i32
    };
    reg_file.write(rd, div_val as u32);
}

pub fn inst_r_divu(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 / rs2, unsigned. Never traps:
    //   rs2 == 0 -> rd = u32::MAX (all bits set)
    // No overflow case exists for unsigned division.
    let rs1_val = reg_file.read(rs1) as u32;
    let rs2_val = reg_file.read(rs2) as u32;
    let div_val = if rs2_val == 0 {
        u32::MAX
    } else {
        (rs1_val / rs2_val) as u32
    };
    reg_file.write(rd, div_val as u32);
}

pub fn inst_r_rem(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 % rs2, signed. Sign of a nonzero result matches the
    // dividend (rs1), not the divisor. Never traps:
    //   rs2 == 0             -> rd = rs1 (the dividend, unchanged)
    //   rs1 == i32::MIN &&
    //   rs2 == -1 (overflow) -> rd = 0
    let rs1_val = reg_file.read(rs1) as i32;
    let rs2_val = reg_file.read(rs2) as i32;
    let mod_val = if rs2_val == 0 {
        rs1_val
    } else if rs1_val == i32::MIN && rs2_val == -1 {
        0
    } else {
        (rs1_val % rs2_val) as i32
    };
    reg_file.write(rd, mod_val as u32);
}

pub fn inst_r_remu(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 % rs2, unsigned. Never traps:
    //   rs2 == 0 -> rd = rs1 (the dividend, unchanged)
    let rs1_val = reg_file.read(rs1) as u32;
    let rs2_val = reg_file.read(rs2) as u32;
    let mod_val = if rs2_val == 0 {
        rs1_val
    } else {
        (rs1_val % rs2_val) as u32
    };
    reg_file.write(rd, mod_val as u32);
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
        inst_r_add(rd, rs1, rs2, &mut rf);
        assert_eq!(rf.read(rd), 7)
    }

    #[test]
    fn test_parse_r_inst_add() {
        use crate::cpu::programs::instructions::ADD_X3_X1_X2;

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
        // funct7=0b1111111, funct3=0b000 -- not a real combination for any
        // R-type instruction (only 0b0000000 and 0b0100000 are valid funct7
        // values), so this should hit the catch-all err
        let raw_word = InstructionWord(0x04_00_00_33);
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
        inst_r_sub(rd, rs1, rs2, &mut reg);
        assert_eq!(reg.read(5) as i32, -5);
    }

    #[test]
    fn test_inst_r_sll() {
        let mut reg = build_register_file();
        reg.write(2, 5); // 101
        reg.write(3, 0b0_0011);
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_sll(rd, rs1, rs2, &mut reg);
        assert_eq!(reg.read(5), 0b10_1000);
    }

    #[test]
    fn test_inst_r_slt() {
        let mut reg = build_register_file();
        reg.write(2, (-22i32) as u32);
        reg.write(3, 33);
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_slt(rd, rs1, rs2, &mut reg);
        assert_eq!(reg.read(5), 1);
        reg.write(2, 33);
        reg.write(3, (-22i32) as u32);
        inst_r_slt(rd, rs1, rs2, &mut reg);
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
        inst_r_sltu(rd, rs1, rs2, &mut reg);
        assert_eq!(reg.read(5), 0);
        reg.write(2, 18);
        reg.write(3, 33);
        inst_r_sltu(rd, rs1, rs2, &mut reg);
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
        inst_r_xor(rd, rs1, rs2, &mut reg);
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
        inst_r_srl(rd, rs1, rs2, &mut reg);
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
        reg.write(3, (-30i32) as u32);
        let rs1 = 2;
        let rs2 = 3;
        let rd = 5;
        inst_r_sra(rd, rs1, rs2, &mut reg);
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
        inst_r_or(rd, rs1, rs2, &mut reg);
        assert_eq!(reg.read(5), 11); // 1011
    }

    #[test]
    fn test_inst_r_and() {
        let mut reg = build_register_file();
        let rs1 = 2;
        let rs2 = 3;
        reg.write(2, 3); // 0011
        reg.write(3, 9); // 1001
        let rd = 5;
        inst_r_and(rd, rs1, rs2, &mut reg);
        assert_eq!(reg.read(5), 1);
    }

    // --- boundary tests ---

    #[test]
    fn test_inst_r_add_wraps_at_u32_max() {
        let mut reg = build_register_file();
        reg.write(2, u32::MAX);
        reg.write(3, 1);
        inst_r_add(5, 2, 3, &mut reg);
        assert_eq!(reg.read(5), 0);
    }

    #[test]
    fn test_inst_r_sub_wraps_below_zero() {
        let mut reg = build_register_file();
        reg.write(2, 0);
        reg.write(3, 1);
        inst_r_sub(5, 2, 3, &mut reg);
        assert_eq!(reg.read(5), u32::MAX);
    }

    #[test]
    fn test_inst_r_sll_at_max_shift_no_panic() {
        let mut reg = build_register_file();
        reg.write(2, 1);
        reg.write(3, 31);
        inst_r_sll(5, 2, 3, &mut reg);
        assert_eq!(reg.read(5), 0x8000_0000);
    }

    #[test]
    fn test_inst_r_slt_at_i32_extremes() {
        let mut reg = build_register_file();
        reg.write(2, i32::MIN as u32);
        reg.write(3, i32::MAX as u32);
        inst_r_slt(5, 2, 3, &mut reg);
        assert_eq!(reg.read(5), 1);
    }

    #[test]
    fn test_inst_r_sltu_at_u32_max() {
        let mut reg = build_register_file();
        reg.write(2, u32::MAX);
        reg.write(3, 0);
        inst_r_sltu(5, 2, 3, &mut reg);
        assert_eq!(reg.read(5), 0);
    }

    #[test]
    fn test_inst_r_xor_self_at_u32_max() {
        let mut reg = build_register_file();
        reg.write(2, u32::MAX);
        reg.write(3, u32::MAX);
        inst_r_xor(5, 2, 3, &mut reg);
        assert_eq!(reg.read(5), 0);
    }

    #[test]
    fn test_inst_r_srl_at_max_shift_no_panic() {
        let mut reg = build_register_file();
        // u32::MAX = 0xFFFF_FFFF = 0b1111_1111_1111_1111_1111_1111_1111_1111
        reg.write(2, u32::MAX);
        reg.write(3, 31);
        inst_r_srl(5, 2, 3, &mut reg);
        // logical shift right 31: only the original bit 31 survives, now at bit 0
        // 0b1111_...1111 >> 31 = 0b0000_...0001 = 0x1 = 1
        assert_eq!(reg.read(5), 1);
    }

    #[test]
    fn test_inst_r_sra_at_i32_min_max_shift() {
        let mut reg = build_register_file();
        // i32::MIN = 0x8000_0000 = 0b1000_0000_0000_0000_0000_0000_0000_0000 (sign bit set, rest zero)
        reg.write(2, i32::MIN as u32);
        reg.write(3, 31);
        inst_r_sra(5, 2, 3, &mut reg);
        // arithmetic shift right 31: the sign bit (1) is replicated into every
        // vacated position, so all 32 bits end up set
        // 0b1000_...0000 >>s 31 = 0b1111_...1111 = 0xFFFF_FFFF = -1 (as i32)
        assert_eq!(reg.read(5) as i32, -1);
    }

    #[test]
    fn test_inst_r_or_at_u32_max() {
        let mut reg = build_register_file();
        reg.write(2, 0);
        reg.write(3, u32::MAX);
        inst_r_or(5, 2, 3, &mut reg);
        assert_eq!(reg.read(5), u32::MAX);
    }

    #[test]
    fn test_inst_r_and_at_u32_max() {
        let mut reg = build_register_file();
        reg.write(2, u32::MAX);
        reg.write(3, u32::MAX);
        inst_r_and(5, 2, 3, &mut reg);
        assert_eq!(reg.read(5), u32::MAX);
    }

    #[test]
    fn test_inst_r_mulh_sign_extends_negative_rs1() {
        // -1 * 1 = -1, whose 64-bit two's-complement form is all 1 bits,
        // so the upper 32 bits should also be all 1s (u32::MAX). A rs1
        // that's zero-extended instead of sign-extended before the
        // multiply would treat 0xFFFFFFFF as +4294967295 instead of -1,
        // giving an upper half of 0 instead.
        let mut reg = build_register_file();
        reg.write(2, 0xFFFFFFFF); // rs1 = -1
        reg.write(3, 1);          // rs2 = 1
        inst_r_mulh(5, 2, 3, &mut reg);
        assert_eq!(reg.read(5), u32::MAX);
    }

    #[test]
    fn test_inst_r_mulsu_sign_extends_rs1() {
        // Same reasoning as mulh above, but for the mixed signed(rs1) *
        // unsigned(rs2) variant: -1 * 1 = -1, upper 32 bits = u32::MAX.
        let mut reg = build_register_file();
        reg.write(2, 0xFFFFFFFF); // rs1 = -1 (signed)
        reg.write(3, 1);          // rs2 = 1 (unsigned)
        inst_r_mulsu(5, 2, 3, &mut reg);
        assert_eq!(reg.read(5), u32::MAX);
    }
}