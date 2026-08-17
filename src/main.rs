#![allow(unused)]

use rv32i_emulator::definitions::cpu::cpu_definition::build_cpu_state;
use rv32i_emulator::decoder::decode_word_to_instruction;
use rv32i_emulator::fetcher::{InstructionWord, fetch_word_from_memory};
use rv32i_emulator::utility::bit_operations::store_in_mem;
use rv32i_emulator::programs::helpers::basic_addition;
use rv32i_emulator::definitions::codes::ExecutionSignal;
use rv32i_emulator::instructions::pc::advance_pc;
use rv32i_emulator::core::step;

fn main() {
    println!("Hello, welcome to my emulation!");
    let mut cpu = build_cpu_state();
    store_in_mem(&basic_addition(&mut cpu), &mut cpu.bus.ram, 0);

    let mut execution_outcome = ExecutionSignal::Continue;
    while execution_outcome == ExecutionSignal::Continue {
        execution_outcome = match step(&mut cpu) {
            Ok(signal) => signal,
            Err(m) => {
                println!("{:?}", m);
                ExecutionSignal::Halt
            }
        }
    }
    println!("{:?}", cpu.register);
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_add() {
        assert_eq!(1+1, 2)
    }
}