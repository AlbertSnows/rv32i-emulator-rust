// CSR (Control and Status Register) instructions -- Zicsr extension,
//  31          20 19    15 14   12 11     7 6      0
// | csr[11:0]    |rs1/uimm| funct3 |   rd    | opcode |
// |     12       |   5    |   3    |    5    |   7    |
//
// rd <- CSR[csr] (old value, always read first); CSR[csr] <- new value
// rs1/uimm is a register index for csrrw/csrrs/csrrc, but a 5-bit

use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::cpu_definition::CsrState;
use crate::definitions::cpu_definition::CPUState;
use crate::definitions::cpu_definition::CPUMode;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::definitions::codes::ExecutionSignal;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use crate::definitions::trap_cause::TrapCause;

#[derive(Debug, PartialEq)]
pub enum CsrOp {
    Csrrw,
    Csrrs,
    Csrrc,
    Csrrwi,
    Csrrsi,
    Csrrci
}

pub fn parse_csr_inst(raw_word: InstructionWord) -> Result<Format, TrapCause> {
    let content = raw_word.0;
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    let rs1_or_uimm = mask_and_shift(content, masks::REG_SOURCE_ONE);
    let funct_three = mask_and_shift(content, masks::FUNCT_THREE);
    let csr_address = mask_and_shift(content, masks::CSR_ADDRESS);
    let instruction_name = match funct_three {
        0b001 => Ok(CsrOp::Csrrw),
        0b010 => Ok(CsrOp::Csrrs),
        0b011 => Ok(CsrOp::Csrrc),
        0b101 => Ok(CsrOp::Csrrwi),
        0b110 => Ok(CsrOp::Csrrsi),
        0b111 => Ok(CsrOp::Csrrci),
        _ => Err(TrapCause::IllegalInstruction { instruction: Some(content) })
    }?;
    Ok(Format::CsrType {
        op: instruction_name,
        rd: reg_dest as usize,
        rs1_or_uimm: rs1_or_uimm as usize,
        csr: csr_address as usize,
        cpu_mode: CPUMode::M
    })
}

pub fn execute_i_csr_type(op: &CsrOp, rd: usize, rs1_or_uimm: usize, csr_address: usize, register: &mut RegisterFile, csr: &mut CsrState, cpu_mode: &CPUMode) -> Result<ExecutionSignal, TrapCause> {
    match op {
        CsrOp::Csrrw => inst_i_csrrw(rd, rs1_or_uimm, csr_address, register, csr, *cpu_mode)?,
        CsrOp::Csrrs => inst_i_csrrs(rd, rs1_or_uimm, csr_address, register, csr, *cpu_mode)?,
        CsrOp::Csrrc => inst_i_csrrc(rd, rs1_or_uimm, csr_address, register, csr, *cpu_mode)?,
        CsrOp::Csrrwi => inst_i_csrrwi(rd, rs1_or_uimm as u32, csr_address, register, csr, *cpu_mode)?,
        CsrOp::Csrrsi => inst_i_csrrsi(rd, rs1_or_uimm as u32, csr_address, register, csr, *cpu_mode)?,
        CsrOp::Csrrci => inst_i_csrrci(rd, rs1_or_uimm as u32, csr_address, register, csr, *cpu_mode)?,
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_i_csrrw(rd: usize, rs1: usize, csr_address: usize, register: &mut RegisterFile, csr: &mut CsrState, cpu_mode: CPUMode) -> Result<(), TrapCause> {
    // t = CSR[csr]; CSR[csr] = rs1; rd = t
    // Per the instructions:
    // "If rd=x0, then the instruction shall not read the CSR and shall not cause any of the side effects that might occur on a CSR read."
    // https://docs.riscv.org/reference/isa/v20260120/unpriv/zicsr.html
    if rd != 0 {
        let old_val = csr.read(csr_address);
        register.write(rd, old_val);
    }
    let rs1_val = register.read(rs1);
    csr.write(csr_address, rs1_val, cpu_mode)?;
    Ok(())
}

pub fn inst_i_csrrs(rd: usize, rs1: usize, csr_address: usize, register: &mut RegisterFile, csr: &mut CsrState, cpu_mode: CPUMode) -> Result<(), TrapCause> {
    // t = CSR[csr]; CSR[csr] = t | rs1; rd = t
    // "Both CSRRS and CSRRC always read the addressed CSR and cause any read side effects regardless of rs1 and rd fields."
    let old_val = csr.read(csr_address);
    register.write(rd, old_val);
    if rs1 != 0 {
        let rs1_val = register.read(rs1);
        let masked_val = old_val | rs1_val;
        csr.write(csr_address, masked_val, cpu_mode)?;
    }
    Ok(())
}

pub fn inst_i_csrrc(rd: usize, rs1: usize, csr_address: usize, register: &mut RegisterFile, csr: &mut CsrState, cpu_mode: CPUMode) -> Result<(), TrapCause> {
    // t = CSR[csr]; CSR[csr] = t & !rs1; rd = t
    // "Both CSRRS and CSRRC always read the addressed CSR and cause any read side effects regardless of rs1 and rd fields."
    let old_val = csr.read(csr_address);
    register.write(rd, old_val);
    if rs1 != 0 {
        let rs1_val = register.read(rs1);
        let masked_val = old_val & !rs1_val;
        csr.write(csr_address, masked_val, cpu_mode)?;
    }
    Ok(())
}

pub fn inst_i_csrrwi(rd: usize, uimm: u32, csr_address: usize, register: &mut RegisterFile, csr: &mut CsrState, cpu_mode: CPUMode) -> Result<(), TrapCause> {
    // t = CSR[csr]; CSR[csr] = uimm; rd = t
    if rd != 0 {
        let old_val = csr.read(csr_address);
        register.write(rd, old_val);
    }
    csr.write(csr_address, uimm, cpu_mode)?;
    Ok(())
}

pub fn inst_i_csrrsi(rd: usize, uimm: u32, csr_address: usize, register: &mut RegisterFile, csr: &mut CsrState, cpu_mode: CPUMode) -> Result<(), TrapCause> {
    // t = CSR[csr]; CSR[csr] = t | uimm; rd = t
    //  "For CSRRSI and CSRRCI, if the uimm[4:0] field is zero, then these instructions will not write to the CSR"
    let old_val = csr.read(csr_address);
    register.write(rd, old_val);
    if uimm != 0 {
        let masked_val = old_val | uimm;
        csr.write(csr_address, masked_val, cpu_mode)?;
    }
    Ok(())
}

pub fn inst_i_csrrci(rd: usize, uimm: u32, csr_address: usize, register: &mut RegisterFile, csr: &mut CsrState, cpu_mode: CPUMode) -> Result<(), TrapCause> {
    // t = CSR[csr]; CSR[csr] = t & !uimm; rd = t
    //  "For CSRRSI and CSRRCI, if the uimm[4:0] field is zero, then these instructions will not write to the CSR"
    let old_val = csr.read(csr_address);
    register.write(rd, old_val);
    if uimm != 0 {
        let masked_val = old_val & !uimm;
        csr.write(csr_address, masked_val, cpu_mode)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::cpu_definition::build_register_file;
    use crate::definitions::cpu_definition::build_csr_state;

    #[test]
    fn test_parse_csr_inst_all_ops() {
        // All six CSR ops share the same field layout (rd = 1, rs1/uimm = 2,
        // csr = 0x300) and differ only by funct3, so one word per op --
        // built the same way (csr << 20 | rs1 << 15 | funct3 << 12 | rd << 7
        // | opcode) -- covers every branch of parse_csr_inst's match.
        //
        // 0x300110F3 = 0b0011_0000_0000_0001_0001_0000_1111_0011
        // csr[11:0]  bits 31:20 = 0011_0000_0000 = 0x300
        // rs1/uimm   bits 19:15 = 00010          = 2
        // funct3     bits 14:12 = 001                     (varies per op below)
        // rd         bits 11:7  = 00001          = 1
        // opcode     bits 6:0   = 1110011        = 0x73

        let expected = |op| Ok(Format::CsrType { op, rd: 1, rs1_or_uimm: 2, csr: 0x300, cpu_mode: CPUMode::M });

        assert_eq!(parse_csr_inst(InstructionWord(0x300110F3)), expected(CsrOp::Csrrw));  // funct3 = 001
        assert_eq!(parse_csr_inst(InstructionWord(0x300120F3)), expected(CsrOp::Csrrs));  // funct3 = 010
        assert_eq!(parse_csr_inst(InstructionWord(0x300130F3)), expected(CsrOp::Csrrc));  // funct3 = 011
        assert_eq!(parse_csr_inst(InstructionWord(0x300150F3)), expected(CsrOp::Csrrwi)); // funct3 = 101
        assert_eq!(parse_csr_inst(InstructionWord(0x300160F3)), expected(CsrOp::Csrrsi)); // funct3 = 110
        assert_eq!(parse_csr_inst(InstructionWord(0x300170F3)), expected(CsrOp::Csrrci)); // funct3 = 111
    }

    #[test]
    fn test_parse_csr_inst_invalid_funct3_returns_err() {
        // funct3 = 100 -- not a real CSR op (000 is ecall/ebreak, handled
        // upstream in parse_system_inst; 100 is simply undefined)
        let raw_word = InstructionWord(0x300140F3);
        let result = parse_csr_inst(raw_word);
        assert!(result.is_err());
    }

    // --- execution tests ---

    #[test]
    fn test_inst_i_csrrw() {
        // t = CSR[csr]; CSR[csr] = rs1; rd = t
        let mut register = build_register_file();
        let mut csr = build_csr_state();
        register.write(2, 55); // rs1's value
        csr.write(0x300, 100, CPUMode::M); // CSR's old value
        inst_i_csrrw(1, 2, 0x300, &mut register, &mut csr, CPUMode::M);
        assert_eq!(register.read(1), 100); // rd gets the old CSR value
        assert_eq!(csr.read(0x300), 55);   // CSR gets rs1's value
    }

    #[test]
    fn test_inst_i_csrrw_skips_register_write_when_rd_is_zero() {
        // "If rd=x0, then the instruction shall not read the CSR..."
        // The CSR write is unconditional regardless of rd.
        let mut register = build_register_file();
        let mut csr = build_csr_state();
        register.write(2, 42);  // rs1's value
        csr.write(0x300, 999, CPUMode::M);  // CSR's old value
        inst_i_csrrw(0, 2, 0x300, &mut register, &mut csr, CPUMode::M); // rd = 0
        assert_eq!(register.read(0), 0);   // x0 stays 0 -- write was skipped
        assert_eq!(csr.read(0x300), 42);   // CSR write still happens
    }

    #[test]
    fn test_inst_i_csrrs() {
        // t = CSR[csr]; CSR[csr] = t | rs1; rd = t
        let mut register = build_register_file();
        let mut csr = build_csr_state();
        register.write(2, 0b1100); // rs1 = bits to set
        csr.write(0x300, 0b0011, CPUMode::M);  // CSR's old value
        inst_i_csrrs(1, 2, 0x300, &mut register, &mut csr, CPUMode::M);
        assert_eq!(register.read(1), 0b0011);       // rd gets the old CSR value
        assert_eq!(csr.read(0x300), 0b1111);        // 0b0011 | 0b1100
    }

    #[test]
    fn test_inst_i_csrrc() {
        // t = CSR[csr]; CSR[csr] = t & !rs1; rd = t
        let mut register = build_register_file();
        let mut csr = build_csr_state();
        register.write(2, 0b0011); // rs1 = bits to clear
        csr.write(0x300, 0b1111, CPUMode::M);  // CSR's old value
        inst_i_csrrc(1, 2, 0x300, &mut register, &mut csr, CPUMode::M);
        assert_eq!(register.read(1), 0b1111);       // rd gets the old CSR value
        assert_eq!(csr.read(0x300), 0b1100);        // 0b1111 & !0b0011
    }

    #[test]
    fn test_inst_i_csrrwi() {
        // t = CSR[csr]; CSR[csr] = uimm; rd = t -- same as csrrw, but the
        // replacement value is a 5-bit immediate, not a register's contents
        let mut register = build_register_file();
        let mut csr = build_csr_state();
        csr.write(0x300, 100, CPUMode::M); // CSR's old value
        inst_i_csrrwi(1, 5, 0x300, &mut register, &mut csr, CPUMode::M); // uimm = 5
        assert_eq!(register.read(1), 100); // rd gets the old CSR value
        assert_eq!(csr.read(0x300), 5);    // CSR gets the immediate
    }

    #[test]
    fn test_inst_i_csrrwi_skips_register_write_when_rd_is_zero() {
        // same rd = x0 rule as csrrw
        let mut register = build_register_file();
        let mut csr = build_csr_state();
        csr.write(0x300, 999, CPUMode::M); // CSR's old value
        inst_i_csrrwi(0, 42, 0x300, &mut register, &mut csr, CPUMode::M); // rd = 0, uimm = 42
        assert_eq!(register.read(0), 0);   // x0 stays 0 -- write was skipped
        assert_eq!(csr.read(0x300), 42);   // CSR write still happens
    }

    #[test]
    fn test_inst_i_csrrsi() {
        // t = CSR[csr]; CSR[csr] = t | uimm; rd = t
        let mut register = build_register_file();
        let mut csr = build_csr_state();
        csr.write(0x300, 0b0011, CPUMode::M); // CSR's old value
        inst_i_csrrsi(1, 0b1100, 0x300, &mut register, &mut csr, CPUMode::M); // uimm = bits to set
        assert_eq!(register.read(1), 0b0011); // rd gets the old CSR value
        assert_eq!(csr.read(0x300), 0b1111);  // 0b0011 | 0b1100
    }

    #[test]
    fn test_inst_i_csrrci() {
        // t = CSR[csr]; CSR[csr] = t & !uimm; rd = t
        let mut register = build_register_file();
        let mut csr = build_csr_state();
        csr.write(0x300, 0b1111,CPUMode::M); // CSR's old value
        inst_i_csrrci(1, 0b0011, 0x300, &mut register, &mut csr, CPUMode::M); // uimm = bits to clear
        assert_eq!(register.read(1), 0b1111); // rd gets the old CSR value
        assert_eq!(csr.read(0x300), 0b1100);  // 0b1111 & !0b0011
    }
}
