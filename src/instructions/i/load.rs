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
        LoadOp::Lb => inst_i_lb(rd, rs1, imm, mem, register),
        LoadOp::Lh => inst_i_lh(rd, rs1, imm, mem, register),
        LoadOp::Lw => inst_i_lw(rd, rs1, imm, mem, register),
        LoadOp::Lbu => inst_i_lbu(rd, rs1, imm, mem, register),
        LoadOp::Lhu => inst_i_lhu(rd, rs1, imm, mem, register),
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_i_lb(rd: usize, rs1: usize, imm_i: i32, mem: &MemoryState, reg_file: &mut RegisterFile) {
    // sext = sign extended
    // rd <- sext(m8(rs1 + imm_i))
    let val = reg_file.read(rs1);
    let address = ((val as i32) + imm_i) as usize;
    let num = mem.storage[address];
    let sext_num = shake_to_signed(num.into(), 8);
    reg_file.write(rd, sext_num as u32);
}

pub fn inst_i_lh(rd: usize, rs1: usize, imm_i: i32, mem: &MemoryState, reg_file: &mut RegisterFile) {
    // rd <- sext(m16(rs1 + imm_i))
    let val = reg_file.read(rs1);
    let address = ((val as i32) + imm_i) as usize;
    let num = mem.read_bytes(address, 2);
    let sext_num = shake_to_signed(num, 16);
    reg_file.write(rd, sext_num as u32);
}

pub fn inst_i_lw(rd: usize, rs1: usize, imm_i: i32, mem: &MemoryState, reg_file: &mut RegisterFile) {
    // rd <- sext(m32(rs1 + imm_i))
    let val = reg_file.read(rs1);
    let address = ((val as i32) + imm_i) as usize;
    let num = mem.read_bytes(address, 4);
    let sext_num = shake_to_signed(num, 32);
    reg_file.write(rd, sext_num as u32);
}

pub fn inst_i_lbu(rd: usize, rs1: usize, imm_i: i32, mem: &MemoryState, reg_file: &mut RegisterFile) {
    // zero = zero extended
    // rd <- zext(m8(rs1 + imm_i))
    let val = reg_file.read(rs1);
    let address = ((val as i32) + imm_i) as usize;
    let num = mem.storage[address];
    let zext_num = num as u32;
    reg_file.write(rd, zext_num);
}

pub fn inst_i_lhu(rd: usize, rs1: usize, imm_i: i32, mem: &MemoryState, reg_file: &mut RegisterFile) {
    // rd <- zext(m16(rs1 + imm_i))
    let val = reg_file.read(rs1);
    let address = ((val as i32) + imm_i) as usize;
    let num = mem.read_bytes(address, 2);
    reg_file.write(rd, num);
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
        let mut reg_file = build_register_file();
        reg_file.write(3, 4);
        let mut mem = build_memory_state();
        mem.storage[10] = 0b1000_0001;
        inst_i_lb(rd, rs1, imm_i, mem, reg_file); 
        assert_eq!(reg_file.read(1), -127);
    }

    #[test]
    fn test_inst_i_lh() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let mut reg_file = build_register_file();
        reg_file.write(3, 4);
        let mut mem = build_memory_state();
        mem.storage[10] = 0b0000_0001;
        mem.storage[11] = 0b1000_0000;
        inst_i_lh(rd, rs1, imm_i, mem, reg_file);
        assert_eq!(reg_file.read(1), -32767);
    }

    #[test]
    fn test_inst_i_lw() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let mut reg_file = build_register_file();
        reg_file.write(3, 4);
        let mut mem = build_memory_state();
        mem.storage[10] = 0b0000_0001;
        mem.storage[11] = 0b0000_0000;
        mem.storage[12] = 0b0000_0000;
        mem.storage[13] = 0b1000_0000;
        inst_i_lw(rd, rs1, imm_i, mem, reg_file);
        assert_eq!(reg_file.read(1), -2147483647);
    }

    #[test]
    fn test_inst_i_lbu() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let mut reg_file = build_register_file();
        reg_file.write(3, 4);
        let mut mem = build_memory_state();
        mem.storage[10] = 0b1000_0001;
        inst_i_lbu(rd, rs1, imm_i, mem, reg_file); 
        assert_eq!(reg_file.read(1), 129);
    }

    #[test]
    fn test_inst_i_lhu() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let mut reg_file = build_register_file();
        reg_file.write(3, 4);
        let mut mem = build_memory_state();
        mem.storage[10] = 0b0000_0001;
        mem.storage[11] = 0b1000_0000;
        inst_i_lhu(rd, rs1, imm_i, mem, reg_file);
        assert_eq!(reg_file.read(1), 32769);
    }
}