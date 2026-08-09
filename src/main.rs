#![allow(unused)]
mod decoder;
mod fetcher;
mod instructions;
mod definitions;
mod utility;
mod programs;
mod core;
use definitions::cpu_definition;
use decoder::decode_word_to_instruction;
use fetcher::InstructionWord;
use utility::bit_operations::store_in_mem;
use programs::basic_addition;
use fetcher::fetch_word_from_memory;
use crate::definitions::codes::ExecutionSignal;
use crate::instructions::pc::advance_pc;
use core::step;
fn main() {
    println!("Hello, welcome to my emulation!");
    let mut cpu = cpu_definition::build_cpu_state();
    store_in_mem(&basic_addition(&mut cpu), &mut cpu.mem, 0);

    let mut execution_outcome = ExecutionSignal::Continue;
    while execution_outcome == ExecutionSignal::Continue {
        execution_outcome = match step(&mut cpu) {
            Ok(signal) => signal,
            Err(m) => {
                println!("{}", m);
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