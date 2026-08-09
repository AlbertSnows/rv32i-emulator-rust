use crate::definitions::cpu_definition::CPUState;
pub fn basic_addition(cpu: &mut CPUState) -> [u8; 4] {
    cpu.register.write(1, 10);
    cpu.register.write(2, 7);
    [0xB3, 0x81, 0x20, 0x00]
}