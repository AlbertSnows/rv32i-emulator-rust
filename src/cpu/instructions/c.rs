// Quadrant 0 of the RVC ("C") extension — see docs/plans/c_extension.md
// and .claude/riscv-unprivileged.txt Chapter 28 for the full spec.

use crate::cpu::definitions::masks;
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::cpu::fetcher::Instruction;
use crate::cpu::instructions::Format;
use crate::cpu::instructions::i::alu_imm_or_shift::AluImmOp;
use crate::cpu::instructions::s::SOp;
use crate::cpu::instructions::i::load::LoadOp;
use crate::utility::bit_operations::{mask, mask_and_shift};

/// The three quadrant-0 instructions this emulator supports, distinguished
/// by `funct3` (`inst[15:13]`). Quadrant 0 has 8 possible `funct3` values;
/// the other 5 are floating-point loads/stores or RV64-only forms, neither
/// of which this emulator implements (see docs/plans/c_extension.md).
pub enum CTypeOps {
    AddI4Spn,
    Lw,
    Sw
}

impl CTypeOps {
    /// Classifies a quadrant-0 instruction by its funct3 field (0b000, 0b010,
    /// or 0b110). Any other funct3 value in this quadrant is either a
    /// floating-point/RV64-only opcode or genuinely reserved, and should be
    /// treated as an illegal instruction by the caller.
    pub fn from_func_three(func3: u32) -> Result<CTypeOps, TrapCause> {
        match func3 {
            0b000 => Ok(CTypeOps::AddI4Spn),
            0b010 => Ok(CTypeOps::Lw),
            0b110 => Ok(CTypeOps::Sw),
            _ => Err(TrapCause::IllegalInstruction { instruction: None })
        }
    }
}

/// Entry point for decoding any compressed instruction into the same
/// `Format` values the standard 32-bit decoder produces — see
/// docs/plans/c_extension.md's "decode-time expansion" section. Checks
/// the quadrant (`inst[1:0]`) first, since the same `funct3` value means
/// a different instruction in each quadrant, then dispatches within
/// that quadrant.
pub fn parse_c_inst(raw_word: Instruction) -> Result<Format, TrapCause> {
    let half_word = raw_word.0;
    let quadrant = mask_and_shift(half_word, masks::C_QUADRANT);
    match quadrant {
        0b00 => parse_c_quadrant_zero(half_word),
        0b01 => parse_c_quadrant_one(half_word),
        0b10 => parse_c_quadrant_two(half_word),
        _ => Err(TrapCause::IllegalInstruction { instruction: Some(half_word) })
    }
}

/// Quadrant 0 (`inst[1:0] == 00`): dispatches on `funct3` to one of the
/// three real instructions this quadrant supports for RV32 without
/// floating-point.
fn parse_c_quadrant_zero(half_word: u32) -> Result<Format, TrapCause> {
    let funct3 = mask_and_shift(half_word, masks::FUNCT_THREE);
    let build_formatted_type = match CTypeOps::from_func_three(funct3)? {
        CTypeOps::AddI4Spn => parse_c_addi4spn,
        CTypeOps::Lw => parse_c_lw,
        CTypeOps::Sw => parse_c_sw,
    };
    Ok(build_formatted_type(half_word)?)
}

/// Quadrant 2 (`inst[1:0] == 10`): `C.SLLI`, `C.LWSP`, `C.SWSP` are each
/// their own `funct3`. `funct3 == 0b100` is a fourth case covering FIVE
/// different instructions (`C.JR`/`C.MV`/`C.EBREAK`/`C.JALR`/`C.ADD`),
/// distinguished by bit 12 and whether `rs2` (`inst[6:2]`) is zero — see
/// Figure 5, §28.8, p.166.
fn parse_c_quadrant_two(half_word: u32) -> Result<Format, TrapCause> {
    let funct3 = mask_and_shift(half_word, masks::FUNCT_THREE);
    match funct3 {
        0b000 => parse_c_slli(half_word),
        0b010 => parse_c_lwsp(half_word),
        0b100 => parse_c_cr_group(half_word),
        0b110 => parse_c_swsp(half_word),
        _ => Err(TrapCause::IllegalInstruction { instruction: Some(half_word) })
    }
}

/// `C.ADDI4SPN` (CIW format): adds a zero-extended, non-zero 10-bit
/// immediate (scaled by 4) to the stack pointer (`x2`) and writes the
/// result to `rd'` (`8 + inst[4:2]`). Expands to `addi rd', x2, nzuimm[9:2]`.
/// The immediate's bits are scrambled in the encoding as
/// `inst[12:5] = nzuimm[5|4|9|8|7|6|2|3]` and must be reassembled in that
/// order before shifting left by 2.
pub fn parse_c_addi4spn(raw_inst: u32) -> Result<Format, TrapCause> {
    let scrambled_field = mask_and_shift(raw_inst, masks::C_ADDI4SPN_IMM);
    let rd = mask_and_shift(raw_inst, masks::C_REG);
    let imm_five = mask(raw_inst, 1 << 12) >> 7;
    let imm_four = mask(raw_inst, 1 << 11) >> 7;
    let imm_nine = mask(raw_inst, 1 << 10) >> 1;

    let imm_eight = mask(raw_inst, 1 << 9) >> 1;
    let imm_seven = mask(raw_inst, 1 << 8) >> 1;
    let imm_six = mask(raw_inst, 1 << 7) >> 1;

    let imm_two = mask(raw_inst, 1 << 6) >> 4;
    let imm_three = mask(raw_inst, 1 << 5) >> 2;
    let imm_combined = imm_two | imm_three | imm_four | imm_five | imm_six | imm_seven | imm_eight | imm_nine;
    if imm_combined == 0 {
        return Err(TrapCause::IllegalInstruction { instruction: Some(raw_inst) });
    }
    Ok(Format::AluImmType {
        op: AluImmOp::Addi,
        imm: imm_combined as i32,
        rd: (8 + rd) as usize,
        rs1: 2usize
    })
}

/// `C.SW` (CS format): stores the value in `rs2'` (`8 + inst[4:2]`) to the
/// address formed by `rs1'` (`8 + inst[9:7]`) plus a zero-extended,
/// word-aligned offset. Expands to `sw rs2', offset(rs1')`. Same immediate
/// layout as `C.LW` (`inst[12:10] = uimm[5:3]`, `inst[6] = uimm[2]`,
/// `inst[5] = uimm[6]`) — only which register field is source vs.
/// destination differs between the two.
pub fn parse_c_sw(raw_word: u32) -> Result<Format, TrapCause> {
    let rs1 = mask_and_shift(raw_word, masks::C_REG_BASE);
    let rs2 = mask_and_shift(raw_word, masks::C_REG);

    let imm_five = mask(raw_word, 1 << 12) >> 7;
    let imm_four = mask(raw_word, 1 << 11) >> 7;
    let imm_three = mask(raw_word, 1 << 10) >> 7;
    let imm_two = mask(raw_word, 1 << 6) >> 4;
    let imm_six = mask(raw_word, 1 << 5) << 1;

    let reassembled = imm_two | imm_three | imm_four | imm_five | imm_six;
    Ok(Format::SType {
        op: SOp::Sw,
        rs1: (8 + rs1) as usize,
        rs2: (8 + rs2) as usize,
        imm: reassembled as i32
    })
}

/// `C.LW` (CL format): loads a 32-bit value from the address formed by
/// `rs1'` (`8 + inst[9:7]`) plus a zero-extended, word-aligned offset, into
/// `rd'` (`8 + inst[4:2]`). Expands to `lw rd', offset(rs1')`. Immediate
/// bits are scrambled as `inst[12:10] = uimm[5:3]`, `inst[6] = uimm[2]`,
/// `inst[5] = uimm[6]` — reassemble to `uimm[6:2]` before use.
pub fn parse_c_lw(raw_word: u32) -> Result<Format, TrapCause> {
    let rs1 = mask_and_shift(raw_word, masks::C_REG_BASE);
    let rd = mask_and_shift(raw_word, masks::C_REG);

    let imm_five = mask(raw_word, 1 << 12) >> 7;
    let imm_four = mask(raw_word, 1 << 11) >> 7;
    let imm_three = mask(raw_word, 1 << 10) >> 7;
    let imm_two = mask(raw_word, 1 << 6) >> 4;
    let imm_six = mask(raw_word, 1 << 5) << 1;

    let reassembled = imm_two | imm_three | imm_four | imm_five | imm_six;
    Ok(Format::LoadType {
        op: LoadOp::Lw,
        rd: (8 + rd) as usize,
        rs1: (8 + rs1) as usize,
        imm: reassembled as i32
    })
}

/// `C.SLLI` (CI format, quadrant 2, funct3 `000`): shifts `rd` left by
/// `shamt` and writes the result back to `rd`. Unlike quadrant 0's
/// register fields, this uses the *full* 5-bit register (`inst[11:7]`,
/// any of x0-x31), no `8 +` remapping. Expands to `slli rd, rd,
/// shamt`. `shamt` is split across `inst[12]` (its top bit) and
/// `inst[6:2]` (the rest) — for RV32C, `inst[12]` must be 0 (a set bit
/// there is reserved for custom extensions per the spec).
fn parse_c_slli(raw_word: u32) -> Result<Format, TrapCause> {
    todo!()
}

/// `C.LWSP` (CI format, quadrant 2, funct3 `010`): loads a 32-bit value
/// from `offset(x2)` (the stack pointer) into `rd`. Full 5-bit `rd`
/// (`inst[11:7]`), must be nonzero — `rd == 0` is a reserved code point.
/// Expands to `lw rd, offset(x2)`.
fn parse_c_lwsp(raw_word: u32) -> Result<Format, TrapCause> {
    todo!()
}

/// `C.SWSP` (CSS format, quadrant 2, funct3 `110`): stores `rs2` (full
/// 5-bit, `inst[6:2]`) to `offset(x2)`. Expands to `sw rs2, offset(x2)`.
fn parse_c_swsp(raw_word: u32) -> Result<Format, TrapCause> {
    todo!()
}

/// Quadrant 2, funct3 `100` (CR format) — five different instructions
/// share this one funct3, distinguished by bit 12 and whether `rs2`
/// (`inst[6:2]`) is zero:
/// - bit12=0, rs2=0:  `C.JR`     — jump to address in `rs1` (`inst[11:7]`, must be ≠0)
/// - bit12=0, rs2≠0:  `C.MV`     — `rd = rs2` (rd = inst[11:7], rs2 = inst[6:2])
/// - bit12=1, rs1=0, rs2=0: `C.EBREAK`
/// - bit12=1, rs1≠0, rs2=0: `C.JALR`   — jump-and-link to address in `rs1`
/// - bit12=1, rs1≠0, rs2≠0: `C.ADD`    — `rd = rd + rs2`
/// All registers here are full 5-bit fields, same as `C.SLLI`/`C.LWSP` —
/// no x8-x15 remapping in this quadrant.
fn parse_c_cr_group(raw_word: u32) -> Result<Format, TrapCause> {
    todo!()
}

/// Quadrant 1 (`inst[1:0] == 01`): the largest and most varied quadrant
/// — 8 `funct3` values, mixing full 5-bit registers (`C.ADDI`/`C.LI`/
/// `C.LUI`/`C.ADDI16SP`/`C.J`/`C.JAL`) with the x8-x15 compressed
/// register fields (`C.SRLI`/`C.SRAI`/`C.ANDI`/`C.SUB`/`C.XOR`/`C.OR`/
/// `C.AND`/`C.BEQZ`/`C.BNEZ`) — check which each instruction uses
/// individually rather than assuming, unlike quadrants 0 and 2, which
/// were each consistently one or the other.
fn parse_c_quadrant_one(half_word: u32) -> Result<Format, TrapCause> {
    let funct3 = mask_and_shift(half_word, masks::FUNCT_THREE);
    match funct3 {
        0b000 => parse_c_addi_or_nop(half_word),
        0b001 => parse_c_jal(half_word),
        0b010 => parse_c_li(half_word),
        0b011 => parse_c_lui_or_addi16sp(half_word),
        0b100 => parse_c_misc_alu(half_word),
        0b101 => parse_c_j(half_word),
        0b110 => parse_c_beqz(half_word),
        0b111 => parse_c_bnez(half_word),
        _ => unreachable!() // funct3 is 3 bits; every value is one of the 8 above
    }
}

/// `C.ADDI`/`C.NOP` (CI format, funct3 `000`): `C.ADDI` adds a
/// sign-extended 6-bit immediate to `rd` (full 5-bit, `inst[11:7]`) and
/// writes it back. `C.NOP` is the same encoding with `rd = x0` (and,
/// per §28.5.2, `rd≠0` with `imm=0` is a HINT — no special-casing
/// needed, it naturally expands to a real no-op-equivalent `addi rd,
/// rd, 0` through the normal path). Immediate: `inst[12]` = sign bit,
/// `inst[6:2]` = the rest — sign-extend from bit 5 upward. Expands to
/// `addi rd, rd, imm`.
fn parse_c_addi_or_nop(half_word: u32) -> Result<Format, TrapCause> {
    todo!()
}

/// `C.JAL` (CJ format, funct3 `001`, RV32-only — this opcode is
/// `C.ADDIW` on RV64, which this emulator doesn't implement anyway): an
/// 11-bit sign-extended offset, heavily scrambled across `inst[12:2]`
/// (see Figure 4, §28.8, p.165 for the exact bit order), added to `pc`
/// to form the jump target; the return address (`pc+2`) is written to
/// `x1` unconditionally — no register field to extract, the destination
/// is implicit. Expands to `jal x1, offset`.
fn parse_c_jal(half_word: u32) -> Result<Format, TrapCause> {
    todo!()
}

/// `C.LI` (CI format, funct3 `010`): loads a sign-extended 6-bit
/// immediate into `rd` (full 5-bit, `inst[11:7]`) — same immediate
/// layout as `C.ADDI` (`inst[12]` = sign bit, `inst[6:2]` = rest).
/// `rd == 0` is a HINT (handled for free, same reasoning as `C.NOP`
/// above). Expands to `addi rd, x0, imm`.
fn parse_c_li(half_word: u32) -> Result<Format, TrapCause> {
    todo!()
}

/// `C.LUI`/`C.ADDI16SP` (CI format, funct3 `011`) — share an opcode,
/// distinguished by `rd` (`inst[11:7]`): `rd == 2` means `C.ADDI16SP`
/// (adds a sign-extended immediate, scaled by 16, to the stack pointer
/// — `inst[12]` and `inst[6:2]` scrambled as `nzimm[9|4|6|8:7|5]`,
/// expands to `addi x2, x2, imm`); any other nonzero `rd` means `C.LUI`
/// (loads a sign-extended 6-bit immediate into bits 17-12 of `rd`,
/// clearing the bottom 12 — expands to `lui rd, imm`). `rd == 0` or
/// `imm == 0` are both reserved code points for whichever of the two
/// this is.
fn parse_c_lui_or_addi16sp(half_word: u32) -> Result<Format, TrapCause> {
    todo!()
}

/// The `MISC-ALU` group (CB/CA formats, funct3 `100`) — six
/// instructions nested under one `funct3`, all using the x8-x15
/// compressed register field (`inst[9:7]` as `rd'`/`rs1'`). First
/// dispatch on `inst[11:10]`:
/// - `00`: `C.SRLI` — logical right shift by `shamt` (`inst[12]` +
///   `inst[6:2]`; `inst[12]` must be 0 for RV32)
/// - `01`: `C.SRAI` — same shift encoding, arithmetic instead of logical
/// - `10`: `C.ANDI` — bitwise AND with a sign-extended 6-bit immediate
///   (same layout as `C.ADDI`'s)
/// - `11`: a *second* register-register op, sharing `rs2'` at
///   `inst[4:2]`, dispatched further by `inst[6:5]` (and `inst[12]`,
///   which must be 0 here — a set bit selects RV64-only `C.SUBW`/
///   `C.ADDW`, not applicable to this emulator):
///   - `00`: `C.SUB`, `01`: `C.XOR`, `10`: `C.OR`, `11`: `C.AND`
fn parse_c_misc_alu(half_word: u32) -> Result<Format, TrapCause> {
    todo!()
}

/// `C.J` (CJ format, funct3 `101`): identical to `C.JAL` (same
/// scrambled 11-bit offset, same field layout) except it does *not*
/// write a return address anywhere — expands to `jal x0, offset`.
fn parse_c_j(half_word: u32) -> Result<Format, TrapCause> {
    todo!()
}

/// `C.BEQZ` (CB format, funct3 `110`): branches if `rs1'` (x8-x15,
/// `inst[9:7]`) is zero. An 8-bit sign-extended offset scrambled across
/// `inst[12]`, `inst[11:10]`, `inst[6:5]`, `inst[4:3]`, `inst[2]` (see
/// §28.4, p.159 for the exact bit order). Expands to `beq rs1', x0,
/// offset`.
fn parse_c_beqz(half_word: u32) -> Result<Format, TrapCause> {
    todo!()
}

/// `C.BNEZ` (CB format, funct3 `111`): identical field layout to
/// `C.BEQZ`, branches on nonzero instead of zero. Expands to `bne
/// rs1', x0, offset`.
fn parse_c_bnez(half_word: u32) -> Result<Format, TrapCause> {
    todo!()
}
