use crate::cpu::definitions::addresses::{MSTATUS, SATP};
use crate::cpu::definitions::cpu::bus::BUSState;
use crate::cpu::definitions::cpu::cpu_definition::CPUMode;
use crate::cpu::definitions::cpu::csr::CSRState;
use crate::cpu::definitions::cpu::memory::MemoryAccessType;
use crate::cpu::definitions::masks::{MSTATUS_SUM, PTE_A, PTE_D, PTE_PPN_ONE, PTE_PPN_ZERO,
                                SATP_MODE, SATP_PPN, VIRT_ADDR_OFFSET, VPN_ONE, VPN_ZERO};
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::cpu::utility::bit_operations::{extract_sub_bytes, mask_and_shift};
use crate::cpu::utility::types::ByteType;

// A page is 4096 bytes, it's forced by virt_addr's
// `offset` field being 12 bits wide (2^12 = 4096):
// a page has to be exactly as many bytes as `offset` can count
// through, or some offset values would point past the end of it.
const PAGESIZE: u32 = 12;

// Translates a virtual address into a physical one by walking the Sv32
// two-level page table, per the algorithm in riscv_privleged.pdf
// Section 12.3.2 ("Virtual Address Translation Process"), pp.133-134.
//
// Real operating
// systems don't let user programs see physical memory
// directly: each process is handed its own *virtual* address space, and
// the OS maintains a page table
// that maps each process's virtual addresses to the real physical
// addresses backing them. This is what makes per-process isolation,
// demand paging, and copy-on-write possible: two processes can use the
// identical virtual address for completely different physical memory,
// or share one physical page under two different virtual addresses,
// simply by pointing their page tables differently. None of that is
// representable if "the address" only ever means one thing.
//
// Sv32 (Section 12.3, p.129) is RISC-V's answer for a 32-bit hart:
// translation is only active when `satp.MODE` says so (Section 12.1.11,
// p.123) and only applies to S-mode/U-mode-effective accesses. M-mode
// by default still deals in physical addresses directly. When active,
// this function is the thing standing between "the CPU asked for
// address X" and "here's what's actually at X in RAM": it walks the
// two-level table (root table indexed by VPN[1], leaf table indexed by
// VPN[0]) and returns either the real physical address to use instead,
// or the specific page-fault `TrapCause` explaining why the access
// isn't allowed (missing mapping, wrong permissions, etc.).

// `virt_addr`'s shape (Section 12.3.1, Figure 33, p.130) -- this is the
// address the CPU actually asked for (a fetch/load/store target),
// *before* translation. It is not yet a real address into `bus`; it's
// just chopped into three pieces, none of which are computed from
// anything -- each is read straight off virt_addr's own bits.
//
//  31                    22 21                   12 11            0
// |      VPN[1] (10 bits)   |    VPN[0] (10 bits)    |  offset (12 bits) |
//
// VPN[1]/VPN[0] ("Virtual Page Number") are each used purely as an
// array index (0-1023) into a 1024-entry page table -- VPN[1] indexes
// the root table (found via satp), VPN[0] indexes whichever table that
// root entry points to. `offset` is never used as an index into
// anything; it's carried through unchanged into the final physical
// address, since translation only ever changes *which page* an address
// falls in, never the position within that page.
//
// `satp`'s shape (Section 12.1.11, Figure 31, p.123) -- this is where
// VPN[1]'s lookup actually starts: PPN is the root table's own page
// number (x4096 to get its real address), MODE says whether
// translation is even active, and ASID is unused here (no TLB to tag).
// todo: refactor
pub fn lookup_virt_to_phys(virt_addr: u32,
                           access_type: MemoryAccessType,
                           bus: &mut BUSState,
                           state: &CSRState,
                           mode: CPUMode) -> Result<u32, TrapCause> {
    let satp = state.read(SATP, CPUMode::M)?;
    let satp_mode = mask_and_shift(satp, SATP_MODE);
    if (satp_mode == 0) {
        return Ok(virt_addr);
    }
    // PPN = physical page numbers, 4096 byte chunks, this one is stored in satp
    let ppn = mask_and_shift(satp, SATP_PPN);
    let vpn_one = mask_and_shift(virt_addr, VPN_ONE);
    // Address composition, from Section 12.3.2, p.133, steps 1
    // and 2: "Let a be satp.ppn x PAGESIZE... Let pte be the value of
    // the PTE at address a + va.vpn[i] x PTESIZE." PAGESIZE=4096 bytes, turns a
    // page *number* into a real byte address; PTESIZE=4 turns a VPN
    // *index* into a byte offset within that table (each entry is 4
    // bytes).
    // the only thing that changes between the two lookups
    // is which PPN feeds it: satp's PPN for the first, the first
    // PTE's own PPN for the second.
    let table_one_addr = (ppn << PAGESIZE) + vpn_one * 4;
    // PTE = page table entry
    let pte_one = bus.direct_read(table_one_addr as usize, ByteType::Word.as_num())?;
    let vpn_zero = mask_and_shift(virt_addr, VPN_ZERO);
    let pte_ppn = mask_and_shift(pte_one, PTE_PPN_ONE | PTE_PPN_ZERO);
    let table_two_addr = (pte_ppn << PAGESIZE) + vpn_zero * 4;
    let offset = mask_and_shift(virt_addr, VIRT_ADDR_OFFSET);
    //  31              20 19             10|9     8 7 6 5 4 3 2 1 0
    // |   PPN[1] (12)     |  PPN[0] (10)   | RSW   |D|A|G|U|X|W|R|V|
    let v_bit = mask_and_shift(pte_one, 0b1);
    let w_bit = mask_and_shift(pte_one, 0b100);
    let r_bit = mask_and_shift(pte_one, 0b10);

    if (v_bit == 0 || (r_bit == 0 && w_bit == 1)) {
        return Err(page_fault(access_type, virt_addr));
    }
    let x_bit = mask_and_shift(pte_one, 0b1000);
    let mstatus = state.read(MSTATUS, CPUMode::M)?;
    let sum_bit = mask_and_shift(mstatus, MSTATUS_SUM);
    let sum_is_set = sum_bit == 1;
    let is_leaf = r_bit == 1 || x_bit == 1;
    if (is_leaf) {
        let u_bit = mask_and_shift(pte_one, 0b10000);
        if (mode == CPUMode::U && u_bit == 0) {
            return Err(page_fault(access_type, virt_addr));
        }
        let other_case = (mode == CPUMode::S) && u_bit == 1 && !sum_is_set;
        if (other_case) {
            return Err(page_fault(access_type, virt_addr));
        }

        let pte_zero = mask_and_shift(pte_one, PTE_PPN_ZERO);
        if (pte_zero != 0) {
            return Err(page_fault(access_type, virt_addr));
        }

        let a_bit = mask_and_shift(pte_one, PTE_A);
        let d_bit = mask_and_shift(pte_one, PTE_D);
        let should_update = a_bit == 0 || (access_type == MemoryAccessType::Store && d_bit == 0);
        if (should_update) {
            let updated_pte = if access_type == MemoryAccessType::Store {
                pte_one | PTE_A | PTE_D
            } else {
                pte_one | PTE_A
            };
            bus.direct_write(table_one_addr as usize, &updated_pte.to_le_bytes())?;
        }

        // let r_bit = 0b10
        // let x_bit = 0b1000
        let relevant_bit = mask_and_shift(pte_one, MemoryAccessType::to_pte_mask(access_type));
        let access_permitted = relevant_bit == 1;
        if (access_permitted) {
            let ppn_one = mask_and_shift(pte_one, PTE_PPN_ONE);
            Ok(ppn_one << 22 | vpn_zero << PAGESIZE | offset)
        } else {
            Err(page_fault(access_type, virt_addr))
        }
    } else {
        let pte_two = bus.direct_read(table_two_addr as usize, ByteType::Word.as_num())?;
        let v_bit = mask_and_shift(pte_two, 0b1);
        let w_bit = mask_and_shift(pte_two, 0b100);
        let r_bit = mask_and_shift(pte_two, 0b10);

        if (v_bit == 0 || (r_bit == 0 && w_bit == 1)) {
            return Err(page_fault(access_type, virt_addr));
        }
        let x_bit = mask_and_shift(pte_two, 0b1000);
        let is_leaf = r_bit == 1 || x_bit == 1;

        if (is_leaf) {
            let u_bit = mask_and_shift(pte_two, 0b10000);
            if (mode == CPUMode::U && u_bit == 0) {
                return Err(page_fault(access_type, virt_addr));
            }
            let other_case = (mode == CPUMode::S) && u_bit == 1 && !sum_is_set;
            if (other_case) {
                return Err(page_fault(access_type, virt_addr));
            }

            let a_bit = mask_and_shift(pte_two, PTE_A);
            let d_bit = mask_and_shift(pte_two, PTE_D);
            let should_update = a_bit == 0 || (access_type == MemoryAccessType::Store && d_bit == 0);
            if (should_update) {
                let updated_pte = if access_type == MemoryAccessType::Store {
                    pte_two | PTE_A | PTE_D
                } else {
                    pte_two | PTE_A
                };
                bus.direct_write(table_two_addr as usize, &updated_pte.to_le_bytes())?;
            }

            // let r_bit = 0b10
            // let w_bit = 0b100
            // let x_bit = 0b1000
            let relevant_bit = mask_and_shift(pte_two, MemoryAccessType::to_pte_mask(access_type));
            let access_permitted = relevant_bit == 1;
            if (access_permitted) {
                let pte_ppn = mask_and_shift(pte_two, PTE_PPN_ONE | PTE_PPN_ZERO);
                Ok(pte_ppn << PAGESIZE  | offset)
            } else {
                Err(page_fault(access_type, virt_addr))
            }
        } else {
            Err(page_fault(access_type, virt_addr))
        }
    }
}

fn page_fault(access_type: MemoryAccessType, virt_addr: u32) -> TrapCause {
    let addr = virt_addr as usize;
    match access_type {
        MemoryAccessType::Store => TrapCause::StorePageFault { address: addr },
        MemoryAccessType::Load => TrapCause::LoadPageFault { address: addr },
        MemoryAccessType::Fetch => TrapCause::InstructionPageFault { address: addr },
    }
}