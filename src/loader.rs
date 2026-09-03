use crate::cpu::definitions::cpu::cpu_definition::CPUState;
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::cpu::elf::load_elf;
use std::path::Path;
use crate::utility::bit_operations::read_u64;

// trying to copy kernel image into emulator memory
pub fn load_sbi(open_sbi_path: &Path, cpu: &mut CPUState) -> Result<usize, TrapCause> {
    let open_sbi_bytes = std::fs::read(open_sbi_path).unwrap();
    let sbi_end = load_elf(&open_sbi_bytes, cpu)?;
    let a0 = 10;
    cpu.register.write(a0, 0);
    Ok(sbi_end)
}

pub fn boot_kernel(cpu: &mut CPUState) -> Result<(), TrapCause> {
    let sbi_path = "build/platform/generic/firmware/fw_dynamic.elf";
    let kernel_path = "path/to/Image";
    let dtb_location = "path/to/your.dtb";
    let sbi_path = "build/platform/generic/firmware/fw_dynamic.elf";
    let open_sbi_end = load_sbi(sbi_path.as_ref(), cpu)?;
    let (kernel_start, kernel_size) = load_kernel(kernel_path, open_sbi_end as u32, cpu)?;
    let (dtb_start, dtb_size) = load_dtb(kernel_start, kernel_size, dtb_location, cpu)?;
    let fw_dyn_addr = build_fw_dynamic_info(dtb_start, dtb_size, kernel_start, cpu)?;

    let a1 = 11;
    let a2 = 12;
    cpu.register.write(a1, dtb_start as u32);
    cpu.register.write(a2, fw_dyn_addr as u32);
    Ok(())
}

fn build_fw_dynamic_info(dtb_start: usize, dtb_size: usize, kernel_start: usize, cpu: &mut CPUState)
    -> Result<usize, TrapCause> {
    let write_location = dtb_start + dtb_size;
    let contents_to_write = [
        0x4942534fu32.to_le_bytes(),
        0x2u32.to_le_bytes(),
        (kernel_start as u32).to_le_bytes(),
        0x1u32.to_le_bytes(),
        0u32.to_le_bytes(),
        0u32.to_le_bytes()
    ];
    for (i, bytes) in contents_to_write.iter().enumerate() {
        cpu.bus.direct_write(write_location + i * 4, bytes)?;
    }
    Ok(write_location)
}

fn load_dtb(kernel_start: usize, kernel_size: usize, dtb_location: &str, cpu: &mut CPUState)
    -> Result<(usize, usize), TrapCause> {
    let dtb_bytes = std::fs::read(dtb_location).unwrap();
    let dtb_start = kernel_start + kernel_size;
    cpu.bus.direct_write(dtb_start, &dtb_bytes)?;
    Ok((dtb_start, dtb_bytes.len(),))
}

// https://github.com/torvalds/linux/blob/master/Documentation/arch/riscv/boot-image-header.rst
// text offset is the 3rd entry, after two u32's, each is a word in size, so the starting location
// of text offset location is 4 + 4 = 8
const TEXT_OFFSET_LOCATION: u32 = 8;

fn load_kernel(kernel_path: &str, open_sbi_end: u32, cpu: &mut CPUState)
    -> Result<(usize, usize), TrapCause> {
    let kernel_bytes = std::fs::read(kernel_path).unwrap();
    let text_offset = read_u64(&kernel_bytes, TEXT_OFFSET_LOCATION as usize);
    let mib = 1024 * 1024;
    let kernel_start = align_up(open_sbi_end as usize, 4 * mib) + (text_offset as usize);
    cpu.bus.direct_write(kernel_start, &kernel_bytes)?;
    Ok((kernel_start, kernel_bytes.len()))
}

/// Rounds `addr` up to the nearest multiple of `alignment`.
///
/// Returns the smallest value that is both >= `addr` and evenly divisible
/// by `alignment`. If `addr` is already a multiple of `alignment`, it is
/// returned unchanged.
///
/// Used to place the next piece of loaded data (e.g. the kernel) on a
/// clean address boundary without ever overlapping whatever came before
/// it in memory (e.g. OpenSBI) -- rounding up guarantees the result never
/// falls before `addr`, only at or after it.
// end_addr: where the end of the last piece of data was stored
fn align_up(addr: usize, alignment: usize) -> usize {
    // the -1 prevents the probe from moving 2 multiples over if addr is already a mult of align
    let probing_addr = (addr + alignment - 1);
    let alignment_groups = probing_addr / alignment;
    let nearest_multiple = alignment_groups * alignment;
    nearest_multiple
}