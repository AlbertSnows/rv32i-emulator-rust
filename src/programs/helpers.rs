use crate::definitions::cpu::cpu_definition::CPUState;
use crate::programs::instructions::ADD_X3_X1_X2;

pub fn basic_addition(cpu: &mut CPUState) -> [u8; 4] {
    cpu.register.write(1, 10);
    cpu.register.write(2, 7);
    ADD_X3_X1_X2.to_le_bytes()
}