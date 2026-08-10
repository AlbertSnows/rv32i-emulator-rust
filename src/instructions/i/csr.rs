// CSR (Control and Status Register) instructions -- Zicsr extension,
//  31          20 19    15 14   12 11     7 6      0
// | csr[11:0]    |rs1/uimm| funct3 |   rd    | opcode |
// |     12       |   5    |   3    |    5    |   7    |
//
// rd <- CSR[csr] (old value, always read first); CSR[csr] <- new value
// rs1/uimm is a register index for csrrw/csrrs/csrrc, but a 5-bit

use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::cpu_definition::CsrState;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::definitions::codes::ExecutionSignal;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;

#[derive(Debug, PartialEq)]
pub enum CsrOp {
    Csrrw,
    Csrrs,
    Csrrc,
    Csrrwi,
    Csrrsi,
    Csrrci
}

pub fn parse_csr_inst(raw_word: InstructionWord) -> Result<Format, String> {
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
        _ => Err(format!("undefined CSR type"))
    }?;
    Ok(Format::CsrType {
        op: instruction_name,
        rd: reg_dest as usize,
        rs1_or_uimm: rs1_or_uimm as usize,
        csr: csr_address
    })
}

pub fn execute_i_csr_type(op: &CsrOp, rd: usize, rs1_or_uimm: usize, csr_address: u32, register: &mut RegisterFile, csr: &mut CsrState) -> Result<ExecutionSignal, String> {
    match op {
        CsrOp::Csrrw => inst_i_csrrw(rd, rs1_or_uimm, csr_address, register, csr),
        CsrOp::Csrrs => inst_i_csrrs(rd, rs1_or_uimm, csr_address, register, csr),
        CsrOp::Csrrc => inst_i_csrrc(rd, rs1_or_uimm, csr_address, register, csr),
        CsrOp::Csrrwi => inst_i_csrrwi(rd, rs1_or_uimm, csr_address, register, csr),
        CsrOp::Csrrsi => inst_i_csrrsi(rd, rs1_or_uimm, csr_address, register, csr),
        CsrOp::Csrrci => inst_i_csrrci(rd, rs1_or_uimm, csr_address, register, csr),
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_i_csrrw(rd: usize, rs1: usize, csr_address: u32, register: &mut RegisterFile, csr: &mut CsrState) {
    // t = CSR[csr]; CSR[csr] = rs1; rd = t
    // atomic read/write -- old CSR value goes to rd, rs1's value replaces it
    todo!()
}

pub fn inst_i_csrrs(rd: usize, rs1: usize, csr_address: u32, register: &mut RegisterFile, csr: &mut CsrState) {
    // t = CSR[csr]; CSR[csr] = t | rs1; rd = t
    // atomic read and set bits -- rs1 acts as a bitmask of bits to set
    todo!()
}

pub fn inst_i_csrrc(rd: usize, rs1: usize, csr_address: u32, register: &mut RegisterFile, csr: &mut CsrState) {
    // t = CSR[csr]; CSR[csr] = t & !rs1; rd = t
    // atomic read and clear bits -- rs1 acts as a bitmask of bits to clear
    todo!()
}

pub fn inst_i_csrrwi(rd: usize, uimm: usize, csr_address: u32, register: &mut RegisterFile, csr: &mut CsrState) {
    // t = CSR[csr]; CSR[csr] = uimm; rd = t
    // same as csrrw, but the replacement value is a 5-bit zero-extended
    // immediate instead of a register's contents
    todo!()
}

pub fn inst_i_csrrsi(rd: usize, uimm: usize, csr_address: u32, register: &mut RegisterFile, csr: &mut CsrState) {
    // t = CSR[csr]; CSR[csr] = t | uimm; rd = t
    // same as csrrs, but the set-bits mask is a 5-bit zero-extended immediate
    todo!()
}

pub fn inst_i_csrrci(rd: usize, uimm: usize, csr_address: u32, register: &mut RegisterFile, csr: &mut CsrState) {
    // t = CSR[csr]; CSR[csr] = t & !uimm; rd = t
    // same as csrrc, but the clear-bits mask is a 5-bit zero-extended immediate
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csr_inst_csrrw() {
        // csrrw x1, 0x300, x2 -- opcode = 1110011 (SYSTEM), funct3 = 001 (csrrw),
        // rd = 1, rs1 = 2, csr = 0x300 (mstatus, a common real CSR address)
        let raw_word = InstructionWord(0x300110F3);
        let result = parse_csr_inst(raw_word);
        assert_eq!(result, Ok(Format::CsrType { op: CsrOp::Csrrw, rd: 1, rs1_or_uimm: 2, csr: 0x300 }));
    }

    #[test]
    fn test_parse_csr_inst_csrrs() {
        // csrrs x1, 0x300, x2 -- same fields as above, funct3 = 010 (csrrs)
        let raw_word = InstructionWord(0x300120F3);
        let result = parse_csr_inst(raw_word);
        assert_eq!(result, Ok(Format::CsrType { op: CsrOp::Csrrs, rd: 1, rs1_or_uimm: 2, csr: 0x300 }));
    }

    #[test]
    fn test_parse_csr_inst_csrrc() {
        // csrrc x1, 0x300, x2 -- funct3 = 011 (csrrc)
        let raw_word = InstructionWord(0x300130F3);
        let result = parse_csr_inst(raw_word);
        assert_eq!(result, Ok(Format::CsrType { op: CsrOp::Csrrc, rd: 1, rs1_or_uimm: 2, csr: 0x300 }));
    }

    #[test]
    fn test_parse_csr_inst_csrrwi() {
        // csrrwi x1, 0x300, 2 -- funct3 = 101 (csrrwi), uimm = 2 sitting in
        // the same bit position rs1 occupies for the register-operand forms
        let raw_word = InstructionWord(0x300150F3);
        let result = parse_csr_inst(raw_word);
        assert_eq!(result, Ok(Format::CsrType { op: CsrOp::Csrrwi, rd: 1, rs1_or_uimm: 2, csr: 0x300 }));
    }

    #[test]
    fn test_parse_csr_inst_csrrsi() {
        // csrrsi x1, 0x300, 2 -- funct3 = 110 (csrrsi)
        let raw_word = InstructionWord(0x300160F3);
        let result = parse_csr_inst(raw_word);
        assert_eq!(result, Ok(Format::CsrType { op: CsrOp::Csrrsi, rd: 1, rs1_or_uimm: 2, csr: 0x300 }));
    }

    #[test]
    fn test_parse_csr_inst_csrrci() {
        // csrrci x1, 0x300, 2 -- funct3 = 111 (csrrci)
        let raw_word = InstructionWord(0x300170F3);
        let result = parse_csr_inst(raw_word);
        assert_eq!(result, Ok(Format::CsrType { op: CsrOp::Csrrci, rd: 1, rs1_or_uimm: 2, csr: 0x300 }));
    }

    #[test]
    fn test_parse_csr_inst_invalid_funct3_returns_err() {
        // funct3 = 100 -- not a real CSR op (000 is ecall/ebreak, handled
        // upstream in parse_system_inst; 100 is simply undefined)
        let raw_word = InstructionWord(0x300140F3);
        let result = parse_csr_inst(raw_word);
        assert!(result.is_err());
    }
}
