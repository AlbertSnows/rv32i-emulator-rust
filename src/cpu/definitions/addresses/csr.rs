use crate::cpu::definitions::cpu::csr::CsrAddress;

const fn csr(value: u16) -> CsrAddress {
    match CsrAddress::new(value) {
        Some(addr) => addr,
        None => panic!("CSR address must fit in 12 bits"),
    }
}

// CSR addresses
// status keeps track of the cpu's mode configurations
// Each is 12 bits, 3 hex digits, 4 bits apiece
// Bits [11:10] (R/W vs. read-only) and bits [9:8] (required privilege level)
// sit right next to each other, together spanning bits [11:8]
// All five CSRs below are read/write + M-only, i.e. 00(RW) + 11(M) = 0011 = 3,
// which is why every one of them starts with 3.
// 0xF11: 11(read-only) + 11(M) = 1111 = F
// The remaining two digits (bits [7:0]) don't carry a specified per-bit meaning
// they just give each CSR its own unique slot within its access/privilege class;
// mstatus bit layout (RV32). One 32-bit CSR, carved into many independent
// named fields at different bit positions -- touching one field (e.g. MPP)
// must not disturb any of the others sharing this same register. Laid out
// left (bit 31) to right (bit 0), same direction as every other diagram in
// this codebase; split across two lines since it doesn't fit in one.
//
// |SD|WPRI |SDT|SPELP|TSR|TW|TVM|MXR|SUM|MPRV|XS   |FS   |**MPP**|VS  |SPP|MPIE|UBE|SPIE|WPRI|MIE|WPRI|SIE|WPRI|
// |31|30:25|24 |23   |22 |21|20 |19 |18 |17  |16:15|14:13|12:11  |10:9|8  |7   |6  |5   |4   |3  |2   |1  |0   |
//
pub const MSTATUS: CsrAddress = csr(0x300);
pub const MTVEC: CsrAddress = csr(0x305);   // the address a trap jumps pc to (BASE field, Direct mode).
pub const MEPC: CsrAddress = csr(0x341);    // the pc of the instruction that trapped, saved for later resume.
pub const MCAUSE: CsrAddress = csr(0x342);  // a code identifying why the last trap happened (see TrapCause::mcause_code).
pub const MTVAL: CsrAddress = csr(0x343);   // extra trap-specific info: a faulting address, or illegal-instruction bits.

pub const CYCLE: CsrAddress = csr(0xC00); // 1100_0000_0000, shadows MCYCLE
pub const TIME: CsrAddress = csr(0xC01);
pub const INSTRET: CsrAddress = csr(0xC02); // shadows MINSTRET

pub const MCYCLE: CsrAddress = csr(0xB00);
pub const MINSTRET: CsrAddress = csr(0xB02);

// An interrupt is a signal that something happend that is unrelated to
// the cpu's current instruction.
// These contrast traps, which happen because of an instruction.
//
// mip / mie bit layout (RV32).
/// Both registers share the identical bit assignments
/// - bit i always means "interrupt source i", whether the
//    register is asking "is it pending" (mip) or "is it enabled" (mie).
// Even-numbered bits (and 15:14) are Reserved.
// Table 16, riscv_privleged.txt p.49-50.
//
// |Platform Use (custom)|Rsvd |LCOFI|Rsvd|MEI|Rsvd|SEI|Rsvd|MTI|Rsvd|STI|Rsvd|MSI|Rsvd|SSI|Rsvd|
// |31:16                |15:14|13   |12  |11 |10  |9  |8   |7  |6   |5  |4   |3  |2   |1  |0   |
// mie is about what we're willing to have interrupt us/willing to accept
// mei = machine external interrupt = device interrupting through plic
// mti = machine time interrupt = timer expired
//
pub const MIE: CsrAddress = csr(0x304);
// mip is about what's currently interrupting/what's pending
pub const MIP: CsrAddress = csr(0x344);

pub const MHARTID: CsrAddress = csr(0xF14);

// sstatus is is mstatus, but with only access priv to bits 1, 5, and 8
pub const SSTATUS: CsrAddress = csr(0x100);
// SIE is is MIE, but with only access priv to bits 1, 5, and 9
pub const SIE: CsrAddress = csr(0x104);
pub const STVEC: CsrAddress = csr(0x105);
pub const SSCRATCH: CsrAddress = csr(0x140);
pub const SEPC: CsrAddress = csr(0x141);
pub const SCAUSE: CsrAddress = csr(0x142);
pub const STVAL: CsrAddress = csr(0x143);
// SIP is is MIP, but with only access priv to bits 1, 5, and 9
pub const SIP: CsrAddress = csr(0x144);
pub const MEDELEG: CsrAddress = csr(0x302);
pub const MIDELEG: CsrAddress = csr(0x303);
pub const MISA: CsrAddress = csr(0x301);

pub const MCOUNTINHIBIT: CsrAddress = csr(0x320);

// address for who made it
pub const MVENDORID: CsrAddress = csr(0xF11);
// address for which architecture is in use
pub const MARCHID: CsrAddress = csr(0xF12);
// address of exact version of architecture
pub const MIMPID: CsrAddress = csr(0xF13);
pub const MINSTRETH: CsrAddress = csr(0xB82);
pub const INSTRETH: CsrAddress = csr(0xC82);
pub const MCYCLEH: CsrAddress = csr(0xB80);
pub const CYCLEH: CsrAddress = csr(0xC80);

pub const TSELECT: CsrAddress = csr(0x7a0);
pub const TDATA1: CsrAddress = csr(0x7a1);
pub const TDATA2: CsrAddress = csr(0x7a2);
pub const TCONTROL: CsrAddress = csr(0x7a5);
pub const MSCRATCH: CsrAddress = csr(0x340);
pub const MCOUNTEREN: CsrAddress = csr(0x306);
pub const SCOUNTNEREN: CsrAddress = csr(0x106);
pub const PMPCFG0: CsrAddress = csr(0x3A0);
pub const PMPADDR0: CsrAddress = csr(0x3B0);
// satp — a CSR, tells you where the tables are and whether translation is on:
// 31        30            22 21                    0
// | MODE(1) |  ASID(9)      |      PPN(22)          |
pub const SATP: CsrAddress = csr(0x180);

pub const MSTATUSH: CsrAddress = csr(0x310);

pub const TIMEH: CsrAddress = csr(0xC81);
