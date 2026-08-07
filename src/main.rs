#![allow(unused)]
mod decoder;
mod fetcher;
mod instructions;
mod definitions;
mod utility;
use definitions::cpu_definition;
use decoder::decode_word_to_instruction;
use fetcher::InstructionWord;

fn main() {
    println!("Hello, welcome to my emulation!");
    // mut allows cpu to change in the local scope
    let mut cpu = cpu_definition::build_cpu_state();
    let raw_word = InstructionWord(0x002081B3); // 51 = 0x33 = 0011 0011
    let instruction = decode_word_to_instruction(raw_word);
    // &mut cpu passes a mutable reference to cpu
    // &mut cpu = this reference has "mutable" permission to cpu
    instruction.execute(&mut cpu);
    println!("{:?}", cpu.register);
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_add() {
        assert_eq!(1+1, 2)
    }
}