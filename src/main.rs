mod cpu_definition;
mod decoder;
mod fetcher;
mod instructions;
use decoder::decode_word_to_instruction;
use fetcher::InstructionWord;
fn main() {
    println!("Hello, welcome to my emulation!");
    let cpu = cpu_definition::build_cpu_state();
    let raw_word = InstructionWord(0x002081B3); // 51 = 0x33 = 0011 0011
    println!("{}", decode_word_to_instruction(raw_word));
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_add() {
        assert_eq!(1+1, 2)
    }
}