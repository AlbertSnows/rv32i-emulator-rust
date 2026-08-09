use crate::definitions::cpu_definition::RegisterFile;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::definitions::codes::ExecutionSignal;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use crate::utility::bit_operations::shake_to_signed;

#[derive(Debug, PartialEq)]
pub enum LoadOp {
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu
}

pub fn parse_load_inst(raw_word: InstructionWord) -> Result<Format, String> {
    let content = raw_word.0;
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    let imm_unsigned = mask_and_shift(content, masks::I_TYPE_LOAD);
    let imm_val = shake_to_signed(imm_unsigned, 12);
    let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    let funct_three = mask_and_shift(content, masks::FUNCT_THREE);
    let instruction_name = match funct_three {
        0b000 => Ok(LoadOp::Lb),
        0b001 => Ok(LoadOp::Lh),
        0b010 => Ok(LoadOp::Lw),
        0b100 => Ok(LoadOp::Lbu),
        0b101 => Ok(LoadOp::Lhu),
        _ => Err(format!("undefined alu imm type detected"))
    }?;
    
    Ok(Format::LoadType {
        op: instruction_name,
        imm: imm_val,
        rd: reg_dest as usize,
        rs1: reg_source_one as usize
    })
}


pub fn execute_i_load_type(op: &LoadOp, rd: usize, rs1: usize, imm: i32, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    Ok(ExecutionSignal::Continue)
}