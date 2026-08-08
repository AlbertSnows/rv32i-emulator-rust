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
    Sb,
    Sh,
    Sw
}

pub fn parse_s_inst(raw_word: InstructionWord) -> Result<Format, String> {
    let content = raw_word.0;
    let funct_three = mask_and_shift(content, masks::FUNCT_THREE);
    let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    let reg_source_two = mask_and_shift(content, masks::REG_SOURCE_TWO);
    let imm_four_to_zero = mask_and_shift(content, masks::S_TYPE_IMM_FIRST);
    let imm_eleven_to_five = mask_and_shift(content, masks::S_TYPE_IMM_SECOND);
    let imm_combined_unsigned = merge_bits(&[
        (imm_four_to_zero, 0),
        (imm_eleven_to_five, 5)        
    ]);
    let imm_val = shake_to_signed(imm_combined_unsigned, 12);
    let instruction_name = match funct_three {
        0b000 => Ok(SOp::Sb),
        0b001 => Ok(SOp::Sh),
        0b010 => Ok(SOp::Sw),
        _ => Err(format!("undefined S type"))
    }?;

    Ok(Format::SType { 
        op: instruction_name,
        imm: imm_val,
        rs1: reg_source_one as usize,
        rs2: reg_source_two as usize
    })
}

pub fn execute_s_type(op: &SOp, imm: i32, rs1: usize, rs2: usize, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    Ok(ExecutionSignal::Continue)
}