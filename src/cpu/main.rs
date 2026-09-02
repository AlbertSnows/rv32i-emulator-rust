#![allow(unused)]

use rv32i_emulator::cpu::definitions::cpu::cpu_definition::build_cpu_state;
use rv32i_emulator::cpu::decoder::decode_word_to_instruction;
use rv32i_emulator::cpu::fetcher::{InstructionWord, fetch_word_from_memory};
use rv32i_emulator::cpu::utility::bit_operations::store_in_mem;
use rv32i_emulator::cpu::programs::helpers::basic_addition;
use rv32i_emulator::cpu::definitions::codes::ExecutionSignal;
use rv32i_emulator::cpu::instructions::pc::advance_pc;
use rv32i_emulator::cpu::core::step;

fn main() {
    println!("Hello, welcome to my emulation!");
    let mut cpu = build_cpu_state();
    store_in_mem(&basic_addition(&mut cpu), &mut cpu.bus.ram, 0);

    let mut execution_outcome = ExecutionSignal::Continue;
    while execution_outcome == ExecutionSignal::Continue {
        execution_outcome = step(&mut cpu).unwrap_or_else(|m| {
            println!("{:?}", m);
            ExecutionSignal::Halt
        })
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