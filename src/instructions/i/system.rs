use crate::definitions::cpu_definition::RegisterFile;

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

pub fn execute_i_system_type(op: &SystemOp) {

}
