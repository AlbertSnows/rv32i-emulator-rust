use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::codes::ExecutionSignal;
use crate::instructions::i::alu_imm_or_shift::IShOp;

pub fn execute_i_shift_type(op: &IShOp, rd: usize, rs1: usize, shamt: usize, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    match op {
        IShOp::Slli => inst_i_slli(),
        IShOp::Srli => inst_i_srli(),
        IShOp::Srai => inst_i_srai(),
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_i_slli() {
    // rd <- rs1 << shamt
    let shamted_rs1 = rs1 << shamt;
    register.write(rd, shamted_rs1);
}

pub fn inst_i_srli() {
    // rd <- rs1 >>u shamt
    let rs1_shamt = rs1 >> shamt;
    register.write(rd, rs1_shamt);
}

pub fn inst_i_srai() {
    // rd <- rs1 >>s shamt
    let rs1_shamt = rs1 >> shamt;
    register.write(rd, rs1_shamt);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inst_i_slli() {
        let rd = 1;
        let rs1 = 0b0101;
        let shamt = 0b1010;
        let reg_file = build_register_file();
        execute_i_jalr_type(); 
        assert_eq!(reg_file.read(1), 0b0101_1010);
    }

    #[test]
    fn test_inst_i_srli() {
        let rd = 1;
        let rs1 = 0b0101;
        let shamt = 0b1010;
        let reg_file = build_register_file();
        execute_i_jalr_type(); 
        assert_eq!(reg_file.read(1), 0b0101_1010);
    }

    #[test]
    fn test_inst_i_srai() {
        let rd = 1;
        let rs1 = 0b0101;
        let shamt = 0b1010;
        let reg_file = build_register_file();
        execute_i_jalr_type(); 
        assert_eq!(reg_file.read(1), 0b0101_1010);
    }
}
