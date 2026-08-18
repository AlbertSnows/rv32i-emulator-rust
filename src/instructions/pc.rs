use crate::definitions::cpu::cpu_definition::{PCState, RegisterFile};
use crate::instructions::Format;
use crate::instructions::b::BOp;
use crate::definitions::trap_cause::TrapCause;
use crate::instructions::r::AluOp;
use crate::instructions::i::system::{SystemOp};

pub fn advance_pc(pc: &mut PCState, instruction: &Format, reg_file: &RegisterFile) -> Result<usize, TrapCause> {
    let pc_value = pc.read() as u32;

    let new_value = match instruction {
        Format::JType { op, rd, imm } => pc_value.wrapping_add(*imm as u32),
        Format::JalrType { rd, rs1, imm } => {
            let rs1_val = reg_file.read(*rs1);
            // 1 = ..001, !1 = ..110
            // (combine rs1 and imm) -> and with !1 which means keep all bits in (rs1 + imm) but force it to be even (rounded down)
            let new_value = (rs1_val.wrapping_add(*imm as u32)) & !1;
            new_value
        },
        Format::BType { op, imm, rs1, rs2 } => {
            let rs1_val = reg_file.read(*rs1);
            let rs2_val = reg_file.read(*rs2);
            let imm_val = *imm as u32;
            match op {
                BOp::Beq => if rs1_val == rs2_val { pc_value.wrapping_add(imm_val) } else { pc_value.wrapping_add(4) },
                BOp::Bne => if rs1_val != rs2_val { pc_value.wrapping_add(imm_val) } else { pc_value.wrapping_add(4) },
                BOp::Bltu => if rs1_val < rs2_val { pc_value.wrapping_add(imm_val) } else { pc_value.wrapping_add(4) },
                BOp::Bgeu => if rs1_val >= rs2_val { pc_value.wrapping_add(imm_val) } else { pc_value.wrapping_add(4) },
                BOp::Blt => if (rs1_val as i32) < (rs2_val as i32) { pc_value.wrapping_add(imm_val) } else { pc_value.wrapping_add(4) },
                BOp::Bge => if (rs1_val as i32) >= (rs2_val as i32) { pc_value.wrapping_add(imm_val) } else { pc_value.wrapping_add(4) }
            }
        },
        Format::SystemType { op: SystemOp::MRet } => pc_value,
        _ => pc_value.wrapping_add(4)
    };
    let is_invalid_pc_state = new_value % 4 != 0;
    if is_invalid_pc_state {
        return Err(TrapCause::InstructionAddressMisaligned { address: new_value as usize });
    }
    pc.write(new_value as usize);
    Ok(pc.read())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::cpu::cpu_definition::build_pc_state;
    use crate::definitions::cpu::cpu_definition::build_register_file;
    use crate::instructions::j::JOp;
    use crate::instructions::u::UOp;

    // --- boundary tests ---

    #[test]
    fn test_advance_pc_jtype_wraps_at_i32_max() {
        let mut pc = build_pc_state();
        // i32::MAX = 0x7FFF_FFFF = 0b0111_1111_1111_1111_1111_1111_1111_1111
        pc.write(i32::MAX as usize);
        let reg_file = build_register_file();
        let instruction = Format::JType { op: JOp::Jal, rd: 1, imm: 1 };
        let result = advance_pc(&mut pc, &instruction, &reg_file);
        // adding 1 carries through every one-bit and flips the sign bit:
        // 0b0111_...1111 + 1 = 0b1000_0000_0000_0000_0000_0000_0000_0000
        // = 0x8000_0000 = i32::MIN
        assert_eq!(result.unwrap() as i32, i32::MIN);
    }

    #[test]
    fn test_advance_pc_jalrtype_wraps_at_i32_max() {
        let mut pc = build_pc_state();
        let mut reg_file = build_register_file();
        // i32::MAX = 0x7FFF_FFFF = 0b0111_1111_1111_1111_1111_1111_1111_1111
        reg_file.write(2, i32::MAX as u32);
        let instruction = Format::JalrType { rd: 1, rs1: 2, imm: 1 };
        let result = advance_pc(&mut pc, &instruction, &reg_file);
        // = i32::MIN, and & !1 leaves it unchanged since bit 0 is already 0
        assert_eq!(result.unwrap() as i32, i32::MIN);
    }

    #[test]
    fn test_advance_pc_btype_taken_wraps_at_i32_max() {
        let mut pc = build_pc_state();
        pc.write(i32::MAX as usize);
        let mut reg_file = build_register_file();
        reg_file.write(2, 5);
        reg_file.write(3, 5);
        let instruction = Format::BType { op: BOp::Beq, imm: 1, rs1: 2, rs2: 3 };
        let result = advance_pc(&mut pc, &instruction, &reg_file);
        assert_eq!(result.unwrap() as i32, i32::MIN);
    }

    #[test]
    fn test_advance_pc_btype_not_taken_wraps_at_i32_max() {
        let mut pc = build_pc_state();
        pc.write((i32::MAX - 3) as usize);
        let mut reg_file = build_register_file();
        reg_file.write(2, 5);
        reg_file.write(3, 9); // not equal -- branch not taken, falls through to pc + 4
        let instruction = Format::BType { op: BOp::Beq, imm: 100, rs1: 2, rs2: 3 };
        let result = advance_pc(&mut pc, &instruction, &reg_file);
        println!("{:?}", result);
        assert_eq!(result.unwrap() as i32, -2147483648);
    }

        #[test]
    fn test_advance_pc_btype_not_taken_wraps_at_i32_max_bad_addr() {
        let mut pc = build_pc_state();
        pc.write(i32::MAX as usize);
        let mut reg_file = build_register_file();
        reg_file.write(2, 5);
        reg_file.write(3, 9); // not equal -- branch not taken, falls through to pc + 4
        let instruction = Format::BType { op: BOp::Beq, imm: 100, rs1: 2, rs2: 3 };
        let result = advance_pc(&mut pc, &instruction, &reg_file);
        // i32::MAX (0x7FFFFFFF) + 4 wraps to 0x80000003
        assert_eq!(result, Err(TrapCause::InstructionAddressMisaligned { address: 2147483651 }));
    }

    #[test]
    fn test_advance_pc_default_case_wraps_at_i32_max() {
        let mut pc = build_pc_state();
        pc.write((i32::MAX - 3) as usize);
        let reg_file = build_register_file();
        let instruction = Format::UType { op: UOp::Lui, rd: 1, imm_upper: 0 };
        let result = advance_pc(&mut pc, &instruction, &reg_file);
        assert_eq!(result.unwrap() as i32, -2147483648);
    }

    #[test]
    fn test_advance_pc_rejects_misaligned_starting_pc() {
        // an ordinary, non-branching instruction (e.g. an R-type add),
        // starting from a pc that isn't 4-byte aligned -- advance_pc
        // should return Err(InstructionAddressMisaligned { address }),
        // not silently succeed. This is the general case, distinct from
        // the i32::MAX-wrapping tests above.
        let mut pc = build_pc_state();
        pc.write(1);
        let mut reg_file = build_register_file();
        let instruction = Format::RType { op: AluOp::Add, rs1: 2, rs2: 3, rd: 23};
        reg_file.write(2, 1);
        reg_file.write(3, 2);
        let result = advance_pc(&mut pc, &instruction, &reg_file);
        assert_eq!(pc.read(), 1);
        assert_eq!(result, Err(TrapCause::InstructionAddressMisaligned { address: 5 }));
    }
}