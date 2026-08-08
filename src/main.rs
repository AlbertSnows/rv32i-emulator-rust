#![allow(unused)]
mod decoder;
mod fetcher;
mod instructions;
mod definitions;
mod utility;
mod programs;
use definitions::cpu_definition;
use decoder::decode_word_to_instruction;
use fetcher::InstructionWord;
use utility::bit_operations::store_in_mem;
use programs::basic_addition;
use fetcher::fetch_word_from_memory;
use crate::definitions::codes::ExecutionSignal;

fn main() {
    println!("Hello, welcome to my emulation!");
    let mut cpu = cpu_definition::build_cpu_state();
    cpu.register.write(1, 10);
    cpu.register.write(2, 7);
    let mem = &mut cpu.mem;
    let pc = &mut cpu.pc;
    store_in_mem(&basic_addition(), mem, 0);

    let mut execution_outcome = ExecutionSignal::Continue;
    while execution_outcome == ExecutionSignal::Continue {
        // mut allows cpu to change in the local scope
        let fetch_result = fetch_word_from_memory(&cpu.pc, &cpu.mem); // 51 = 0x33 = 0011 0011
        let raw_word = match fetch_result {
            Ok(rw) => rw,
            Err(m) => {
                println!("{}", m);
                ExecutionSignal::Halt
            }
        };
        let instruction_result = decode_word_to_instruction(raw_word);
        let instruction = match instruction_result {
            Ok(i) => i,
            Err(m) => {
                println!("{}", m);
                ExecutionSignal::Halt
            }
        };
        // &mut cpu passes a mutable reference to cpu
        // &mut cpu = this reference has "mutable" permission to cpu
        execution_outcome = match instruction.execute(&mut cpu) {
            Ok(r) => r,
            Err(e) => e
        };
        advance_pc(instruction, pc);
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