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
    let imm_signed = (content & masks::U_TYPE_IMM) as i32;
    let instruction_name = match opcode {
        op_codes::LUI => Ok(UOp::Lui),
        op_codes::AUIPC => Ok(UOp::Auipc),
        _ => Err(format!("Unrecognized U type"))
    }?;
    Ok(Format::UType { 
        op: instruction_name,
        rd: reg_dest as usize,
        imm: imm_signed
    })
}

pub fn execute_u_type(op: &UOp, rd: usize, imm: i32, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    Ok(ExecutionSignal::Continue)
}