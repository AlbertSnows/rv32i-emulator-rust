use rv32i_emulator::definitions::cpu::cpu_definition::build_cpu_state;
use rv32i_emulator::definitions::cpu::bus::BASE_ADDRESS;
use rv32i_emulator::elf::{find_symbol, load_elf};

// A ELF produced by riscv-none-elf-gcc from riscv-tests' rv32ui/add.S.
// Its own load address (e_entry = 0x80000000) is far beyond 
// (4096, in memory.rs), so it's only usable for find_symbol -- which just
// reads metadata out of the byte buffer and never touches cpu.mem -- not
// for load_elf, which does.
const ADD_P_ADD: &[u8] = include_bytes!("fixtures/add-p-add");

#[test]
fn test_find_symbol_locates_tohost() {
    let result = find_symbol(ADD_P_ADD, "tohost");
    assert_eq!(result, Some(0x80001000));
}

#[test]
fn test_find_symbol_returns_none_for_missing_symbol() {
    let result = find_symbol(ADD_P_ADD, "not_a_real_symbol");
    assert_eq!(result, None);
}

// minimal ELF32: header + one PT_LOAD program header entry +
// 4 bytes of segment data, with p_memsz (8) larger than p_filesz (4)
// so it exercises the .bss zero-fill path 
// none of riscv-tests' rv32ui
// binaries actually have a real bss gap (all have
// FileSiz == MemSiz), so this can't be tested against a real compiled file.
fn build_synthetic_elf_with_bss() -> Vec<u8> {
    let mut bytes = vec![0u8; 88];

    bytes[24..28].copy_from_slice(&BASE_ADDRESS.to_le_bytes()); // e_entry
    bytes[28..32].copy_from_slice(&52u32.to_le_bytes()); // e_phoff
    bytes[42..44].copy_from_slice(&32u16.to_le_bytes()); // e_phentsize
    bytes[44..46].copy_from_slice(&1u16.to_le_bytes()); // e_phnum

    bytes[52..56].copy_from_slice(&1u32.to_le_bytes()); // p_type = PT_LOAD
    bytes[56..60].copy_from_slice(&84u32.to_le_bytes()); // p_offset
    bytes[60..64].copy_from_slice(&BASE_ADDRESS.to_le_bytes()); // p_vaddr
    bytes[64..68].copy_from_slice(&BASE_ADDRESS.to_le_bytes()); // p_paddr
    bytes[68..72].copy_from_slice(&4u32.to_le_bytes()); // p_filesz
    bytes[72..76].copy_from_slice(&8u32.to_le_bytes()); // p_memsz -- 4 bytes bigger than p_filesz

    bytes[84..88].copy_from_slice(&[0x11, 0x22, 0x33, 0x44]); // the real segment data

    bytes
}

#[test]
fn test_load_elf_sets_pc_to_entry() {
    let mut cpu = build_cpu_state();
    let elf_bytes = build_synthetic_elf_with_bss();
    load_elf(&elf_bytes, &mut cpu).expect("load_elf should succeed");
    assert_eq!(cpu.pc.read(), BASE_ADDRESS as usize);
}

#[test]
fn test_load_elf_copies_segment_bytes_into_memory() {
    let mut cpu = build_cpu_state();
    let elf_bytes = build_synthetic_elf_with_bss();
    load_elf(&elf_bytes, &mut cpu).expect("load_elf should succeed");
    assert_eq!(cpu.bus.direct_read(BASE_ADDRESS as usize, 4).unwrap(), 0x44332211);
}

#[test]
fn test_load_elf_zero_fills_bss_gap() {
    let mut cpu = build_cpu_state();
    let elf_bytes = build_synthetic_elf_with_bss();
    load_elf(&elf_bytes, &mut cpu).expect("load_elf should succeed");
    assert_eq!(cpu.bus.direct_read(BASE_ADDRESS as usize + 4, 4).unwrap(), 0);
}
