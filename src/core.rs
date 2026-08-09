use crate::definitions::cpu_definition::CPUState;
use crate::definitions::codes::ExecutionSignal;
use crate::fetcher::fetch_word_from_memory;
use crate::decoder::decode_word_to_instruction;
use crate::instructions::pc::advance_pc;

pub fn step(cpu: &mut CPUState) -> Result<ExecutionSignal, String> {
    // mut allows cpu to change in the local scope
    let raw_word = fetch_word_from_memory(&cpu.pc, &cpu.mem)?; // 51 = 0x33 = 0011 0011
    let instruction = decode_word_to_instruction(raw_word)?;
    // &mut cpu passes a mutable reference to cpu
    // &mut cpu = this reference has "mutable" permission to cpu
    let execution_outcome = instruction.execute(cpu)?;
    advance_pc(&mut cpu.pc, &instruction, &cpu.register);
    Ok(execution_outcome)
}