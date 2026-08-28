use crate::definitions::cpu::cpu_definition::{CPUMode, RegisterFile};
use crate::definitions::cpu::bus::BUSState;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::definitions::codes::ExecutionSignal;
use crate::definitions::cpu::csr::CSRState;
use crate::utility::bit_operations::{mask_and_shift, shake_to_signed};
use crate::definitions::masks;
use crate::definitions::trap_cause::TrapCause;

#[derive(Debug, PartialEq)]
pub enum LoadOp {
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu
}

pub fn parse_load_inst(raw_word: InstructionWord) -> Result<Format, TrapCause> {
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
        _ => Err(TrapCause::IllegalInstruction { instruction: Some(content) })
    }?;
    
    Ok(Format::LoadType {
        op: instruction_name,
        imm: imm_val,
        rd: reg_dest as usize,
        rs1: reg_source_one as usize
    })
}


pub fn execute_i_load_type(op: &LoadOp,
                           rd: usize,
                           rs1: usize, 
                           imm: i32,
                           register: &mut RegisterFile,
                           bus: &mut BUSState,
                           state: &CSRState,
                           mode: CPUMode) -> Result<ExecutionSignal, TrapCause> {
    match op {
        LoadOp::Lb => inst_i_lb(rd, rs1, imm, bus, register, state, mode)?,
        LoadOp::Lh => inst_i_lh(rd, rs1, imm, bus, register, state, mode)?,
        LoadOp::Lw => inst_i_lw(rd, rs1, imm, bus, register, state, mode)?,
        LoadOp::Lbu => inst_i_lbu(rd, rs1, imm, bus, register, state, mode)?,
        LoadOp::Lhu => inst_i_lhu(rd, rs1, imm, bus, register, state, mode)?,
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_i_lb(rd: usize, rs1: usize, imm_i: i32, bus: &mut BUSState, reg_file: &mut RegisterFile, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // sext = sign extended
    // rd <- sext(m8(rs1 + imm_i))
    let val = reg_file.read(rs1);
    let address = val.wrapping_add(imm_i as u32) as usize;
    let num = bus.guest_load(address as u32, 1, state, mode)?;
    let sext_num = shake_to_signed(num, 8);
    reg_file.write(rd, sext_num as u32);
    Ok(())
}

pub fn inst_i_lh(rd: usize, rs1: usize, imm_i: i32, bus: &mut BUSState, reg_file: &mut RegisterFile, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // rd <- sext(m16(rs1 + imm_i))
    let val = reg_file.read(rs1);
    let address = val.wrapping_add(imm_i as u32) as usize;
    let num = bus.guest_load(address as u32, 2, state, mode)?;
    let sext_num = shake_to_signed(num, 16);
    reg_file.write(rd, sext_num as u32);
    Ok(())
}

pub fn inst_i_lw(rd: usize, rs1: usize, imm_i: i32, bus: &mut BUSState, reg_file: &mut RegisterFile, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // rd <- sext(m32(rs1 + imm_i))
    let val = reg_file.read(rs1);
    let address = val.wrapping_add(imm_i as u32) as usize;
    let num = bus.guest_load(address as u32, 4, state, mode)?;
    let sext_num = shake_to_signed(num, 32);
    reg_file.write(rd, sext_num as u32);
    Ok(())
}

pub fn inst_i_lbu(rd: usize, rs1: usize, imm_i: i32, bus: &mut BUSState, reg_file: &mut RegisterFile, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // zero = zero extended
    // rd <- zext(m8(rs1 + imm_i))
    let val = reg_file.read(rs1);
    let address = val.wrapping_add(imm_i as u32) as usize;
    let num = bus.guest_load(address as u32, 1, state, mode)?;
    reg_file.write(rd, num);
    Ok(())
}

pub fn inst_i_lhu(rd: usize, rs1: usize, imm_i: i32, bus: &mut BUSState, reg_file: &mut RegisterFile, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // rd <- zext(m16(rs1 + imm_i))
    let val = reg_file.read(rs1);
    let address = val.wrapping_add(imm_i as u32) as usize;
    let num = bus.guest_load(address as u32, 2, state, mode)?;
    reg_file.write(rd, num);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::cpu::cpu_definition::build_register_file;
    use crate::definitions::cpu::bus::{build_bus_state, BASE_ADDRESS};
    use crate::definitions::cpu::csr::build_csr_state;

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
        reg_file.write(3, BASE_ADDRESS + 4);
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        bus.ram.storage[10] = 0b1000_0001;
        inst_i_lb(rd, rs1, imm_i, &mut bus, &mut reg_file, &csr, CPUMode::M).unwrap();
        assert_eq!(reg_file.read(1) as i32, -127);
    }

    #[test]
    fn test_inst_i_lh() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let mut reg_file = build_register_file();
        reg_file.write(3, BASE_ADDRESS + 4);
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        bus.ram.storage[10] = 0b0000_0001;
        bus.ram.storage[11] = 0b1000_0000;
        inst_i_lh(rd, rs1, imm_i, &mut bus, &mut reg_file, &csr, CPUMode::M).unwrap();
        assert_eq!(reg_file.read(1) as i32, -32767);
    }

    #[test]
    fn test_inst_i_lw() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 8;
        let mut reg_file = build_register_file();
        reg_file.write(3, BASE_ADDRESS + 4);
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        bus.ram.storage[12] = 0b0000_0001;
        bus.ram.storage[13] = 0b0000_0000;
        bus.ram.storage[14] = 0b0000_0000;
        bus.ram.storage[15] = 0b1000_0000;
        let outcome = inst_i_lw(rd, rs1, imm_i, &mut bus, &mut reg_file, &csr, CPUMode::M).unwrap();
        assert_eq!(reg_file.read(1) as i32, -2147483647);
    }

    #[test]
    fn test_inst_i_lw_misaligned_reads_correct_value() {
        // address = BASE_ADDRESS + 4 + 9 = BASE_ADDRESS + 13 
        //  not a multiple of 4, and straddles the word boundary between
        // storage[12..16] and storage[16..20]. Misaligned data accesses
        // are implementation-defined per the ISA (unlike misaligned
        // instruction fetches, which are always rejected), and this
        // emulator chooses to support them directly,  this should
        // succeed and read the correct value.
        let rd = 1;
        let rs1 = 3;
        let imm_i = 9;
        let mut reg_file = build_register_file();
        reg_file.write(3, BASE_ADDRESS + 4);
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        bus.ram.storage[13] = 0b0000_0001;
        bus.ram.storage[14] = 0b0000_0000;
        bus.ram.storage[15] = 0b0000_0000;
        bus.ram.storage[16] = 0b1000_0000;
        let outcome = inst_i_lw(rd, rs1, imm_i, &mut bus, &mut reg_file, &csr, CPUMode::M);
        assert_eq!(outcome, Ok(()));
        assert_eq!(reg_file.read(1) as i32, -2147483647);
    }

    #[test]
    fn test_inst_i_lbu() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let mut reg_file = build_register_file();
        reg_file.write(3, BASE_ADDRESS + 4);
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        bus.ram.storage[10] = 0b1000_0001;
        inst_i_lbu(rd, rs1, imm_i, &mut bus, &mut reg_file, &csr, CPUMode::M).unwrap();
        assert_eq!(reg_file.read(1), 129);
    }

    #[test]
    fn test_inst_i_lhu() {
        let rd = 1;
        let rs1 = 3;
        let imm_i = 6;
        let mut reg_file = build_register_file();
        reg_file.write(3, BASE_ADDRESS + 4);
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        bus.ram.storage[10] = 0b0000_0001;
        bus.ram.storage[11] = 0b1000_0000;
        inst_i_lhu(rd, rs1, imm_i, &mut bus, &mut reg_file, &csr, CPUMode::M).unwrap();
        assert_eq!(reg_file.read(1), 32769);
    }

    // --- boundary tests ---

    #[test]
    fn test_inst_i_lb_wraps_and_out_of_bounds_returns_err() {
        let rd = 1;
        let rs1 = 3;
        let mut reg_file = build_register_file();
        reg_file.write(3, u32::MAX);
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        let imm_i = bus.ram.storage.len() as i32 + 1;
        let outcome = inst_i_lb(rd, rs1, imm_i, &mut bus, &mut reg_file, &csr, CPUMode::M);
        assert!(outcome.is_err());
    }

    #[test]
    fn test_inst_i_lh_wraps_and_out_of_bounds_returns_err() {
        let rd = 1;
        let rs1 = 3;
        let mut reg_file = build_register_file();
        reg_file.write(3, u32::MAX);
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        let imm_i = bus.ram.storage.len() as i32 + 1;
        let outcome = inst_i_lh(rd, rs1, imm_i, &mut bus, &mut reg_file, &csr, CPUMode::M);
        assert!(outcome.is_err());
    }

    #[test]
    fn test_inst_i_lw_wraps_and_out_of_bounds_returns_err() {
        let rd = 1;
        let rs1 = 3;
        let mut reg_file = build_register_file();
        reg_file.write(3, u32::MAX);
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        let imm_i = bus.ram.storage.len() as i32 + 1;
        let outcome = inst_i_lw(rd, rs1, imm_i, &mut bus, &mut reg_file, &csr, CPUMode::M);
        assert!(outcome.is_err());
    }

    #[test]
    fn test_inst_i_lbu_wraps_and_out_of_bounds_returns_err() {
        let rd = 1;
        let rs1 = 3;
        let mut reg_file = build_register_file();
        reg_file.write(3, u32::MAX);
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        let imm_i = bus.ram.storage.len() as i32 + 1;
        let outcome = inst_i_lbu(rd, rs1, imm_i, &mut bus, &mut reg_file, &csr, CPUMode::M);
        assert!(outcome.is_err());
    }

    #[test]
    fn test_inst_i_lhu_wraps_and_out_of_bounds_returns_err() {
        let rd = 1;
        let rs1 = 3;
        let mut reg_file = build_register_file();
        reg_file.write(3, u32::MAX);
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        let imm_i = bus.ram.storage.len() as i32 + 1;
        let outcome = inst_i_lhu(rd, rs1, imm_i, &mut bus, &mut reg_file, &csr, CPUMode::M);
        assert!(outcome.is_err());
    }
}