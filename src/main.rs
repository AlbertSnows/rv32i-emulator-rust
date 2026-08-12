#![allow(unused)]
mod decoder;
mod fetcher;
mod instructions;
mod definitions;
mod utility;
mod programs;
mod core;
use crate::definitions::cpu_definition::build_cpu_state;
use decoder::decode_word_to_instruction;
use fetcher::{InstructionWord, fetch_word_from_memory};
use utility::bit_operations::store_in_mem;
use programs::helpers::basic_addition;
use crate::definitions::codes::ExecutionSignal;
use crate::instructions::pc::advance_pc;
use core::step;

fn main() {
    println!("Hello, welcome to my emulation!");
    let mut cpu = build_cpu_state();
    store_in_mem(&basic_addition(&mut cpu), &mut cpu.mem, 0);

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