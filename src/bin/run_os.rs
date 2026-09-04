use std::io::Read;
use std::sync::mpsc;
use std::thread;
use rv32i_emulator::cpu::core::step;
use rv32i_emulator::cpu::definitions::codes::ExecutionSignal;
use rv32i_emulator::cpu::definitions::cpu::cpu_definition::build_cpu_state;
use rv32i_emulator::loader::boot_kernel;

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

fn read_stdin_loop(tx: mpsc::Sender<u8>) {
    let stdin = std::io::stdin();
    for byte in stdin.lock().bytes() {
        match byte {
            Ok(b) => {
                // tx.send(b) pushes a byte to the channel, returns Result
                // if failure, stop iterating over loop, thread ends
                // only fails when rx fails
                // rx only fails when main loop stops running
                if tx.send(b).is_err() { break; }
            },
            Err(_) => break,
        }
    }
}