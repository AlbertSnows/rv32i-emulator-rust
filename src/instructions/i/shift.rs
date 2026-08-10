use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::codes::ExecutionSignal;
use crate::instructions::i::alu_imm_or_shift::IShOp;

pub fn execute_i_shift_type(op: &IShOp, rd: usize, rs1: usize, shamt: usize, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
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
        let shamt = 0b11;
        inst_i_srli(rd, rs1, shamt, &mut reg_file); 
        assert_eq!(reg_file.read(1), 0b0000_1010);
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
}
