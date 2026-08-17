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
/// per entry:
pub const PT_LOAD: u32 = 1;

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
