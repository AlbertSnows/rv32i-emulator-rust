use crate::cpu::definitions::codes::ExecutionSignal;
use crate::cpu::definitions::cpu::cpu_definition::{PCState, RegisterFile};
use crate::cpu::definitions::masks;
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::cpu::fetcher::Instruction;
use crate::utility::types::ByteType;
// J-type
//
//  31                                         12 11     7 6      0
// |       imm[20|10:1|11|19:12]                |   rd    | opcode |
// |                    20                      |    5    |   7    |
//
// rd <- pc + 4; pc <- pc + imm
// same field shape as U-type, but the immediate means "jump target offset"
// (PC-relative, always even, bit 0 implied zero) instead of raw upper bits.
// immediate bits arrive scrambled: [20][10:1][11][19:12].
// e.g. jal
use crate::cpu::instructions::Format;
use crate::utility::bit_operations::{mask_and_shift, merge_bits, shake_to_signed};

#[derive(Debug, PartialEq)]
pub enum JOp {
    Jal
}

pub fn parse_j_inst(raw_word: Instruction) -> Result<Format, TrapCause> {
    let content = raw_word.0;
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    // [|20|10:1|11|19:12]
    // [|31|30:21|20|19:12]
    // [|0|00_0000_0000|0|0000_0000]
    let imm_raw = mask_and_shift(content, masks::J_TYPE_IMM);
    let imm_nineteen_to_twelve = imm_raw & 0b1111_1111;
    let imm_eleven = (imm_raw >> 8) & 1; // & 1 keeps only 11
    let imm_ten_to_one = mask_and_shift(imm_raw, 0b111_1111_1110_0000_0000);
    let imm_twenty = imm_raw >> 19;
    let imm_combined_unsigned = merge_bits(&[
        (imm_ten_to_one, 1),
        (imm_eleven, 11),
        (imm_nineteen_to_twelve, 12),
        (imm_twenty, 20)
    ]);
    let imm_val = shake_to_signed(imm_combined_unsigned, 21);
    Ok(Format::JType { 
        op: JOp::Jal,
        rd: reg_dest as usize,
        imm: imm_val
    })
}

pub fn execute_j_type(op: &JOp, rd: usize, imm: i32, register: &mut RegisterFile, pc: &PCState) -> Result<ExecutionSignal, TrapCause> {
    // adding i32 as u32 still does the arithmetic correctly
    // let jump_target = (pc.read() as u32).wrapping_add(imm as u32);
    // if (jump_target % advance_amount != 0) {
    //     Err(TrapCause::InstructionAddressMisaligned { address: jump_target as usize })
    // } else {
    //     register.write(rd, (pc.read() as u32).wrapping_add(4));
    //     Ok(ExecutionSignal::Continue)
    // }
    Ok(ExecutionSignal::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::definitions::cpu::cpu_definition::build_pc_state;
    use crate::cpu::definitions::cpu::cpu_definition::build_register_file;

    #[test]
    fn test_parse_j_inst() {
        // jal x1, 16
        // opcode = 1101111 (J), rd = 1, imm = 16
        let raw_word = Instruction(0x010000EF, ByteType::Word);
        let result = parse_j_inst(raw_word);
        assert_eq!(result, Ok(Format::JType { op: JOp::Jal, rd: 1, imm: 16 }));
    }

}