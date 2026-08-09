// In RISC, the first two bits are reserved to discern 16bit from 32bit.
// We use 32 bit, so the first two should always be 11

use crate::definitions::op_codes;
use crate::definitions::masks;
use crate::instructions::r::inst_r_add;
use crate::instructions::r::parse_r_inst;
use crate::instructions::Format;
use crate::instructions::i::load::parse_load_inst;
use crate::instructions::i::alu_imm_or_shift::parse_alu_imm_or_shift_inst;
use crate::instructions::i::system::parse_system_inst;
use crate::instructions::i::jalr::parse_jalr_inst;
use crate::instructions::s::parse_s_inst;
use crate::instructions::u::parse_u_inst;
use crate::instructions::j::parse_j_inst;
use crate::instructions::b::parse_b_inst;
use crate::utility::bit_operations::mask;
use crate::fetcher::InstructionWord;

pub fn decode_word_to_instruction(raw_word: InstructionWord) -> Result<Format, String> {
    // op code is 7 bits wide.
    // the mask will keep the first 7 bits, toss the rest.
    let opcode = mask(raw_word.0, masks::OP_CODE);
    match opcode {
        op_codes::LOAD => parse_load_inst(raw_word), // todo: implement i type closure that takes op code type as first param?
        op_codes::ALU_IMM => parse_alu_imm_or_shift_inst(raw_word),
        op_codes::SYSTEM => parse_system_inst(raw_word),
        op_codes::JALR => parse_jalr_inst(raw_word),
        op_codes::R => parse_r_inst(raw_word),
        op_codes::S => parse_s_inst(raw_word),
        op_codes::B => parse_b_inst(raw_word),
        op_codes::LUI => parse_u_inst(raw_word, op_codes::LUI),
        op_codes::AUIPC => parse_u_inst(raw_word, op_codes::AUIPC),
        op_codes::J => parse_j_inst(raw_word),
        // b = binary format, # = signify binary format, 09 = output 9 total characters w/ 0 as padding
        _ => Err(format!("undefined opcode: {:#09b}", opcode))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_routes_r_to_parse_r_inst() {
        use crate::programs::instructions::ADD_X3_X1_X2;

        let raw_word = InstructionWord(ADD_X3_X1_X2);
        let result = decode_word_to_instruction(raw_word);
        assert!(matches!(result, Ok(Format::RType { .. })));
    }

    #[test]
    fn test_decode_word_to_instruction_unrecognized_opcode_returns_err() {
        // 0b0000000 isn't any of the 10 real RV32I opcodes (doesn't even
        // end in 11), so this should hit the catch-all and return Err
        // rather than a decoded Instruction.
        let raw_word = InstructionWord(0b0000000);
        let result = decode_word_to_instruction(raw_word);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_routes_load_to_parse_load_inst() {
        // lb x1, 4(x2)
        let raw_word = InstructionWord(0x00410083);
        let result = decode_word_to_instruction(raw_word);
        assert!(matches!(result, Ok(Format::LoadType { .. })));
    }

    #[test]
    fn test_decode_routes_alu_imm_to_parse_alu_imm_or_shift_inst() {
        // addi x5, x1, 10
        let raw_word = InstructionWord(0x00A08293);
        let result = decode_word_to_instruction(raw_word);
        assert!(matches!(result, Ok(Format::AluImmType { .. })));
    }

    #[test]
    fn test_decode_routes_system_to_parse_system_inst() {
        // ecall
        let raw_word = InstructionWord(0x00000073);
        let result = decode_word_to_instruction(raw_word);
        assert!(matches!(result, Ok(Format::SystemType { .. })));
    }

    #[test]
    fn test_decode_routes_jalr_to_parse_jalr_inst() {
        // jalr x1, x2, 8
        let raw_word = InstructionWord(0x008100E7);
        let result = decode_word_to_instruction(raw_word);
        assert!(matches!(result, Ok(Format::JalrType { .. })));
    }

    #[test]
    fn test_decode_routes_s_to_parse_s_inst() {
        // sw x2, 4(x1)
        let raw_word = InstructionWord(0x0020A223);
        let result = decode_word_to_instruction(raw_word);
        assert!(matches!(result, Ok(Format::SType { .. })));
    }

    #[test]
    fn test_decode_routes_b_to_parse_b_inst() {
        // beq x1, x2, 8
        let raw_word = InstructionWord(0x00208463);
        let result = decode_word_to_instruction(raw_word);
        assert!(matches!(result, Ok(Format::BType { .. })));
    }

    #[test]
    fn test_decode_routes_lui_to_parse_u_inst() {
        use crate::instructions::u::UOp;

        // lui x1, 5 -- checking op specifically (not just the UType variant)
        // since LUI and AUIPC share Format::UType; a bare variant match
        // wouldn't catch the two opcodes getting swapped in the dispatch.
        let raw_word = InstructionWord(0x000050B7);
        let result = decode_word_to_instruction(raw_word);
        assert!(matches!(result, Ok(Format::UType { op: UOp::Lui, .. })));
    }

    #[test]
    fn test_decode_routes_auipc_to_parse_u_inst() {
        use crate::instructions::u::UOp;

        // auipc x1, 5 -- same reasoning as the lui test above
        let raw_word = InstructionWord(0x00005097);
        let result = decode_word_to_instruction(raw_word);
        assert!(matches!(result, Ok(Format::UType { op: UOp::Auipc, .. })));
    }

    #[test]
    fn test_decode_routes_j_to_parse_j_inst() {
        // jal x1, 16
        let raw_word = InstructionWord(0x010000EF);
        let result = decode_word_to_instruction(raw_word);
        assert!(matches!(result, Ok(Format::JType { .. })));
    }
}