use std::io::Read;
use std::sync::mpsc;
use std::thread;
use rv32i_emulator::cpu::core::step;
use rv32i_emulator::cpu::definitions::codes::ExecutionSignal;
use rv32i_emulator::cpu::definitions::cpu::cpu_definition::build_cpu_state;
use rv32i_emulator::loader::boot_kernel;
use rv32i_emulator::utility::host_io::read_stdin_loop;

fn main() {
    println!("Hello, welcome to my emulation!");
    let mut cpu = build_cpu_state();
    boot_kernel(&mut cpu).expect("kernel should boot");

    // tx = sending half, rx = receiving half
    // a channel is a thread safe queue for passing values between threads
    let (tx, rx) = mpsc::channel::<u8>();
    thread::spawn(move || read_stdin_loop(tx));

    let mut execution_outcome = ExecutionSignal::Continue;
    while execution_outcome == ExecutionSignal::Continue {
        // checks the channel for a waiting val
        if let Ok(byte) = rx.try_recv() {
            cpu.bus.receive_uart_byte(byte);
        }
        execution_outcome = step(&mut cpu).unwrap_or_else(|m| {
            println!("{:?}", m);
            ExecutionSignal::Halt
        })
    }
    println!("{:?}", cpu.register);
}

