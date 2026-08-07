use crate::definitions::cpu_definition::RegisterFile;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;

#[derive(Debug, PartialEq)]
pub enum SystemOp {
    ECall,
    EBreak
}

#[derive(Debug, PartialEq)]
pub enum CsrType {
    Csrrw,
    Csrrs,
    Csrrc,
    Csrrwi,
    Csrrsi,
    Csrrci
}

pub fn parse_system_inst(raw_word: InstructionWord) -> Format {
    Format::SystemType {
        op: SystemOp::ECall
    }
}


pub fn execute_i_system_type(op: &SystemOp) {

}
