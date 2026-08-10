// U-type
//
//  31                                12 11     7 6      0
// |            imm[31:12]             |   rd    | opcode |
// |                20                 |    5    |   7    |
//
// lui:   rd <- imm << 12
// auipc: rd <- pc + (imm << 12)
// no register operands in, one register operand out, one 20-bit immediate
// that becomes the upper bits of a 32-bit value. used (with an I-type addi)
// to build large constants two instructions at a time.
// e.g. lui, auipc
use crate::instructions::Format;
use crate::fetcher::InstructionWord;
use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::cpu_definition::PCState;
use crate::instructions::i::system::SystemOp;
use crate::definitions::codes::ExecutionSignal;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use crate::definitions::op_codes;

#[derive(Debug, PartialEq)]
pub enum UOp {
    Lui,
    Auipc
}

pub fn parse_u_inst(raw_word: InstructionWord, opcode: u32) -> Result<Format, String> {
    let content = raw_word.0;
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    let imm_as_upper_bits = (content & masks::U_TYPE_IMM) as i32;
    let instruction_name = match opcode {
        op_codes::LUI => Ok(UOp::Lui),
        op_codes::AUIPC => Ok(UOp::Auipc),
        _ => Err(format!("Unrecognized U type"))
    }?;
    Ok(Format::UType { 
        op: instruction_name,
        rd: reg_dest as usize,
        imm_upper: imm_as_upper_bits
    })
}

pub fn execute_u_type(op: &UOp, rd: usize, imm_upper: i32, register: &mut RegisterFile, pc: &PCState) -> Result<ExecutionSignal, String> {
    match op {
        UOp::Lui => inst_u_lui(),
        UOp::Auipc => inst_u_auipc(),
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_u_lui() {
    // rd <- imm_upper (already shifted into place, low 12 bits zero)
    register.write(rd, imm_upper);
}

pub fn inst_u_auipc() {
    // rd <- pc + imm_upper
    register.write(rd, pc.read() + imm_upper);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_u_inst_lui() {
        // lui x1, 5 -- opcode = 0110111 (LUI), rd = 1, imm_upper = 5 << 12
        let raw_word = InstructionWord(0x000050B7);
        let result = parse_u_inst(raw_word, op_codes::LUI);
        assert_eq!(result, Ok(Format::UType { op: UOp::Lui, rd: 1, imm_upper: 5 << 12 }));
    }

    #[test]
    fn test_parse_u_inst_auipc() {
        // auipc x1, 5 -- opcode = 0010111 (AUIPC), same shape as lui otherwise
        let raw_word = InstructionWord(0x00005097);
        let result = parse_u_inst(raw_word, op_codes::AUIPC);
        assert_eq!(result, Ok(Format::UType { op: UOp::Auipc, rd: 1, imm_upper: 5 << 12 }));
    }

    #[test]
    fn test_inst_u_lui() {
        let mut reg = build_register_file();
        let rd = 1;
        // 5 << 12 = 5 x 2^12 = 5 x 4096 = 20480 = 0x5000
        // 5 = 0b101
        // becomes 101_0000_0000_0000 
        let upper = 5 << 12;
        inst_u_lui(rd, upper, reg);
        assert_eq!(reg.read(1), 5 << 12); 
    }

    #[test]
    fn test_inst_u_auipc() {
        let mut reg = build_register_file();
        let mut pc = build_pc_state();
        pc.write(1);
        let rd = 1;
        let upper = 5 << 12;
        inst_u_auipc(rd, upper, pc, reg);
        assert_eq!(reg.read(1), (5 << 12) + 1); 
    }
}