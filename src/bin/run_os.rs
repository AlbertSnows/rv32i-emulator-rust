use rv32i_emulator::cpu::core::step;
use rv32i_emulator::cpu::definitions::codes::ExecutionSignal;
use rv32i_emulator::cpu::definitions::cpu::cpu_definition::build_cpu_state;
use rv32i_emulator::loader::boot_kernel;

fn main() {
    println!("Hello, welcome to my emulation!");
    let mut cpu = build_cpu_state();
    boot_kernel(&mut cpu).expect("kernel should boot");

    let mut execution_outcome = ExecutionSignal::Continue;
    while execution_outcome == ExecutionSignal::Continue {
        execution_outcome = step(&mut cpu).unwrap_or_else(|m| {
            println!("{:?}", m);
            ExecutionSignal::Halt
        })
    }
    println!("{:?}", cpu.register);
}