// A-type
//
// 31     27 26 25 24    20 19    15 14    12 11      7 6      0
// | funct5 |aq|rl|  rs2   |  rs1   | funct3 |   rd    | opcode |
// |   5    | 1| 1|   5    |   5    |   3    |    5    |   7    |

use crate::cpu::definitions::codes::ExecutionSignal;
use crate::cpu::definitions::cpu::bus::BUSState;
use crate::cpu::definitions::cpu::cpu_definition::{CPUMode, RegisterFile, build_register_file};
use crate::cpu::definitions::cpu::csr::CSRState;
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::cpu::definitions::masks;
use crate::cpu::fetcher::InstructionWord;
use crate::cpu::instructions::Format;
use crate::utility::bit_operations::mask_and_shift;
use crate::utility::types::ByteType;
use std::cmp::{max, min};

#[derive(Debug, PartialEq)]
pub enum AOp {
    Lr,
    Sc,
    Amoswap,
    Amoadd,
    Amoxor,
    Amoand,
    Amoor,
    Amomin,
    Amomax,
    Amominu,
    Amomaxu
}

pub fn parse_a_inst(raw_word: InstructionWord) -> Result<Format, TrapCause> {
    let content = raw_word.0;
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    let funct_three = mask_and_shift(content, masks::FUNCT_THREE);
    let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    let reg_source_two = mask_and_shift(content, masks::REG_SOURCE_TWO);
    let release = mask_and_shift(content, masks::RELEASE);
    let acquire = mask_and_shift(content, masks::ACQUIRE);
    let funct_five = mask_and_shift(content, masks::FUNCT_FIVE);
    let instruction_name = match (funct_five, funct_three) {
        (0b00010, 0b010) => Ok(AOp::Lr),
        (0b00011, 0b010) => Ok(AOp::Sc),
        (0b00001, 0b010) => Ok(AOp::Amoswap),
        (0b00000, 0b010) => Ok(AOp::Amoadd),
        (0b00100, 0b010) => Ok(AOp::Amoxor),
        (0b01100, 0b010) => Ok(AOp::Amoand),
        (0b01000, 0b010) => Ok(AOp::Amoor),
        (0b10000, 0b010) => Ok(AOp::Amomin),
        (0b10100, 0b010) => Ok(AOp::Amomax),
        (0b11000, 0b010) => Ok(AOp::Amominu),
        (0b11100, 0b010) => Ok(AOp::Amomaxu),
        _ => Err(TrapCause::IllegalInstruction { instruction: Some(content) })
    }?;
    Ok(Format::AType {
        op: instruction_name,
        rd: reg_dest as usize,
        rs1: reg_source_one as usize,
        rs2: reg_source_two as usize,
        rl: release as usize,
        aq: acquire as usize
    })
}

pub fn execute_a_type(op: &AOp,
                      rd: usize,
                      rs1: usize,
                      rs2: usize,
                      rl: usize,
                      aq: usize,
                      register: &mut RegisterFile,
                      bus: &mut BUSState,
                      reservation: &mut Option<u32>,
                      state: &CSRState,
                      mode: CPUMode) -> Result<ExecutionSignal, TrapCause> {
    match op {
        AOp::Lr => inst_a_lr(rd, rs1, rs2, register, bus, reservation, state, mode)?,
        AOp::Sc => inst_a_sc(rd, rs1, rs2, register, bus, reservation, state, mode)?,
        AOp::Amoswap => inst_a_amoswap(rd, rs1, rs2, register, bus, state, mode)?,
        AOp::Amoadd => inst_a_amoadd(rd, rs1, rs2, register, bus, state, mode)?,
        AOp::Amoxor => inst_a_amoxor(rd, rs1, rs2, register, bus, state, mode)?,
        AOp::Amoand => inst_a_amoand(rd, rs1, rs2, register, bus, state, mode)?,
        AOp::Amoor => inst_a_amoor(rd, rs1, rs2, register, bus, state, mode)?,
        AOp::Amomin => inst_a_amomin(rd, rs1, rs2, register, bus, state, mode)?,
        AOp::Amomax => inst_a_amomax(rd, rs1, rs2, register, bus, state, mode)?,
        AOp::Amominu => inst_a_amominu(rd, rs1, rs2, register, bus, state, mode)?,
        AOp::Amomaxu => inst_a_amomaxu(rd, rs1, rs2, register, bus, state, mode)?,
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_a_lr(rd: usize,
                 rs1: usize,
                 _rs2: usize,
                 reg_file: &mut RegisterFile,
                 bus: &mut BUSState,
                 reservation: &mut Option<u32>,
                 state: &CSRState,
                 mode: CPUMode) -> Result<(), TrapCause> {
    // rd <- mem[rs1] (word). rs2 is 00000 (or should be)
    // register a reservation on rs1's address for a following SC.W.
    // Sign-extension is a no-op on RV32 (rd is already the full 32 bits).
    let mem_addr = reg_file.read(rs1);
    let mem_val = bus.guest_load(mem_addr, ByteType::Word.as_num(), state, mode)?;
    reg_file.write(rd, mem_val);
    *reservation = Some(mem_addr);
    Ok(())
}

pub fn inst_a_sc(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile, bus: &mut BUSState, reservation: &mut Option<u32>, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // If `reservation` is Some(addr) and addr matches rs1's address:
    //
    // write rs2's value to mem[rs1], write 0 to rd (success).
    //
    // Otherwise:
    //
    // write nothing to memory, write 1 to rd (the
    // spec reserves 1 for "unspecified failure"; portable software only
    // ever checks non-zero, so any nonzero value would do).
    //
    // Either way, clear `reservation` to None before returning.
    // executing SC.W invalidates any held reservation regardless of
    // whether it succeeds or fails.
    let rs1_addr = reg_file.read(rs1);
    let rs2_val = reg_file.read(rs2);
    let addresses_match = *reservation == Some(rs1_addr);
    if (addresses_match) {
        bus.guest_write(rs1_addr, &rs2_val.to_le_bytes(), state, mode)?;
        reg_file.write(rd, 0);
    } else {
        reg_file.write(rd, 1);
    }
    *reservation = None;
    Ok(())
}

fn amo_write_back(rd: usize, rs1: usize, new_val: u32, reg_file: &mut RegisterFile, bus: &mut BUSState, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    let addr = reg_file.read(rs1);
    let original = bus.guest_load(addr, ByteType::Word.as_num(), state, mode)?;
    reg_file.write(rd, original);
    bus.guest_write(addr, &new_val.to_le_bytes(), state, mode)?;
    Ok(())
}


pub fn inst_a_amoswap(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile, bus: &mut BUSState, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // original <- mem[rs1]; rd <- original; mem[rs1] <- rs2
    let rs2_val = reg_file.read(rs2);
    amo_write_back(rd, rs1, rs2_val, reg_file, bus, state, mode)?;
    Ok(())
}

pub fn inst_a_amoadd(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile, bus: &mut BUSState, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // original <- mem[rs1]; rd <- original; mem[rs1] <- original.wrapping_add(rs2)
    let rs1_addr = reg_file.read(rs1);
    let original = bus.guest_load(rs1_addr, ByteType::Word.as_num(), state, mode)?;
    let rs2_addr = reg_file.read(rs2);
    let rs2_val = original.wrapping_add(rs2_addr);
    amo_write_back(rd, rs1, rs2_val, reg_file, bus, state, mode)?;
    Ok(())
}

pub fn inst_a_amoxor(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile, bus: &mut BUSState, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // original <- mem[rs1]; rd <- original; mem[rs1] <- original ^ rs2
    let rs1_addr = reg_file.read(rs1);
    let original = bus.guest_load(rs1_addr, ByteType::Word.as_num(), state, mode)?;
    let rs2_addr = reg_file.read(rs2);
    let rs2_val = original ^ rs2_addr;
    amo_write_back(rd, rs1, rs2_val, reg_file, bus, state, mode)?;
    Ok(())
}

pub fn inst_a_amoand(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile, bus: &mut BUSState, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // original <- mem[rs1]; rd <- original; mem[rs1] <- original & rs2
    let rs1_addr = reg_file.read(rs1);
    let original = bus.guest_load(rs1_addr, ByteType::Word.as_num(), state, mode)?;
    let rs2_addr = reg_file.read(rs2);
    let rs2_val = original & rs2_addr;
    amo_write_back(rd, rs1, rs2_val, reg_file, bus, state, mode)?;
    Ok(())
}

pub fn inst_a_amoor(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile, bus: &mut BUSState, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // original <- mem[rs1]; rd <- original; mem[rs1] <- original | rs2
    let rs1_addr = reg_file.read(rs1);
    let original = bus.guest_load(rs1_addr, ByteType::Word.as_num(), state, mode)?;
    let rs2_addr = reg_file.read(rs2);
    let rs2_val = original | rs2_addr;
    amo_write_back(rd, rs1, rs2_val, reg_file, bus, state, mode)?;
    Ok(())
}

pub fn inst_a_amomin(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile, bus: &mut BUSState, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // original <- mem[rs1]; rd <- original;
    // mem[rs1] <- signed min(original, rs2) -- same signed treatment as SLT/DIV.
    let rs1_addr = reg_file.read(rs1);
    let rs2_addr = reg_file.read(rs2) as i32;
    let original = bus.guest_load(rs1_addr, ByteType::Word.as_num(), state, mode)? as i32;
    reg_file.write(rd, original as u32);
    let min_val = min(original, rs2_addr);
    bus.guest_write(rs1_addr, &min_val.to_le_bytes(), state, mode)?;
    Ok(())
}

pub fn inst_a_amomax(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile, bus: &mut BUSState, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // original <- mem[rs1]; rd <- original;
    // mem[rs1] <- signed max(original, rs2)
    let rs1_addr = reg_file.read(rs1);
    let rs2_addr = reg_file.read(rs2) as i32;
    let original = bus.guest_load(rs1_addr, ByteType::Word.as_num(), state, mode)? as i32;
    reg_file.write(rd, original as u32);
    let max_val = max(original, rs2_addr) as i32;
    bus.guest_write(rs1_addr, &max_val.to_le_bytes(), state, mode)?;
    Ok(())
}

pub fn inst_a_amominu(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile, bus: &mut BUSState, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // original <- mem[rs1]; rd <- original;
    // mem[rs1] <- unsigned min(original, rs2) -- same unsigned treatment as SLTU/DIVU.
    let rs1_addr = reg_file.read(rs1);
    let rs2_addr = reg_file.read(rs2);
    let original = bus.guest_load(rs1_addr, ByteType::Word.as_num(), state, mode)?;
    reg_file.write(rd, original);
    let min_val = min(original, rs2_addr);
    bus.guest_write(rs1_addr, &min_val.to_le_bytes(), state, mode)?;
    Ok(())
}

pub fn inst_a_amomaxu(rd: usize, rs1: usize, rs2: usize, reg_file: &mut RegisterFile, bus: &mut BUSState, state: &CSRState, mode: CPUMode) -> Result<(), TrapCause> {
    // original <- mem[rs1]; rd <- original;
    // mem[rs1] <- unsigned max(original, rs2)
    let rs1_addr = reg_file.read(rs1);
    let rs2_addr = reg_file.read(rs2);
    let original = bus.guest_load(rs1_addr, ByteType::Word.as_num(), state, mode)?;
    reg_file.write(rd, original);
    let max_val = max(original, rs2_addr);
    bus.guest_write(rs1_addr, &max_val.to_le_bytes(), state, mode)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::definitions::cpu::bus::{BASE_ADDRESS, build_bus_state};
    use crate::cpu::definitions::cpu::csr::build_csr_state;

    #[test]
    fn test_inst_a_lr() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        let mut reservation: Option<u32> = None;
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        bus.direct_write(BASE_ADDRESS as usize, &42u32.to_le_bytes()).unwrap();
        let rd = 5;
        inst_a_lr(rd, rs1, 0, &mut reg, &mut bus, &mut reservation, &csr, CPUMode::M).unwrap();
        assert_eq!(reg.read(5), 42);
        assert_eq!(reservation.unwrap(), BASE_ADDRESS);
    }

    #[test]
    fn test_inst_a_sc_succeeds_with_matching_reservation() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        let rs2 = 2;
        reg.write(2, 99);
        let mut reservation = Some(BASE_ADDRESS);
        let rd = 5;
        let csr = build_csr_state();
        inst_a_sc(rd, rs1, rs2, &mut reg, &mut bus, &mut reservation, &csr, CPUMode::M).unwrap();
        assert_eq!(reg.read(5), 0);
        assert_eq!(reservation, None);
        assert_eq!(bus.direct_read(BASE_ADDRESS as usize, ByteType::Word.as_num()).unwrap(), 99);
    }

    #[test]
    fn test_inst_a_sc_fails_without_reservation() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        let rs2 = 2;
        reg.write(2, 99);
        let mut reservation: Option<u32> = None;
        let rd = 5;
        let csr = build_csr_state();
        inst_a_sc(rd, rs1, rs2, &mut reg, &mut bus, &mut reservation, &csr, CPUMode::M).unwrap();
        assert_ne!(reg.read(5), 0);
        assert_eq!(bus.direct_read(BASE_ADDRESS as usize, ByteType::Word.as_num()).unwrap(), 0);
    }

    #[test]
    fn test_inst_a_amoswap() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        bus.direct_write(BASE_ADDRESS as usize, &10u32.to_le_bytes()).unwrap();
        let rs2 = 2;
        reg.write(2, 20);
        let rd = 5;
        let csr = build_csr_state();
        inst_a_amoswap(rd, rs1, rs2, &mut reg, &mut bus, &csr, CPUMode::M).unwrap();
        assert_eq!(reg.read(5), 10);
        assert_eq!(bus.direct_read(BASE_ADDRESS as usize, ByteType::Word.as_num()).unwrap(), 20);
    }

    #[test]
    fn test_inst_a_amoadd() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        bus.direct_write(BASE_ADDRESS as usize, &10u32.to_le_bytes()).unwrap();
        let rs2 = 2;
        reg.write(2, 5);
        let rd = 5;
        let csr = build_csr_state();
        inst_a_amoadd(rd, rs1, rs2, &mut reg, &mut bus, &csr, CPUMode::M).unwrap();
        assert_eq!(reg.read(5), 10);
        assert_eq!(bus.direct_read(BASE_ADDRESS as usize, ByteType::Word.as_num()).unwrap(), 15);
    }

    #[test]
    fn test_inst_a_amoxor() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        bus.direct_write(BASE_ADDRESS as usize, &0b0110u32.to_le_bytes()).unwrap();
        let rs2 = 2;
        reg.write(2, 0b0101);
        let rd = 5;
        let csr = build_csr_state();
        inst_a_amoxor(rd, rs1, rs2, &mut reg, &mut bus, &csr, CPUMode::M).unwrap();
        assert_eq!(reg.read(5), 0b0110);
        assert_eq!(bus.direct_read(BASE_ADDRESS as usize, ByteType::Word.as_num()).unwrap(), 0b0011);
    }

    #[test]
    fn test_inst_a_amoand() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        bus.direct_write(BASE_ADDRESS as usize, &0b0110u32.to_le_bytes()).unwrap();
        let rs2 = 2;
        reg.write(2, 0b0101);
        let rd = 5;
        let csr = build_csr_state();
        inst_a_amoand(rd, rs1, rs2, &mut reg, &mut bus, &csr, CPUMode::M).unwrap();
        assert_eq!(reg.read(5), 0b0110);
        assert_eq!(bus.direct_read(BASE_ADDRESS as usize, ByteType::Word.as_num()).unwrap(), 0b0100);
    }

    #[test]
    fn test_inst_a_amoor() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        bus.direct_write(BASE_ADDRESS as usize, &0b0110u32.to_le_bytes()).unwrap();
        let rs2 = 2;
        reg.write(2, 0b0101);
        let rd = 5;
        let csr = build_csr_state();
        inst_a_amoor(rd, rs1, rs2, &mut reg, &mut bus, &csr, CPUMode::M).unwrap();
        assert_eq!(reg.read(5), 0b0110);
        assert_eq!(bus.direct_read(BASE_ADDRESS as usize, ByteType::Word.as_num()).unwrap(), 0b0111);
    }

    #[test]
    fn test_inst_a_amomin() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        bus.direct_write(BASE_ADDRESS as usize, &((-5i32) as u32).to_le_bytes()).unwrap();
        let rs2 = 2;
        reg.write(2, 3);
        let rd = 5;
        let csr = build_csr_state();
        inst_a_amomin(rd, rs1, rs2, &mut reg, &mut bus, &csr, CPUMode::M).unwrap();
        assert_eq!(reg.read(5) as i32, -5);
        assert_eq!(bus.direct_read(BASE_ADDRESS as usize, ByteType::Word.as_num()).unwrap() as i32, -5);
    }

    #[test]
    fn test_inst_a_amomax() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        bus.direct_write(BASE_ADDRESS as usize, &((-5i32) as u32).to_le_bytes()).unwrap();
        let rs2 = 2;
        reg.write(2, 3);
        let rd = 5;
        let csr = build_csr_state();
        inst_a_amomax(rd, rs1, rs2, &mut reg, &mut bus, &csr, CPUMode::M).unwrap();
        assert_eq!(reg.read(5) as i32, -5);
        assert_eq!(bus.direct_read(BASE_ADDRESS as usize, ByteType::Word.as_num()).unwrap(), 3);
    }

    #[test]
    fn test_inst_a_amominu() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        bus.direct_write(BASE_ADDRESS as usize, &u32::MAX.to_le_bytes()).unwrap();
        let rs2 = 2;
        reg.write(2, 3);
        let rd = 5;
        let csr = build_csr_state();
        inst_a_amominu(rd, rs1, rs2, &mut reg, &mut bus, &csr, CPUMode::M).unwrap();
        assert_eq!(reg.read(5), u32::MAX);
        assert_eq!(bus.direct_read(BASE_ADDRESS as usize, ByteType::Word.as_num()).unwrap(), 3);
    }

    #[test]
    fn test_inst_a_amomaxu() {
        let mut reg = build_register_file();
        let mut bus = build_bus_state();
        let rs1 = 1;
        reg.write(1, BASE_ADDRESS);
        bus.direct_write(BASE_ADDRESS as usize, &u32::MAX.to_le_bytes()).unwrap();
        let rs2 = 2;
        reg.write(2, 3);
        let rd = 5;
        let csr = build_csr_state();
        inst_a_amomaxu(rd, rs1, rs2, &mut reg, &mut bus, &csr, CPUMode::M).unwrap();
        assert_eq!(reg.read(5), u32::MAX);
        assert_eq!(bus.direct_read(BASE_ADDRESS as usize, ByteType::Word.as_num()).unwrap(), u32::MAX);
    }
}