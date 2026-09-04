use crate::cpu::definitions::codes::ExecutionSignal;
use crate::cpu::definitions::cpu::cpu_definition::{PCState, RegisterFile};
use crate::cpu::definitions::masks;
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::cpu::fetcher::Instruction;
use crate::utility::types::ByteType;
use crate::cpu::instructions::Format;
use crate::utility::bit_operations::{mask_and_shift, shake_to_signed};

// jalr -- the only instruction under its opcode, so no op enum needed
// (same reasoning as JType). Not yet implemented.

pub fn parse_jalr_inst(raw_word: Instruction) -> Result<Format, TrapCause> {
    let content = raw_word.0;
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    let imm_unsigned = mask_and_shift(content, masks::I_TYPE_JALR);
    let imm_val = shake_to_signed(imm_unsigned, 12);
    let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    Ok(Format::JalrType {
        imm: imm_val,
        rd: reg_dest as usize,
        rs1: reg_source_one as usize
    })
}

pub fn execute_i_jalr_type(rd: usize, rs1: usize, imm: i32, register: &mut RegisterFile, pc: &mut PCState) -> Result<ExecutionSignal, TrapCause> {
    // let rs1_val = register.read(rs1);
    // // 1 = ..001, !1 = ..110
    // // (combine rs1 and imm) -> and with !1 which means keep all bits in (rs1 + imm) but force it to be even (rounded down)
    // let new_value = (rs1_val.wrapping_add(imm as u32)) & !1;
    // if (new_value % 4 != 0) {
    //     Err(TrapCause::InstructionAddressMisaligned { address: new_value as usize })
    // } else {
    //     register.write(rd, (pc.read().wrapping_add(4)) as u32);
    //     pc.write(new_value as usize);
        Ok(ExecutionSignal::Continue)
    // }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::definitions::cpu::cpu_definition::{build_pc_state, build_register_file};

    #[test]
    fn test_parse_jalr_inst() {
        // jalr x1, x2, 8 -- opcode = 1100111 (JALR), rd = 1, rs1 = 2, imm = 8
        let raw_word = Instruction(0x008100E7, ByteType::Word);
        let result = parse_jalr_inst(raw_word);
        assert_eq!(result, Ok(Format::JalrType { rd: 1, rs1: 2, imm: 8 }));
    }

}