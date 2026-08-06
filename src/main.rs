mod cpu_definition;
fn main() {
    println!("Hello, welcome to my emulation!");
    let cpu = cpu_definition::build_cpu_state();
    println!("{}", cpu.pc.value);
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_add() {
        assert_eq!(1+1, 2)
    }
}