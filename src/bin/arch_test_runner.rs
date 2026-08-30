use std::path::Path;
use std::process::exit;
use rv32i_emulator::core::step;
use rv32i_emulator::definitions::cpu::cpu_definition::build_cpu_state;
use rv32i_emulator::elf::{find_symbol, load_elf};

pub const MAX_ITERATIONS: u32 = 1_000_000;


fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let elf_path = Path::new(&args[1]); // todo: why?
    let elf_bytes = std::fs::read(elf_path).unwrap();
    let mut cpu = build_cpu_state();
    load_elf(&elf_bytes, &mut cpu).expect("load elf should succeed");
    let tohost_addr = find_symbol(&elf_bytes, "tohost").expect("tohost should resolve") as usize;
    let name = elf_path.file_name().unwrap().to_str().unwrap();
    for _ in 0..MAX_ITERATIONS {
        let _ = step(&mut cpu);
        let tohost_value = cpu.bus.direct_read(tohost_addr, 4).unwrap();
        if tohost_value != 0 {
            let tohost_h_value = cpu.bus.direct_read(tohost_addr + 4, 4).unwrap();
            let is_char = tohost_h_value == 0x01_01_00_00;
            if is_char {
                print!("{}", (tohost_value & 0xFF) as u8 as char);
                cpu.bus.direct_write(tohost_addr, &0u32.to_le_bytes()).expect("to host addr should not fail");
                cpu.bus.direct_write(tohost_addr + 4, &0u32.to_le_bytes()).expect("to host high addr should not fail");
            } else if tohost_h_value == 0 {
                let outcome = if tohost_value == 1 { "PASSED" } else { "FAILED" };
                println!("RVCP-SUMMARY: TEST {} - Test File \"{}\"", outcome, name);
                exit(if tohost_value == 1 { 0 } else { 1 });
            }
        }
    }
    println!("RVCP-SUMMARY: TEST FAILED - Test File \"{}\"", name);
    exit(1)
}
