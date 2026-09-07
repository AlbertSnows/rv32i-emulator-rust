use crate::cpu::definitions::cpu::cpu_definition::CPUState;
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::utility::bit_operations::{read_u16, read_u32};
use crate::utility::bit_operations::read_string_until_terminator;
use crate::utility::types::ByteType;

// ELF = Executable and Linkable Format
// refer to the elf.pdf for more details about how elf files are defined
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
// the identifier for the symbol table
pub const SHT_SYMTAB: u32 = 2;

// ELF header (Elf32_Ehdr) field offsets, relative to the start of the
// file (docs/books/elf.pdf, Figure 1-3, p.1-4).
const E_ENTRY: usize = 24; // virtual address of the first instruction to run
const E_PHOFF: usize = 28; // where the program header table starts in the file
const E_SHOFF: usize = 32; // where the section header table starts in the file
const E_PHENTSIZE: usize = 42; // size in bytes of one program header entry
const E_SHENTSIZE: usize = 46; // size in bytes of one section header entry
const E_PHNUM: usize = 44; // how many program header entries there are
const E_SHNUM: usize = 48; // how many section header entries there are

// Program header (Elf32_Phdr) field offsets, relative to the start of
// one entry (docs/books/elf.pdf, Figure 2-1, p.2-2).
const P_TYPE: usize = 0; // what kind of segment this is -- only PT_LOAD ones get loaded
const P_OFFSET: usize = 4; // where this segment's bytes start in the file
const P_VADDR: usize = 8; // where those bytes should be placed in memory
const P_FILESZ: usize = 16; // how many bytes of the segment actually exist in the file
// how many bytes the segment occupies once loaded (>= FILESZ, remainder zero-filled)
const P_MEMSZ: usize = 20;
// Section header (Elf32_Shdr) field offsets, relative to the start of
// one entry (docs/books/elf.pdf, Figure 1-8, p.1-10).
const SH_NAME: usize = 0; // index into the string table for this section's own name
const SH_TYPE: usize = 4; // what kind of section this is -- used to find SHT_SYMTAB
const SH_OFFSET: usize = 16; // where this section's data starts in the file
const SH_SIZE: usize = 20; // total size in bytes of this section's data
const SH_LINK: usize = 24; // for a symtab section, the index of its associated string table section
const SH_ENTSIZE: usize = 36; // size of one record, when this section is an array of fixed-size records

// segments exist for the loader
// these bytes go in memory
pub fn load_elf(elf_bytes: &[u8], cpu: &mut CPUState, base_address: usize) -> Result<usize, TrapCause> {
    let header_table_location = read_u32(elf_bytes, E_PHOFF) as usize; // byte offset to the header table
    let header_entry_size = read_u16(elf_bytes, E_PHENTSIZE); // size of an entry
    let number_of_header_entries = read_u16(elf_bytes, E_PHNUM); // how many entries there are
    // Number of segments varies per ELF file. e_phnum lists how many segments there are
    // A segment is a contiguous chunk of the program.

    // Find every program header entry's location, keep only the
    // PT_LOAD ones. The rest (PT_NOTE, PT_DYNAMIC, etc.) don't describe
    // bytes the loader is responsible for placing in memory.
    let mut loadable_segment_locations = Vec::new();
    for header_entry_index in 0..number_of_header_entries {
        let segment_location = (header_entry_index as usize) * (header_entry_size as usize);
        let current_segment_location = header_table_location + segment_location;
        // p type identifies the kind of segment we're reading
        let p_type = read_u32(elf_bytes, current_segment_location + P_TYPE);
        // PT_LOAD: The array element specifies a loadable segment, described by p_filesz and
        // p_memsz. The bytes from the file are mapped to the beginning of the memory segment.
        if p_type == PT_LOAD {
            loadable_segment_locations.push(current_segment_location);
        }
    }

    let mut loaded_image_end = 0;
    // Copy each loadable segment's bytes into memory.
    for current_segment_location in loadable_segment_locations {
        // location to write to in memory
        let destination_address = read_u32(elf_bytes, current_segment_location + P_VADDR) as usize;
        // p_memsz: always >= p_filez, indicates how large the segment is once in memory
        let memory_byte_count = read_u32(elf_bytes, current_segment_location + P_MEMSZ) as usize;
        let segment_start = base_address + destination_address;
        let segment_end = segment_start + memory_byte_count;
        if segment_end > loaded_image_end {
            loaded_image_end = segment_end;
        }
        // size of the segment in elf
        let file_byte_count = read_u32(elf_bytes, current_segment_location + P_FILESZ) as usize;
        let file_offset = read_u32(elf_bytes, current_segment_location + P_OFFSET) as usize;
        let file_byte_range = file_offset..file_offset + file_byte_count;
        cpu.bus.direct_write(segment_start, &elf_bytes[file_byte_range])?; // write elf data to mem
        // If the segment's memory size p_memsz is larger than the file size p_filesz, 
        /// the 'extra' bytes are defined to hold the value 0 and to follow the segment's initialized area.
        let zero_padding_start = segment_start + file_byte_count;
        let zero_fill_count = memory_byte_count - file_byte_count;
        cpu.bus.direct_write(zero_padding_start, &vec![0u8; zero_fill_count])?;
    }

    let e_entry = read_u32(elf_bytes, E_ENTRY); // pc start address
    cpu.pc.write(base_address + e_entry as usize);
    Ok(loaded_image_end)
}

// tohost is where test program writes pass/fail results
// address depends on linker
// find_symbol is finding where the pass/fail results where stored via tohost
// sections exist for tools. it tells you what part of the file you're working with
// todo: refactor

// Walks the section header table looking for the one section whose
// sh_type is SHT_SYMTAB (In our files, there is only one).
//
// Elf32_Shdr (one section header entry, 40 bytes)
// byte:   0        4        8        12       16          20      24       28       32             36
// field: [sh_name][sh_type][sh_flags][sh_addr][sh_offset][sh_size][sh_link][sh_info][sh_addralign][sh_entsize]
fn find_symtab_metadata_location(elf_bytes: &[u8], file_start_location: usize, number_of_sections: usize, entry_size: usize) -> Option<usize> {
    for section_number in 0..number_of_sections {
        let section_offset = section_number * entry_size;
        let shdr_record_start = file_start_location + section_offset;
        let sh_type = read_u32(elf_bytes, shdr_record_start + SH_TYPE);
        if sh_type == SHT_SYMTAB {
            return Some(shdr_record_start);
        }
    }
    None
}

pub fn find_symbol(elf_bytes: &[u8], symbol_name: &str) -> Option<u32> {
    // find the section header table (e_shoff, e_shnum, e_shentsize)
    let file_start_location = read_u32(elf_bytes, E_SHOFF) as usize; // where file starts
    let number_of_sections = read_u16(elf_bytes, E_SHNUM) as usize; // how many sections are in the list
    let entry_size = read_u16(elf_bytes, E_SHENTSIZE) as usize; // how big the entry iss

    // symtab is the name=>address table
    // sht_symtab is the symbol table, the symbol table is an array of Elf32_Sym structs
    // we need to symbol table to get the symbol we're looking for
    let symbol_table_metadata_start = find_symtab_metadata_location(elf_bytes,
                                                                    file_start_location,
                                                                    number_of_sections,
                                                                    entry_size)?;


    // where the string table's own name text starts in the file
    // where symtab's own records start in the file
    let symbol_table_start = read_u32(elf_bytes, symbol_table_metadata_start + SH_OFFSET) as usize;
    // total size in bytes of all symtab records
    let symtab_data_size = read_u32(elf_bytes, symbol_table_metadata_start + SH_SIZE);
    // size of one record
    let symtab_record_size = read_u32(elf_bytes, symbol_table_metadata_start + SH_ENTSIZE);
    let symtab_end = symbol_table_start + symtab_data_size as usize;
    let symtab_record_indexes = (symbol_table_start..symtab_end).step_by(symtab_record_size as usize);

    // SH_LINK is only relevant to an Elf struct whose sh type is SHT_SYMTAB.
    // SH_LINK represents the section number of my paired string table.
    // which section holds symtab's associated names; an index, not a byte location
    let strtab_section_index = read_u32(elf_bytes, symbol_table_metadata_start + SH_LINK);
    let strtab_entry_offset = (strtab_section_index as usize) * (entry_size);
    // location of the str table's header entry
    let strtab_entry_location = file_start_location + strtab_entry_offset;
    let strtab_start = read_u32(elf_bytes, strtab_entry_location + SH_OFFSET);

    // Definition of an Elf32_Sym
    // byte:    0    1    2    3    4    5    6    7    8    9   10   11   12   13   14   15
    // field:  [------ st_name ------][------ st_value -----][--- st_size ---][info][oth][-shndx-]

    // symbtab holds the symbol table, e.g. 3 => <value we're looking for>
    // str tab holds the string identifier we're looking for, e.g. tohost => 3
    // thus we get tohost => 3 => <target value>
    // string table is just an array of bytes:
    // byte:     0    1    2    3    4    5    6    7    8    9   10   11   12  ...
    // data:    \0   't'  'o'  'h'  'o'  's'  't'  \0  'f'  'o'  'o'  \0  ...
    //     ^                                  ^                  ^
    //     offset 0                          offset 7           offset 12
    //     (empty string,                    "tohost"           "foo"
    //      conventional)
    for symtab_record_start in symtab_record_indexes {
        // st_name hold the offset into strtab's text data
        let name_offset_into_strtab = read_u32(elf_bytes, symtab_record_start);
        let name_location = (strtab_start + name_offset_into_strtab) as usize;
        let string_table_value = read_string_until_terminator(elf_bytes, name_location);
        let st_value = read_u32(elf_bytes, symtab_record_start + ByteType::Word.as_num());
        if string_table_value == symbol_name {
            return Some(st_value);
        }
    }
    None
}


