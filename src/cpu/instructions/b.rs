// B-type
//
//  31            25 24    20 19    15 14   12 11          7 6      0
// | imm[12|10:5]  |  rs2   |  rs1   | funct3 | imm[4:1|11]  | opcode |
// |     7         |   5    |   5    |   3    |     5        |   7    |
//
// pc <- pc + ((rs1 CMP rs2) ? imm : 4)   (no rd -- branches never write a register)
// same field shape as S-type, but the immediate means "branch offset" and is
// always even, so bit 0 is implied zero and isn't stored -- one extra bit of
// range for free. immediate bits arrive scrambled: [12][10:5] ... [4:1][11].
// e.g. beq, bne, blt, bge, bltu, bgeu
use crate::cpu::instructions::Format;
use crate::cpu::fetcher::InstructionWord;
use crate::cpu::definitions::cpu::cpu_definition::RegisterFile;
use crate::cpu::definitions::codes::ExecutionSignal;
use crate::cpu::utility::bit_operations::{mask_and_shift, merge_bits, shake_to_signed};
use crate::cpu::definitions::masks;
use crate::cpu::definitions::trap_cause::TrapCause;

#[derive(Debug, PartialEq)]
pub enum BOp {
    Beq,
    Bne,
    Blt,
    Bge,
    Bltu,
    Bgeu
}

// suppose imm_first = 29
// 29 = 11101
// imm_first is [4:1|11] which means
// 1    1    1    0    1
// imm4 imm3 imm2 imm1 imm11
// so the first bit is imm11
// suppose imm_second = 127
// 127 = 1111111
// imm_second is [12|10:5] which means
// 1     1     1    1    1    1    1
// imm12 imm10 imm9 imm8 imm7 imm6 imm5

pub fn parse_b_inst(raw_word: InstructionWord) -> Result<Format, TrapCause> {
    let content = raw_word.0;
    let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    let reg_source_two = mask_and_shift(content, masks::REG_SOURCE_TWO);
    // Syntax:
    // X:Y = bits X to y, e.g. 10:8 = 10, 9, 8
    // A|B = separate, notcontigous groups
    // [4:1|11] = imm[4:1], imm[11 @ 0]
    let imm_first = mask_and_shift(content, masks::B_TYPE_IMM_FIRST);
    let imm_four_to_one = imm_first >> 1; // 01010 -> 00101
    let imm_eleven = imm_first & 1;
    // [12|10:5] = imm[12], im[10:5],
    let imm_second = mask_and_shift(content, masks::B_TYPE_IMM_SECOND); // e.g. 0b|1|010101
    let imm_twelve = imm_second >> 6; // 1
    let imm_ten_to_five = imm_second & 0b111111; // 010101
    // NOTE: Now we have, say, 0b1_0101_0101_0101
    // But for u32, that means we have 0b0000_0000_0000_0000_0001_0101_0101_0101
    // imm is i32 not u32, so we need to covert the leading digits from 0 to 1
    let imm_combined_unsigned = merge_bits(&[
        (imm_twelve, 12), 
        (imm_eleven, 11), 
        (imm_ten_to_five, 5), 
        (imm_four_to_one, 1)
    ]);
    // for 0bX00, X determines what's brought down if we shift right (>>)
    // for 0b1000 >> 2 => 0b1110 
    // for 0b0111 >> 2 => 0b0001
    // we want imm to be signed, and it's left most bit is currently bit position 13
    // 32 - 13 = 19, so we "shake" 19, e.g. << 19 and >> 19
    // remember: u32 has no notion of sign, to drag the leftmost bit down, we must mark it as i32
    let imm_signed = shake_to_signed(imm_combined_unsigned, 13);

    let funct_3 = mask_and_shift(content, masks::FUNCT_THREE);
    let instruction_name = match funct_3 {
        0b000 => Ok(BOp::Beq),
        0b001 => Ok(BOp::Bne),
        0b100 => Ok(BOp::Blt),
        0b101 => Ok(BOp::Bge),
        0b110 => Ok(BOp::Bltu),
        0b111 => Ok(BOp::Bgeu),
        _ => Err(TrapCause::IllegalInstruction { instruction: Some(content) })
    }?;
    Ok(Format::BType { 
        op: instruction_name,
        imm: imm_signed,
        rs1: reg_source_one as usize,
        rs2: reg_source_two as usize
    })
}

pub fn execute_b_type(op: &BOp, imm: i32, rs1: usize, rs2: usize, register: &mut RegisterFile) -> Result<ExecutionSignal, TrapCause> {
    // B Type only modifies PC, which is handled in the advance_pc step
    // This execution is a no op
    Ok(ExecutionSignal::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_b_inst() {
        // beq x1, x2, 8
        // opcode = 1100011 (B), funct3 = 000 (beq), rs1 = 1, rs2 = 2, imm = 8
        let raw_word = InstructionWord(0x00208463);
        let result = parse_b_inst(raw_word);
        assert_eq!(result, Ok(Format::BType { op: BOp::Beq, imm: 8, rs1: 1, rs2: 2 }));
    }

}
