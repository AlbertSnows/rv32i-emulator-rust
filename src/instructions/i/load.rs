use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::cpu_definition::MemoryState;
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


pub fn execute_i_load_type(op: &LoadOp, rd: usize, rs1: usize, imm: i32, register: &mut RegisterFile, mem: &MemoryState) -> Result<ExecutionSignal, String> {
    match op {
        LoadOp::Lb => inst_i_lb(),
        LoadOp::Lh => inst_i_lh(),
        LoadOp::Lw => inst_i_lw(),
        LoadOp::Lbu => inst_i_lbu(),
        LoadOp::Lhu => inst_i_lhu(),
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_i_lb() {
    // sext = sign extended
    // rd <- sext(m8(rs1 + imm_i))
    let num = rs1 + imm_i;
    let sext_num = sext(num);
    register.write(rd, sext_num);
}

pub fn inst_i_lh() {
    // rd <- sext(m16(rs1 + imm_i))
    let num = rs1 + imm_i;
    let sext_num = sext(num);
    register.write(rd, sext_num);
}

pub fn inst_i_lw() {
    // rd <- sext(m32(rs1 + imm_i))
    let num = rs1 + imm_i;
    let sext_num = sext(num);
    register.write(rd, sext_num);
}

pub fn inst_i_lbu() {
    // zero = zero extended
    // rd <- zext(m8(rs1 + imm_i))
    let num = rs1 + imm_i;
    let zext_num = zext(num);
    register.write(rd, zext_num);
}

pub fn inst_i_lhu() {
    // rd <- zext(m16(rs1 + imm_i))
    let num = rs1 + imm_i;
    let zext_num = zext(num);
    register.write(rd, zext_num);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_load_inst() {
        // lb x1, 4(x2) -- opcode = 0000011 (LOAD), funct3 = 000 (lb), rd = 1, rs1 = 2, imm = 4
        let raw_word = InstructionWord(0x00410083);
        let result = parse_load_inst(raw_word);
        assert_eq!(result, Ok(Format::LoadType { op: LoadOp::Lb, rd: 1, rs1: 2, imm: 4 }));
    }

    #[test]
    fn test_inst_i_lb() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let reg_file = build_register_file();
        execute_i_jalr_type(); // sext(0b1001) = ?
        assert_eq!(reg_file.read(1), ?);
    }

    #[test]
    fn test_inst_i_lh() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let reg_file = build_register_file();
        execute_i_jalr_type(); // sext(0b1001) = ?
        assert_eq!(reg_file.read(1), ?);
    }

    #[test]
    fn test_inst_i_lw() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let reg_file = build_register_file();
        execute_i_jalr_type(); // sext(0b1001) = ?
        assert_eq!(reg_file.read(1), ?);
    }

    #[test]
    fn test_inst_i_lbu() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let reg_file = build_register_file();
        execute_i_jalr_type(); // zext(0b1001) = ?
        assert_eq!(reg_file.read(1), ?);
    }

    #[test]
    fn test_inst_i_lhu() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let reg_file = build_register_file();
        execute_i_jalr_type(); // zext(0b1001) = ?
        assert_eq!(reg_file.read(1), ?);
    }
}