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
use crate::definitions::cpu::cpu_definition::RegisterFile;
use crate::definitions::cpu::memory::MemoryState;
use crate::definitions::codes::ExecutionSignal;
use crate::utility::bit_operations::{mask_and_shift, merge_bits, shake_to_signed};
use crate::definitions::masks;
use crate::definitions::trap_cause::TrapCause;

#[derive(Debug, PartialEq)]
pub enum SOp {
    Sb,
    Sh,
    Sw
}

pub fn parse_s_inst(raw_word: InstructionWord) -> Result<Format, TrapCause> {
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
        _ => Err(TrapCause::IllegalInstruction { instruction: Some(content) })
    }?;

    Ok(Format::SType { 
        op: instruction_name,
        imm: imm_val,
        rs1: reg_source_one as usize,
        rs2: reg_source_two as usize
    })
}

pub fn execute_s_type(op: &SOp, imm: i32, rs1: usize, rs2: usize, register: &RegisterFile, mem: &mut MemoryState) -> Result<ExecutionSignal, TrapCause> {
    match op {
        SOp::Sb => inst_s_sb(rs1, rs2, imm, mem, register)?,
        SOp::Sh => inst_s_sh(rs1, rs2, imm, mem, register)?,
        SOp::Sw => inst_s_sw(rs1, rs2, imm, mem, register)?,
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_s_sb(rs1: usize, rs2: usize, imm: i32, mem: &mut MemoryState, reg_file: &RegisterFile) -> Result<(), TrapCause> {
    // m8(rs1+imm_s) ← rs2[7:0]
    let val = reg_file.read(rs1);
    let mem_address = val.wrapping_add(imm as u32);
    mem.write_bytes(mem_address as usize, &(rs2 as u8).to_le_bytes())
}

pub fn inst_s_sh(rs1: usize, rs2: usize, imm: i32, mem: &mut MemoryState, reg_file: &RegisterFile) -> Result<(), TrapCause> {
    // m16(rs1+imm_s) <- rs2[15:0]
    let val = reg_file.read(rs1);
    let mem_address = val.wrapping_add(imm as u32);
    if mem_address % 2 != 0 {
        return Err(TrapCause::StoreAddressMisaligned { address: mem_address as usize });
    }
    mem.write_bytes(mem_address as usize, &(rs2 as u16).to_le_bytes())
}

pub fn inst_s_sw(rs1: usize, rs2: usize, imm: i32, mem: &mut MemoryState, reg_file: &RegisterFile) -> Result<(), TrapCause> {
    // m32(rs1+imm_s) <- rs2[31:0]
    let val = reg_file.read(rs1);
    let mem_address = val.wrapping_add(imm as u32);
    if mem_address % 4 != 0 {
        return Err(TrapCause::StoreAddressMisaligned { address: mem_address as usize });
    }
    mem.write_bytes(mem_address as usize, &(rs2 as u32).to_le_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::cpu::cpu_definition::{build_register_file, build_pc_state};
    use crate::definitions::cpu::memory::build_memory_state;

    #[test]
    fn test_parse_s_inst() {
        // sw x2, 4(x1)
        // opcode = 0100011 (S), funct3 = 010 (sw), rs1 = 1, rs2 = 2, imm = 4
        let raw_word = InstructionWord(0x0020A223);
        let result = parse_s_inst(raw_word);
        assert_eq!(result, Ok(Format::SType { op: SOp::Sw, imm: 4, rs1: 1, rs2: 2 }));
    }

    #[test]
    fn test_inst_s_sb() {
        let mut mem = build_memory_state();
        let mut reg_file = build_register_file();
        let rs1 = 1;
        reg_file.write(1, 3);
        let rs2 = 0b0101_1010_0101_1010;
        let imm = 7;
        inst_s_sb(rs1, rs2, imm, &mut mem, &reg_file).unwrap();
        assert_eq!(mem.storage[3 + 7], 0b0101_1010);
    }

    #[test]
    fn test_inst_s_sh() {
        let mut mem = build_memory_state();
        let mut reg_file = build_register_file();
        let rs1 = 1;
        reg_file.write(1, 3);
        let rs2 = 0b1111_0000_1010_0101;
        let imm = 7;
        inst_s_sh(rs1, rs2, imm, &mut mem, &reg_file).unwrap();
        assert_eq!(mem.storage[3 + 7], 0b1010_0101);
        assert_eq!(mem.storage[3 + 7 + 1], 0b1111_0000);
   }

    #[test]
    fn test_inst_s_sw() {
        let mut mem = build_memory_state();
        let mut reg_file = build_register_file();
        let rs1 = 1;
        reg_file.write(1, 3);
        // 12, 34, 56, 78
        // 18, 52, 86, 120
        let rs2 = 0x12345678;
        let imm = 9;
        inst_s_sw(rs1, rs2, imm, &mut mem, &reg_file).unwrap();
        assert_eq!(mem.storage[12], 0x78);
        assert_eq!(mem.storage[13], 0x56);
        assert_eq!(mem.storage[14], 0x34);
        assert_eq!(mem.storage[15], 0x12);
    }

    #[test]
    fn test_inst_s_sw_bad_addr() {
        let mut mem = build_memory_state();
        let mut reg_file = build_register_file();
        let rs1 = 1;
        reg_file.write(1, 3);
        // 12, 34, 56, 78
        // 18, 52, 86, 120
        let rs2 = 0x12345678;
        let imm = 7;
        let outcome = inst_s_sw(rs1, rs2, imm, &mut mem, &reg_file);
        assert_eq!(outcome, Err(TrapCause::StoreAddressMisaligned { address: 10 }));
    }

    // --- boundary tests ---

    #[test]
    fn test_inst_s_sb_wraps_and_out_of_bounds_returns_err() {
        let mut mem = build_memory_state();
        let mut reg_file = build_register_file();
        reg_file.write(1, u32::MAX);
        let rs1 = 1;
        let rs2 = 0xAA; // 0b1010_1010
        let imm = mem.storage.len() as i32 + 1;
        let outcome = inst_s_sb(rs1, rs2, imm, &mut mem, &reg_file);
        assert!(outcome.is_err());
    }

    #[test]
    fn test_inst_s_sh_wraps_and_out_of_bounds_returns_err() {
        let mut mem = build_memory_state();
        let mut reg_file = build_register_file();
        reg_file.write(1, u32::MAX);
        let rs1 = 1;
        let rs2 = 0xAABB; // 0b1010_1010_1011_1011
        let imm = mem.storage.len() as i32 + 1;
        let outcome = inst_s_sh(rs1, rs2, imm, &mut mem, &reg_file);
        assert!(outcome.is_err());
    }

    #[test]
    fn test_inst_s_sw_wraps_and_out_of_bounds_returns_err() {
        let mut mem = build_memory_state();
        let mut reg_file = build_register_file();
        reg_file.write(1, u32::MAX);
        let rs1 = 1;
        let rs2 = 0x12345678; // 0b0001_0010_0011_0100_0101_0110_0111_1000
        let imm = mem.storage.len() as i32 + 1;
        let outcome = inst_s_sw(rs1, rs2, imm, &mut mem, &reg_file);
        assert!(outcome.is_err());
    }
}