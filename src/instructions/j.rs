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
use crate::instructions::Format;
use crate::fetcher::InstructionWord;
use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::cpu_definition::PCState;
use crate::definitions::codes::ExecutionSignal;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use crate::utility::bit_operations::merge_bits;
use crate::utility::bit_operations::shake_to_signed;

#[derive(Debug, PartialEq)]
pub enum JOp {
    Jal
}

pub fn parse_j_inst(raw_word: InstructionWord) -> Result<Format, String> {
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

pub fn execute_j_type(op: &JOp, rd: usize, imm: i32, register: &mut RegisterFile, pc: &PCState) -> Result<ExecutionSignal, String> {
    let write_value = (pc.read() + 4) as u32;
    register.write(rd, write_value);
    Ok(ExecutionSignal::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::cpu_definition::build_register_file;
    use crate::definitions::cpu_definition::build_pc_state;

    #[test]
    fn test_parse_j_inst() {
        // jal x1, 16
        // opcode = 1101111 (J), rd = 1, imm = 16
        let raw_word = InstructionWord(0x010000EF);
        let result = parse_j_inst(raw_word);
        assert_eq!(result, Ok(Format::JType { op: JOp::Jal, rd: 1, imm: 16 }));
    }

    #[test]
    fn test_execute_j_type_writes_return_address() {
        let mut register = build_register_file();
        let mut pc = build_pc_state();
        pc.write(100);

        let outcome = execute_j_type(&JOp::Jal, 5, 16, &mut register, &pc);

        assert_eq!(outcome, Ok(ExecutionSignal::Continue));
        assert_eq!(register.read(5), 104);
    }
}