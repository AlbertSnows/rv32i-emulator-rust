use crate::definitions::cpu_definition::RegisterFile;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::definitions::codes::ExecutionSignal;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use crate::utility::bit_operations::shake_to_signed;

// jalr -- the only instruction under its opcode, so no op enum needed
// (same reasoning as JType). Not yet implemented.

pub fn parse_jalr_inst(raw_word: InstructionWord) -> Result<Format, String> {
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

pub fn execute_i_jalr_type(rd: usize, rs1: usize, imm: i32, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    Ok(ExecutionSignal::Continue)
}