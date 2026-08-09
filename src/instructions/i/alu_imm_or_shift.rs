use crate::definitions::cpu_definition::RegisterFile;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::definitions::codes::ExecutionSignal;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use crate::utility::bit_operations::shake_to_signed;

#[derive(Debug, PartialEq)]
pub enum AluImmOp {
    Addi,
    Slti,
    Sltiu,
    Xori,
    Ori,
    Andi
}

#[derive(Debug, PartialEq)]
pub enum IShOp {
    Slli,
    Srli,
    Srai
}

pub fn parse_alu_imm_or_shift_inst(raw_word: InstructionWord) -> Result<Format, String> {
    let content = raw_word.0;
    let funct_three = mask_and_shift(content, masks::FUNCT_THREE);
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    match funct_three {
        (0b000 | 0b010 | 0b011 | 0b100 | 0b110 | 0b111) => parse_i_alum_imm(&content, &funct_three, &reg_dest, &reg_source_one),
        (0b001 | 0b101) => parse_i_shift(&content, &funct_three, &reg_dest, &reg_source_one),
        _ => Err(format!("Unrecognized funct three format"))
    }
}

pub fn parse_i_shift(content: &u32, funct_three: &u32, reg_dest: &u32, reg_source_one: &u32) -> Result<Format, String> {
    let shamt = mask_and_shift(*content, masks::I_TYPE_SHAMT);
    let funct_seven = mask_and_shift(*content, masks::FUNCT_SEVEN);
    let instruction_name = match (funct_seven, funct_three) {
        (0b0000000, 0b001) => Ok(IShOp::Slli),
        (0b0000000, 0b101) => Ok(IShOp::Srli),
        (0b0100000, 0b101) => Ok(IShOp::Srai),
        _ => Err(format!("undefined shift type type detected"))
    }?;
    Ok(Format::IShiftType {
        op: instruction_name,
        shamt: shamt as usize,
        rd: *reg_dest as usize,
        rs1: *reg_source_one as usize
    })
}

pub fn parse_i_alum_imm(content: &u32, funct_three: &u32, reg_dest: &u32, reg_source_one: &u32) -> Result<Format, String> {
    let imm_unsigned = mask_and_shift(*content, masks::I_TYPE_ALU_IMM);
    let imm_val = shake_to_signed(imm_unsigned, 12);
    let instruction_name = match funct_three {
        0b000 => Ok(AluImmOp::Addi),
        0b010 => Ok(AluImmOp::Slti),
        0b011 => Ok(AluImmOp::Sltiu),
        0b100 => Ok(AluImmOp::Xori),
        0b110 => Ok(AluImmOp::Ori),
        0b111 => Ok(AluImmOp::Andi),
        0b001 => Ok(AluImmOp::Xori),
        0b101 => Ok(AluImmOp::Ori),       
        _ => Err(format!("undefined alu imm type detected"))
    }?;
    Ok(Format::AluImmType {
        op: instruction_name,
        imm: imm_val,
        rd: *reg_dest as usize,
        rs1: *reg_source_one as usize
    })
}

pub fn execute_i_alu_imm_type(op: &AluImmOp, rd: usize, rs1: usize, imm: i32, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    Ok(ExecutionSignal::Continue)
}