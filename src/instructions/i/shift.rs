use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::codes::ExecutionSignal;
use crate::instructions::i::alu_imm_or_shift::IShOp;
use crate::definitions::trap_cause::TrapCause;

pub fn execute_i_shift_type(op: &IShOp, rd: usize, rs1: usize, shamt: usize, register: &mut RegisterFile) -> Result<ExecutionSignal, TrapCause> {
    match op {
        IShOp::Slli => inst_i_slli(rd, rs1, shamt, register),
        IShOp::Srli => inst_i_srli(rd, rs1, shamt, register),
        IShOp::Srai => inst_i_srai(rd, rs1, shamt, register),
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_i_slli(rd: usize, rs1: usize, shamt: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 << shamt
    let val = reg_file.read(rs1);
    let shamted_val = val << shamt;
    reg_file.write(rd, shamted_val);
}

pub fn inst_i_srli(rd: usize, rs1: usize, shamt: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 >>u shamt
    let val = reg_file.read(rs1);
    let shamted_val = val >> shamt;
    reg_file.write(rd, shamted_val as u32);

}

pub fn inst_i_srai(rd: usize, rs1: usize, shamt: usize, reg_file: &mut RegisterFile) {
    // rd <- rs1 >>s shamt
    let val = reg_file.read(rs1) as i32;
    let shamted_val = val >> shamt;
    reg_file.write(rd, shamted_val as u32);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_definition::build_register_file;

    #[test]
    fn test_inst_i_slli() {
        let mut reg_file = build_register_file();
        let rd = 1;
        let rs1 = 2;
        let shamt = 0b0010;
        reg_file.write(2, 0b0101);
        inst_i_slli(rd, rs1, shamt, &mut reg_file); 
        assert_eq!(reg_file.read(1), 0b0001_0100);
    }

    #[test]
    fn test_inst_i_srli() {
        let mut reg_file = build_register_file();
        let rd = 1;
        let rs1 = 2;
        reg_file.write(2, 0b0101);
        let shamt = 0b10;
        inst_i_srli(rd, rs1, shamt, &mut reg_file); 
        assert_eq!(reg_file.read(1), 1);
    }

    #[test]
    fn test_inst_i_srai() {
        let mut reg_file = build_register_file();
        let rd = 1;
        let rs1 = 2;
        reg_file.write(2, -8i32 as u32);
        let shamt = 1;
        inst_i_srai(rd, rs1, shamt, &mut reg_file);
        assert_eq!(reg_file.read(1) as i32, -4);
    }

    // --- boundary tests: max valid shift amount (31) plus extreme operand values ---

    #[test]
    fn test_inst_i_slli_at_max_shift_no_panic() {
        let mut reg_file = build_register_file();
        let rd = 1;
        let rs1 = 2;
        let shamt = 31;
        // 1 = 0b1, shifted left 31 places lands that single bit at the top
        reg_file.write(2, 1);
        inst_i_slli(rd, rs1, shamt, &mut reg_file);
        // 0b1 << 31 = 0b1000_0000_0000_0000_0000_0000_0000_0000 = 0x8000_0000
        assert_eq!(reg_file.read(1), 0x8000_0000);
    }

    #[test]
    fn test_inst_i_srli_at_max_shift_no_panic() {
        let mut reg_file = build_register_file();
        let rd = 1;
        let rs1 = 2;
        let shamt = 31;
        // u32::MAX = 0xFFFF_FFFF = 0b1111_1111_1111_1111_1111_1111_1111_1111
        reg_file.write(2, u32::MAX);
        inst_i_srli(rd, rs1, shamt, &mut reg_file);
        // logical shift right 31: only bit 31 survives, now sitting at bit 0
        // 0b1111_...1111 >>u 31 = 0b0000_...0001 = 0x1 = 1
        assert_eq!(reg_file.read(1), 1);
    }

    #[test]
    fn test_inst_i_srai_at_i32_min_max_shift() {
        let mut reg_file = build_register_file();
        let rd = 1;
        let rs1 = 2;
        let shamt = 31;
        // i32::MIN = 0x8000_0000 = 0b1000_0000_0000_0000_0000_0000_0000_0000
        reg_file.write(2, i32::MIN as u32);
        inst_i_srai(rd, rs1, shamt, &mut reg_file);
        // arithmetic shift right 31: the sign bit (1) replicates into every
        // vacated position, so all 32 bits end up set
        // 0b1000_...0000 >>s 31 = 0b1111_...1111 = 0xFFFF_FFFF = -1 (as i32)
        assert_eq!(reg_file.read(1) as i32, -1);
    }
}
