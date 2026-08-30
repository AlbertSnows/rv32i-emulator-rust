use std::path::Path;
use rv32i_emulator::core::step;
use rv32i_emulator::definitions::cpu::cpu_definition::build_cpu_state;
use rv32i_emulator::elf::{find_symbol, load_elf};

pub const MAX_ITERATIONS: u32 = 10000;


fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let elf_path = Path::new(&args[1]); // todo: why?
    let elf_bytes = std::fs::read(elf_path).unwrap();
    let mut cpu = build_cpu_state();
    load_elf(&elf_bytes, &mut cpu).expect("load elf should succeed");
    let mut last_pc = cpu.pc.read();
    for _ in 0..MAX_ITERATIONS {
        let _ = step(&mut cpu);
        if last_pc == cpu.pc.read() {
            // stop
            break;
        } else {
            last_pc = cpu.pc.read();
        }
    }
    let tohost_addr = find_symbol(&elf_bytes, "tohost").expect("tohost should resolve") as usize;
    for _ in 0..MAX_ITERATIONS {
        let _ = step(&mut cpu);
        let tohost_value = cpu.bus.direct_read(tohost_addr, 4).unwrap();
        if tohost_value != 0 {
            println!("{}", if tohost_value == 1 { "PASS" } else { "FAIL" });
            break;
        }
    }

}
