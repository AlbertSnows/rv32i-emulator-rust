use crate::definitions::cpu::cpu_definition::{CPUState};
use crate::utility::bit_operations::{read_u32, read_u16};
use crate::definitions::trap_cause::{TrapCause};

// ELF = Executable and Linkable Format
// standard format for compiled unix programs
// gcc outputs elf
// ELF has three parts
// header
/// has magic bytes that communicate a bunch of information. 
/// specifically:
/// e_entry, the address execution starting point
/// e_phoff, e_phnum, e_phentsize - pointers to the rest of the 
//// file
// header table
/// list of segments. segments describe contiguous chunk of program.
pub const PT_LOAD: u32 = 1;
pub const SHT_SYMTAB: u32 = 2;

// segments exist for the loader
// these bytes go in memory
pub fn load_elf(elf_bytes: &[u8], cpu: &mut CPUState) -> Result<(), TrapCause> {
    let e_entry = read_u32(elf_bytes, 24); // pc start address
    let e_phoff = read_u32(elf_bytes, 28); // byte offset to the header table
    let e_phentsize = read_u16(elf_bytes, 42); // size of an entry
    let e_phnum = read_u16(elf_bytes, 44); // how many entries there are
    
    // Number of segments varies per ELF file. e_phnum lists how many segments there are
    // A segment is a contiguous chunk of the program. 
    for i in 0..e_phnum {
        let segment_start = e_phoff as usize;
        let segment_location = (i as usize) * (e_phentsize as usize);
        let current_segment_location = segment_start + segment_location;
        let p_type = read_u32(elf_bytes, current_segment_location);
        if p_type != PT_LOAD {
            continue;
        }
        let p_offset = read_u32(elf_bytes, current_segment_location + 4) as usize;
        let p_vaddr = read_u32(elf_bytes, current_segment_location + 8) as usize; // location to write to in memory 
        let p_filesz = read_u32(elf_bytes, current_segment_location + 16) as usize; // size of the segment in elf
        // p_memsz: always >= p_filez, indicates how large the segment is once in memory 
        let p_memsz = read_u32(elf_bytes, current_segment_location + 20) as usize; 

        cpu.mem.write_bytes(p_vaddr, &elf_bytes[p_offset..p_offset + p_filesz])?; // write elf data to mem
        // If the segment's memory size p_memsz is larger than the file size p_filesz, 
        /// the 'extra' bytes are defined to hold the value 0 and to follow the segment's initialized area.
        let zero_count = p_memsz - p_filesz;
        cpu.mem.write_bytes(p_vaddr + p_filesz, &vec![0u8; zero_count])?; 
    }
    cpu.pc.write(e_entry as usize);
    Ok(())
}

//               sh_name  sh_type  sh_flags  sh_addr  sh_offset  sh_size  sh_link  sh_info  sh_addralign  sh_entsize
// offset        0        4        8         12       16         20       24       28       32            36
// size (bytes)  4        4        4         4        4          4        4        4        4             4

// tohost is where test program writes pass/fail results
// address depends on linker
// find_symbol is finding where the pass/fail results where stored via tohost
// sections exist for tools. it tells you what part of the file you're working with 
pub fn find_symbol(elf_bytes: &[u8], symbol_name: &str) -> Option<u32> {
    // find the section header table (e_shoff, e_shnum, e_shentsize)
    let e_shoff = read_u32(elf_bytes, 32); // where file starts
    let e_shnum = read_u16(elf_bytes, 46); // how many sections are in the list
    let e_shentsize = read_u16(elf_bytes, 48); // how big the entry iss

    let tophost_location = 0;
    for i in 0..e_shnum {
        let section_start = e_shoff as usize;
        let section_location = (i as usize) * (e_shentsize as usize);
        let current_section_location = section_start + section_location;
        let sh_type = read_u32(elf_bytes, current_section_location + 4);
        if sh_type == SHT_SYMTAB {
            // todo: 
            tophost_location = i;
            break;
        }
    }
    let records = elf_bytes[tophost_location];
    for i in 0..records.len() {
        let record = records[i];
        let st_name = record["st_name"];
        if st_name == "tohost" {
            return Some(record["st_value"]);
        }
    }
    None

}