use crate::cpu::definitions::addresses::{MSTATUS, SATP};
use crate::cpu::definitions::cpu::bus::BUSState;
use crate::cpu::definitions::cpu::cpu_definition::CPUMode;
use crate::cpu::definitions::cpu::csr::CSRState;
use crate::cpu::definitions::cpu::memory::MemoryAccessType;
use crate::cpu::definitions::masks::{MSTATUS_SUM, PTE_A, PTE_D, PTE_PPN_ONE, PTE_PPN_ZERO,
                                     PTE_R, PTE_U, PTE_V, PTE_W, PTE_X,
                                     SATP_MODE, SATP_PPN, VIRT_ADDR_OFFSET, VPN_ONE, VPN_ZERO};
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::utility::bit_operations::mask_and_shift;
use crate::utility::types::ByteType;

// A page is 4096 bytes, it's forced by virt_addr's
// `offset` field being 12 bits wide (2^12 = 4096):
// a page has to be exactly as many bytes as `offset` can count
// through, or some offset values would point past the end of it.
const PAGESIZE: u32 = 12;

// A page table is sized to fit in exactly one page: 1024 entries x 4
// bytes/entry = 4096 bytes, so allocating one needs no special-cased
// machinery beyond "give me one ordinary page". This is also *why*
// VPN[0]/VPN[1] (and, to match them, PPN[0]) are each 10 bits: indexing
// 1024 distinct entries needs exactly 10 bits (2^10 = 1024).
const PAGE_TABLE_SIZE: u32 = 1024;
// Bits needed to index PAGE_TABLE_SIZE entries -- the inverse of the
// 2^10 = 1024 relationship above. Used to shift PPN[1] up by exactly
// PPN[0]'s own width when recombining the two into one page number.
const PPN_ZERO_WIDTH: u32 = PAGE_TABLE_SIZE.ilog2(); // 10

// A superpage is 4 MiB (Section 12.3, p.135: "Sv32 supports 4 MiB
// megapages"), so ppn_one, for a superpage, counts in 4MiB units
// instead of 4KiB ones. Shifting by this many bits turns that count
// into a real byte address, the same way PAGESIZE does for an
// ordinary 4KiB page number.
const SUPERPAGE_SHIFT: u32 = PAGESIZE + PPN_ZERO_WIDTH; // 22

// The address the CPU actually asked for (a fetch/load/store target),
// before translation. Any u32 is a legal virtual address -- unlike
// CsrAddress, there's no invalid range to reject, so `new` is
// infallible. Fields are parsed once, up front, instead of
// re-extracted on every access. `raw` is kept alongside them since
// callers (e.g. page_fault) need the whole, untranslated address.
//
//  31                    22 21                   12 11            0
// |      VPN[1] (10 bits)   |    VPN[0] (10 bits)    |  offset (12 bits) |
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct VirtAddr {
    pub raw: u32,
    // Index into the root page table -- the first lookup, using satp's
    // own PPN to find where that table starts.
    pub root_index: u32,
    // Index into the second (leaf) page table -- the second lookup,
    // using the root entry's own PPN to find where that table starts.
    pub leaf_index: u32,
    // Position within whatever page this address falls in.
    pub physical_offset: u32,
}

impl VirtAddr {
    pub fn new(value: u32) -> Self {
        Self {
            raw: value,
            root_index: mask_and_shift(value, VPN_ONE),
            leaf_index: mask_and_shift(value, VPN_ZERO),
            physical_offset: mask_and_shift(value, VIRT_ADDR_OFFSET),
        }
    }
}

// One page table entry: the 4-byte value read from memory at a
// root/leaf entry address, describing where a virtual page's real data
// lives and what's allowed to be done with it (Section 12.3.1,
// Figure 35, p.130). `raw` is kept alongside the parsed fields since
// the code writes a *modified* version back (setting accessed/dirty).
// it that needs the original value to OR the new bits into. ppn_one
// and ppn_zero are kept separate, not combined into one page number,
// because the superpage case uses ppn_one alone and fills the other
// half from the virtual address's own leaf_index instead.
//
//  31              20 19            10 9    8 7 6 5 4 3 2 1 0
// |   PPN[1] (12)     |  PPN[0] (10)   |RSW|D|A|G|U|X|W|R|V|
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Pte {
    pub raw: u32,
    pub valid: bool, // v
    pub readable: bool, // r
    pub writable: bool, // w
    pub executable: bool, // x
    pub user_accessible: bool, // u
    pub accessed: bool, // a
    pub dirty: bool, // d
    pub ppn_one: u32,
    pub ppn_zero: u32,
}

impl Pte {
    pub fn new(raw: u32) -> Self {
        Self {
            raw,
            valid: mask_and_shift(raw, PTE_V) == 1,
            readable: mask_and_shift(raw, PTE_R) == 1,
            writable: mask_and_shift(raw, PTE_W) == 1,
            executable: mask_and_shift(raw, PTE_X) == 1,
            user_accessible: mask_and_shift(raw, PTE_U) == 1,
            accessed: mask_and_shift(raw, PTE_A) == 1,
            dirty: mask_and_shift(raw, PTE_D) == 1,
            ppn_one: mask_and_shift(raw, PTE_PPN_ONE),
            ppn_zero: mask_and_shift(raw, PTE_PPN_ZERO),
        }
    }
}

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
pub fn lookup_virt_to_phys(virt_addr: VirtAddr,
                           access_type: MemoryAccessType,
                           bus: &mut BUSState,
                           state: &CSRState,
                           mode: CPUMode) -> Result<u32, TrapCause> {
    let satp = state.read(SATP, CPUMode::M)?;
    let satp_mode = mask_and_shift(satp, SATP_MODE);
    if (satp_mode == 0) {
        return Ok(virt_addr.raw);
    }
    // PPN = physical page numbers, 4096 byte chunks, this one is stored in satp
    let root_table_page_number = mask_and_shift(satp, SATP_PPN);

    // Address composition, from Section 12.3.2, p.133, steps 1
    // and 2: "Let a be satp.ppn x PAGESIZE... Let pte be the value of
    // the PTE at address a + va.vpn[i] x PTESIZE." PAGESIZE=4096 bytes, turns a
    // page *number* into a real byte address; PTESIZE=4 turns a VPN
    // *index* into a byte offset within that table (each entry is 4
    // bytes).
    // the only thing that changes between the two lookups
    // is which PPN feeds it: satp's PPN for the first, the first
    // PTE's own PPN for the second.
    let root_table_start_location = (root_table_page_number << PAGESIZE);
    let root_entry_byte_offset = virt_addr.root_index * (ByteType::Word.as_num() as u32);
    let root_entry_addr = root_table_start_location + root_entry_byte_offset;

    // PTE = page table entry
    // the root table entry points to the relevant leaf table entry
    let root_pte = Pte::new(bus.direct_read(
        root_entry_addr as usize,
        ByteType::Word.as_num())?);
    let leaf_table_page_number = (root_pte.ppn_one << PPN_ZERO_WIDTH) | root_pte.ppn_zero;
    let leaf_table_start_location = leaf_table_page_number << PAGESIZE;
    let leaf_entry_byte_offset = virt_addr.leaf_index * ByteType::Word.as_num() as u32;
    let leaf_entry_addr = leaf_entry_byte_offset + leaf_table_start_location;

    if (!root_pte.valid || (!root_pte.readable && root_pte.writable)) {
        return Err(page_fault(access_type, virt_addr.raw));
    }

    // If the kernel needs to touch a user page, the sum bit
    // in mstatus must be set.
    // a user page as U=1, bit 4 of PTE
    // from the book:
    // "The SUM (permit Supervisor User Memory access) bit modifies the privilege with which
    // S-mode loads and stores access virtual memory. When SUM=0, S-mode memory accesses to
    // pages that are accessible by U-mode (U=1 in Sv32 page table entry) will fault. When
    // SUM=1, these accesses are permitted."
    let mstatus = state.read(MSTATUS, CPUMode::M)?;
    let sum_bit = mask_and_shift(mstatus, MSTATUS_SUM);
    let sum_is_set = sum_bit == 1;

    // "The permission bits, R, W, and X, indicate whether the page is readable, writable,
    // and executable, respectively. When all three are zero, the PTE is a pointer to the
    // next level of the page table; otherwise, it is a leaf PTE. Writable pages must also
    // be marked readable; the contrary combinations are reserved for future use."
    let is_leaf = root_pte.readable || root_pte.executable;
    if (is_leaf) {
        // "[When] a leaf PTE has been reached, if i>0 and pte.ppn[i-1:0] ≠ 0, this is a misaligned
        // superpage; stop and raise a page-fault exception."
        let pte_ppn_is_zero = root_pte.ppn_zero != 0;
        if (pte_ppn_is_zero) {
            return Err(page_fault(access_type, virt_addr.raw));
        }

        finish_leaf_translation(
            root_pte,
            root_entry_addr,
            root_pte.ppn_one,
            SUPERPAGE_SHIFT,
            virt_addr.leaf_index << PAGESIZE | virt_addr.physical_offset,
            virt_addr.raw,
            access_type,
            mode,
            sum_is_set,
            bus,
        )
    } else {
        let leaf_pte = Pte::new(bus.direct_read(leaf_entry_addr as usize, ByteType::Word.as_num())?);

        if (!leaf_pte.valid || (!leaf_pte.readable && leaf_pte.writable)) {
            return Err(page_fault(access_type, virt_addr.raw));
        }
        let is_leaf = leaf_pte.readable || leaf_pte.executable;

        if (is_leaf) {
            let leaf_page_number = (leaf_pte.ppn_one << PPN_ZERO_WIDTH) | leaf_pte.ppn_zero;
            finish_leaf_translation(
                leaf_pte,
                leaf_entry_addr,
                leaf_page_number,
                PAGESIZE,
                virt_addr.physical_offset,
                virt_addr.raw,
                access_type,
                mode,
                sum_is_set,
                bus,
            )
        } else {
            Err(page_fault(access_type, virt_addr.raw))
        }
    }
}

// Stitches a page number and the bits below it into one physical
// address. `shift` is how far up `page_number` needs to move to clear
// out room for everything below it, PAGESIZE for an ordinary page,
// SUPERPAGE_SHIFT for a superpage. `low_bits` is whatever fills that
// cleared-out space: just the offset for an ordinary page, or
// leaf_index and offset combined for a superpage.
fn build_physical_address(page_number: u32, shift: u32, low_bits: u32) -> u32 {
    (page_number << shift) | low_bits
}

// The part of translating a leaf PTE that's identical whether it was
// reached as a superpage (root level) or an ordinary page (leaf
// level): permission checks, the accessed/dirty write-back, and
// building the final address. `page_number`/`shift`/`low_bits` are
// exactly `build_physical_address`'s own arguments, chosen by the
// caller to match which case this is. `pte_addr` is where `pte` itself
// was read from, needed only if it has to be written back.
fn finish_leaf_translation(
    pte: Pte,
    pte_addr: u32,
    page_number: u32,
    shift: u32,
    low_bits: u32,
    virt_addr_raw: u32,
    access_type: MemoryAccessType,
    mode: CPUMode,
    sum_is_set: bool,
    bus: &mut BUSState,
) -> Result<u32, TrapCause> {
    let not_user_accessible = mode == CPUMode::U && !pte.user_accessible;
    let supervisor_denied_user_page = (mode == CPUMode::S) && pte.user_accessible && !sum_is_set;
    if not_user_accessible || supervisor_denied_user_page {
        return Err(page_fault(access_type, virt_addr_raw));
    }

    // indicates that the hardware needs to set one of these bits itself
    // accessed = first touch, store & dirty = first write
    let should_update = !pte.accessed
        || (access_type == MemoryAccessType::Store && !pte.dirty);
    if should_update {
        let updated_pte = if access_type == MemoryAccessType::Store {
            pte.raw | PTE_A | PTE_D
        } else {
            pte.raw | PTE_A
        };
        bus.direct_write(pte_addr as usize, &updated_pte.to_le_bytes())?;
    }

    // "Determine if the requested memory access is allowed by the pte.r, pte.w,
    //  and pte.x bits."
    let access_permitted = match access_type {
        MemoryAccessType::Load => pte.readable,
        MemoryAccessType::Store => pte.writable,
        MemoryAccessType::Fetch => pte.executable,
    };
    if access_permitted {
        Ok(build_physical_address(page_number, shift, low_bits))
    } else {
        Err(page_fault(access_type, virt_addr_raw))
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