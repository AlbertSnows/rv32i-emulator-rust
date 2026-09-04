// Masks are sequences of bits used to separate bits from one another. 

const SEVEN_TO_ELEVEN: u32 = 0b1111_1000_0000;

// U/J type
const TWELVE_TO_THIRTY_ONE: u32 = 0b1111_1111_1111_1111_1111_0000_0000_0000;

const TWELVE_TO_FOURTEEN: u32 = 0b0000_0000_0000_0111_0000_0000_0000;
const FIFTEEN_TO_NINETEEN: u32 = 0b0000_0000_1111_1000_0000_0000_0000;

// I type
const TWENTY_TO_THIRTY_ONE: u32 = 0b1111_1111_1111_0000_0000_0000_0000_0000;


const TWENTY_TO_TWENTY_FOUR: u32 = 0b0001_1111_0000_0000_0000_0000_0000;
const TWENTY_FIVE_TO_THIRTY_ONE: u32 = 0b1111_1110_0000_0000_0000_0000_0000_0000;
const TWELVE_AND_ELEVEN: u32 = 0b1_1000_0000_0000;
const TWENTY_SEVEN_TO_THIRTY_ONE: u32 = 0b1111_1000_0000_0000_0000_0000_0000_0000;
const ZERO_TO_ELEVEN: u32 = 0b0000_0000_0000_0000_0000_1111_1111_1111; // bits 11:0 (0xFFF)
const THIRTY_ONE: u32 = 0b1000_0000_0000_0000_0000_0000_0000_0000; // bit 31
const ZERO_TO_TWENTY_ONE: u32 = 0b0000_0000_0011_1111_1111_1111_1111_1111; // bits 21:0
const TWENTY_TWO_TO_THIRTY_ONE: u32 = 0b1111_1111_1100_0000_0000_0000_0000_0000; // bits 31:22
const TWELVE_TO_TWENTY_ONE: u32 = 0b0000_0000_0011_1111_1111_0000_0000_0000; // bits 21:12
const TEN_TO_NINETEEN: u32 = 0b0000_0000_0000_1111_1111_1100_0000_0000; // bits 19:10

// RVC (compressed instruction) formats
const TWO_TO_FOUR: u32 = 0b0001_1100; // bits 4:2
const FIVE_TO_TWELVE: u32 = 0b0001_1111_1110_0000; // bits 12:5
const SEVEN_TO_NINE: u32 = 0b0000_0011_1000_0000; // bits 9:7
const ZERO_TO_ONE: u32 = 0b0000_0011; // bits 1:0
const TWO_TO_SIX: u32 = 0b0111_1100; // bits 6:2
const THIRTEEN_TO_FIFTEEN: u32 = 0b1110_0000_0000_0000; // bits 15:13

// 0xB3 = 1011 0011
// mask = 0111 1111 <- 7 bits
// yields 0011 0011
pub const OP_CODE: u32 = 0b0111_1111;
pub const REG_DESTINATION: u32 = SEVEN_TO_ELEVEN;
pub const U_TYPE_IMM: u32 = TWELVE_TO_THIRTY_ONE;
pub const J_TYPE_IMM: u32 = TWELVE_TO_THIRTY_ONE;
pub const FUNCT_THREE: u32 = TWELVE_TO_FOURTEEN;
pub const REG_SOURCE_ONE: u32 = FIFTEEN_TO_NINETEEN;
pub const I_TYPE_JALR: u32 = TWENTY_TO_THIRTY_ONE;
pub const B_TYPE_IMM_FIRST: u32 = SEVEN_TO_ELEVEN;
pub const REG_SOURCE_TWO: u32 = TWENTY_TO_TWENTY_FOUR;
pub const B_TYPE_IMM_SECOND: u32 = TWENTY_FIVE_TO_THIRTY_ONE;
pub const S_TYPE_IMM_FIRST: u32 = SEVEN_TO_ELEVEN;
pub const S_TYPE_IMM_SECOND: u32 = TWENTY_FIVE_TO_THIRTY_ONE;
pub const I_TYPE_SHAMT: u32 = TWENTY_TO_TWENTY_FOUR;
pub const FUNCT_SEVEN: u32 = TWENTY_FIVE_TO_THIRTY_ONE;
pub const I_TYPE_ALU_IMM: u32 = TWENTY_TO_THIRTY_ONE;
pub const I_TYPE_LOAD: u32 = TWENTY_TO_THIRTY_ONE;
pub const BIT_TWENTY: u32 = 0b1_0000_0000_0000_0000_0000;
pub const CSR_ADDRESS: u32 = TWENTY_TO_THIRTY_ONE;
pub const MPP: u32 = TWELVE_AND_ELEVEN;
pub const MPIE: u32 = 0b1000_0000;
pub const GLOBAL_MIE: u32 = 0b1000;
pub const MCAUSE_INTERRUPT: u32 = 0x8000_0000;
pub const MTI: u32 = 0b1000_0000;
pub const MTIP: u32 = MTI;
pub const MTIE: u32 = MTI;

pub const FUNCT_FIVE: u32 = TWENTY_SEVEN_TO_THIRTY_ONE;
pub const ACQUIRE: u32 = 0b0000_0100_0000_0000_0000_0000_0000_0000;
pub const RELEASE: u32 = 0b0000_0010_0000_0000_0000_0000_0000_0000;

pub const ONE: u32 = 0b10;
pub const FIVE: u32 = 0b10_0000;
pub const EIGHT: u32 = 0b1_0000_0000;
pub const NINE: u32 = 0b10_0000_0000;

// sstatus bit layout (RV32), a restricted view of mstatus (Section
// 12.1.1, Figure 17, p.112): every field below sits at the exact same
// bit position it occupies in mstatus itself, but most of mstatus's
// bits are WPRI (reserved/inaccessible) through this address -- S-mode
// only gets to see the subset of mstatus that's actually S-mode's
// business.
//
// |SD|WPRI |SDT|SPELP|WPRI |MXR|SUM|WPRI|XS   |FS   |WPRI |VS  |SPP|WPRI|UBE|SPIE|WPRI    |SIE|WPRI|
// |31|30:25|24 |23   |22:20|19 |18 |17  |16:15|14:13|12:11|10:9|8  |7   |6  |5   |4:2     |1  |0   |
//
// This codebase only wires up the subset actually needed: SIE/SPIE/SPP
// (S-mode CSR/trap-delegation, item #11) and SUM/MXR (Sv32 permission
// checks in the page-table walker). FS/XS/VS/SD are
// dirty-state tracking for extensions this codebase doesn't implement
// (F/D, custom, V) deliberately left WPRI/reserved here, not a gap.
// Same for UBE (switches U-mode accesses to big-endian, this
// codebase is little-endian throughout, top to bottom).
pub const SSTATUS: u32 = ONE | FIVE | EIGHT | MSTATUS_SUM | MSTATUS_MXR;
pub const PER_SOURCE_SIE: u32 = ONE | FIVE | NINE;
pub const SIP: u32 = PER_SOURCE_SIE;

pub const GLOBAL_SIE: u32 = ONE;
pub const SSIE: u32 = ONE;
pub const SSIP: u32 = ONE;
pub const SPIE: u32 = FIVE;
pub const STIE: u32 = FIVE;
pub const STIP: u32 = FIVE;

pub const SPP: u32 = EIGHT;
pub const SEIE: u32 = NINE;
pub const SEIP: u32 = NINE;
// 31        30            22 21                    0
// | MODE(1) |  ASID(9)      |      PPN(22)          |
pub const SATP_PPN: u32 = ZERO_TO_TWENTY_ONE;
pub const SATP_MODE: u32 = THIRTY_ONE;

// virt_addr's fields (Section 12.3.1, Figure 33, p.130) -- named to
// match the spec's own VPN[1]/VPN[0] indices, not just "first"/"second".
pub const VPN_ONE: u32 = TWENTY_TWO_TO_THIRTY_ONE;   //  indexes the root table
pub const VPN_ZERO: u32 = TWELVE_TO_TWENTY_ONE;      // indexes the leaf table
pub const VIRT_ADDR_OFFSET: u32 = ZERO_TO_ELEVEN;    //  untranslated, carried through

// A PTE's own fields (Section 12.3.1, Figure 35, p.130). Its page
// number is split into two pieces, PTE_PPN_ONE is the top 12 bits, PTE_PPN_ZERO the next 10;
// reconstructing the full 22-bit page number means combining them
// (PPN_ONE shifted up by 10, OR'd with PPN_ZERO).
//
//  31              20 19            10 9    8 7 6 5 4 3 2 1 0
// |   PPN[1] (12)     |  PPN[0] (10)   |RSW|D|A|G|U|X|W|R|V|
pub const PTE_PPN_ONE: u32 = TWENTY_TO_THIRTY_ONE; // bits 31:20
pub const PTE_PPN_ZERO: u32 = TEN_TO_NINETEEN;     // bits 19:10
pub const PTE_A: u32 = 0b100_0000; // bit 6, accessed
pub const PTE_D: u32 = 0b1000_0000; // bit 7, dirty
// MPRV ("Modify PRiVilege") -- lets M-mode's *data* accesses (loads and
// stores only, never instruction fetches) be checked as if issued by
// whatever privilege level MPP names, instead of M-mode's own real
// privilege. Used to let M-mode software (or, here, a conformance test)
// exercise page-table permission checks and A/D-bit tracking without
// actually leaving M-mode via mret/sret.
pub const MSTATUS_MPRV: u32 = 0b10_0000_0000_0000_0000; // bit 17
pub const MSTATUS_SUM: u32 = 0b100_0000000000000000; // bit 18
pub const MSTATUS_MXR: u32 = 0b1000_0000000000000000; // bit 19
// TVM ("Trap Virtual Memory") -- M-mode's lock barring S-mode from
// touching virtual-memory management at all: when set, S-mode
// executing sfence.vma or accessing satp must trap illegal-instruction
// instead of succeeding. M-mode only -- WPRI/reserved through sstatus,
// so this is checked against mstatus directly, never sstatus.
pub const MSTATUS_TVM: u32 = 0b1_0000_0000_0000_0000_0000; // bit 20
// TSR ("Trap SRET") -- the same shape of M-mode lock as TVM, but for
// SRET instead of sfence.vma/satp: when set, S-mode executing SRET
// must trap illegal-instruction instead of returning to whatever
// sepc/sstatus.SPP say to return to. Also M-mode only -- WPRI/reserved
// through sstatus, same reasoning as TVM.
pub const MSTATUS_TSR: u32 = 0b1_00_0000_0000_0000_0000_0000; // bit 22

pub const MEIP: u32 = 0b1000_0000_0000;
pub const MEIE: u32 = MEIP;

// RVC's compressed 3-bit register field (CIW/CL/CS/CA/CB formats) --
// only addresses the 8 "popular" registers, x8-x15; real register
// number is 8 + this field's value (see docs/plans/c_extension.md).
pub const C_REG: u32 = TWO_TO_FOUR;
// C.ADDI4SPN's own scrambled 8-bit immediate field, inst[12:5]. The
// bits inside still need reassembling in spec order
// (nzuimm[5|4|9|8|7|6|2|3]) -- this mask only isolates the whole field.
pub const C_ADDI4SPN_IMM: u32 = FIVE_TO_TWELVE;
// The compressed "base register" field used by CL/CS formats (C.LW's and
// C.SW's rs1'), at bits 9:7 -- a different position from C_REG's bits
// 4:2, but the same 8-popular-registers convention (real register = 8 +
// this field's value).
pub const C_REG_BASE: u32 = SEVEN_TO_NINE;
// Which of the 3 usable RVC quadrants (00/01/10) a compressed
// instruction belongs to -- the same funct3 value means a different
// instruction in each quadrant, so this has to be checked first.
pub const C_QUADRANT: u32 = ZERO_TO_ONE;
// The low 5 bits of quadrant 2's shift-amount field (C.SLLI's
// inst[6:2]) -- these map directly, in order, to shamt[4:0], unlike
// most other RVC immediates. Only shamt's top bit (inst[12]) needs
// separate handling.
pub const C_SHAMT_LOW: u32 = TWO_TO_SIX;
// Quadrant 2's full 5-bit register field at inst[6:2] -- same bits as
// C_SHAMT_LOW, but used where this field is a real register (C.SWSP's
// rs2, C.MV/C.ADD's rs2) rather than part of a shift amount. Full
// 32-register space, no x8-x15 remapping (quadrant 2 never remaps).
pub const C_REG_FULL: u32 = TWO_TO_SIX;
// RVC's own funct3 field, inst[15:13] -- NOT the same bits as the base
// 32-bit ISA's FUNCT_THREE (inst[14:12]). Reusing FUNCT_THREE here would
// silently extract the wrong 3 bits for every compressed instruction.
pub const C_FUNCT_THREE: u32 = THIRTEEN_TO_FIFTEEN;

pub const PER_SOURCE_MIP: u32 = SIP;
pub const PER_SOURCE_SIP: u32 = SIP;