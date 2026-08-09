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
    fn test_decode_word_to_instruction() {
        use crate::instructions::r::AluOp;
        use crate::programs::instructions::ADD_X3_X1_X2;

        // add x3, x1, x2 -- confirms opcode dispatch routes to parse_r_inst
        // correctly; parse_r_inst's own funct3/funct7 logic is covered by
        // r.rs's tests, not re-tested here.
        let raw_word = InstructionWord(ADD_X3_X1_X2);
        let result = decode_word_to_instruction(raw_word);
        assert_eq!(result, Ok(Format::RType { op: AluOp::Add, rd: 3, rs1: 1, rs2: 2 }));
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
        use crate::instructions::i::load::LoadOp;

        // lb x1, 4(x2)
        // opcode = 0000011 (LOAD)
        // funct3 = 000 (lb)
        // rd = 1
        // rs1 = 2
        // imm = 4
        let raw_word = InstructionWord(0x00410083);
        let result = decode_word_to_instruction(raw_word);
        assert_eq!(result, Ok(Format::LoadType { op: LoadOp::Lb, rd: 1, rs1: 2, imm: 4 }));
    }

    #[test]
    fn test_decode_routes_alu_imm_to_parse_alu_imm_or_shift_inst() {
        use crate::instructions::i::alu_imm_or_shift::AluImmOp;

        // addi x5, x1, 10
        // opcode = 0010011 (ALU_IMM)
        // funct3 = 000 (addi)
        // rd = 5
        // rs1 = 1
        // imm = 10
        let raw_word = InstructionWord(0x00A08293);
        let result = decode_word_to_instruction(raw_word);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 5, rs1: 1, imm: 10 }));
    }

    #[test]
    fn test_decode_routes_system_to_parse_system_inst() {
        use crate::instructions::i::system::SystemOp;

        // ecall
        // opcode = 1110011 (SYSTEM)
        // every other field = 0
        // bit 20 (ecall/ebreak discriminator) = 0 -> ECall
        let raw_word = InstructionWord(0x00000073);
        let result = decode_word_to_instruction(raw_word);
        assert_eq!(result, Ok(Format::SystemType { op: SystemOp::ECall }));
    }

    #[test]
    fn test_decode_routes_jalr_to_parse_jalr_inst() {
        // jalr x1, x2, 8
        // opcode = 1100111 (JALR)
        // funct3 = 000
        // rd = 1
        // rs1 = 2
        // imm = 8
        let raw_word = InstructionWord(0x008100E7);
        let result = decode_word_to_instruction(raw_word);
        assert_eq!(result, Ok(Format::JalrType { rd: 1, rs1: 2, imm: 8 }));
    }

    #[test]
    fn test_decode_routes_s_to_parse_s_inst() {
        use crate::instructions::s::SOp;

        // sw x2, 4(x1)
        // opcode = 0100011 (S)
        // funct3 = 010 (sw)
        // rs1 = 1
        // rs2 = 2
        // imm = 4
        let raw_word = InstructionWord(0x0020A223);
        let result = decode_word_to_instruction(raw_word);
        assert_eq!(result, Ok(Format::SType { op: SOp::Sw, imm: 4, rs1: 1, rs2: 2 }));
    }

    #[test]
    fn test_decode_routes_b_to_parse_b_inst() {
        use crate::instructions::b::BOp;

        // beq x1, x2, 8
        // opcode = 1100011 (B)
        // funct3 = 000 (beq)
        // rs1 = 1
        // rs2 = 2
        // imm = 8
        let raw_word = InstructionWord(0x00208463);
        let result = decode_word_to_instruction(raw_word);
        assert_eq!(result, Ok(Format::BType { op: BOp::Beq, imm: 8, rs1: 1, rs2: 2 }));
    }

    // LUI and AUIPC are the instructions that fill out the top 20 bits of a u32, hence 5 << 12
    #[test]
    fn test_decode_routes_lui_to_parse_u_inst() {
        use crate::instructions::u::UOp;

        // lui x1, 5
        // opcode = 0110111 (LUI)
        // rd = 1
        // raw imm field = 5
        // imm_upper = 5 << 12 = 20480 (already shifted into place)
        let raw_word = InstructionWord(0x000050B7);
        let result = decode_word_to_instruction(raw_word);
        assert_eq!(result, Ok(Format::UType { op: UOp::Lui, rd: 1, imm_upper: 5 << 12 }));
    }

    #[test]
    fn test_decode_routes_auipc_to_parse_u_inst() {
        use crate::instructions::u::UOp;

        // auipc x1, 5
        // opcode = 0010111 (AUIPC)
        // rd = 1
        // raw imm field = 5
        // imm_upper = 20480 (same shape as LUI, different opcode)
        let raw_word = InstructionWord(0x00005097);
        let result = decode_word_to_instruction(raw_word);
        assert_eq!(result, Ok(Format::UType { op: UOp::Auipc, rd: 1, imm_upper: 5 << 12 }));
    }

    #[test]
    fn test_decode_routes_j_to_parse_j_inst() {
        use crate::instructions::j::JOp;

        // jal x1, 16
        // opcode = 1101111 (J)
        // rd = 1
        // imm = 16
        // note: raw bits are scrambled per imm[20|10:1|11|19:12] -- 16 only
        // becomes visible after parse_j_inst's reassembly, not from the hex directly
        let raw_word = InstructionWord(0x010000EF);
        let result = decode_word_to_instruction(raw_word);
        assert_eq!(result, Ok(Format::JType { op: JOp::Jal, rd: 1, imm: 16 }));
    }
}