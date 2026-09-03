// Quadrant 0 of the RVC ("C") extension — see docs/plans/c_extension.md
// and .claude/riscv-unprivileged.txt Chapter 28 for the full spec.

use crate::cpu::definitions::masks;
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::cpu::fetcher::Instruction;
use crate::cpu::instructions::Format;
use crate::cpu::instructions::i::alu_imm_or_shift::AluImmOp;
use crate::cpu::instructions::s::SOp;
use crate::cpu::instructions::i::load::LoadOp;
use crate::cpu::instructions::i::alu_imm_or_shift::IShOp;
use crate::cpu::instructions::i::system::SystemOp;
use crate::cpu::instructions::r::AluOp;
use crate::cpu::instructions::j::JOp;
use crate::cpu::instructions::u::UOp;
use crate::cpu::instructions::b::BOp;
use crate::utility::bit_operations::{mask, mask_and_shift, shake_to_signed};

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
    let funct3 = mask_and_shift(half_word, masks::C_FUNCT_THREE);
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
    let funct3 = mask_and_shift(half_word, masks::C_FUNCT_THREE);
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
    let rd = mask_and_shift(raw_word, masks::REG_DESTINATION) as usize;
    let shamt_low = mask_and_shift(raw_word, masks::C_SHAMT_LOW);
    let shamt_high = mask(raw_word, 1 << 12) >> 7;
    let shamt = (shamt_high | shamt_low) as usize;
    if shamt_high != 0 {
        return Err(TrapCause::IllegalInstruction { instruction: Some(raw_word)});
    }

    Ok(Format::IShiftType {
        op: IShOp::Slli,
        rd,
        rs1: rd,
        shamt
    })
}

/// `C.LWSP` (CI format, quadrant 2, funct3 `010`): loads a 32-bit value
/// from `offset(x2)` (the stack pointer) into `rd`. Full 5-bit `rd`
/// (`inst[11:7]`), must be nonzero — `rd == 0` is a reserved code point.
/// Expands to `lw rd, offset(x2)`.
fn parse_c_lwsp(raw_word: u32) -> Result<Format, TrapCause> {
    let rd = mask_and_shift(raw_word, masks::REG_DESTINATION) as usize;
    if rd == 0 {
        return Err(TrapCause::IllegalInstruction { instruction: Some(raw_word) });
    }

    let offset_five = mask(raw_word, 1 << 12) >> 7;
    let offset_four = mask(raw_word, 1 << 6) >> 2;
    let offset_three = mask(raw_word, 1 << 5) >> 2;
    let offset_two = mask(raw_word, 1 << 4) >> 2;
    let offset_seven = mask(raw_word, 1 << 3) << 4;
    let offset_six = mask(raw_word, 1 << 2) << 4;
    let reassembled = offset_two | offset_three | offset_four | offset_five | offset_six | offset_seven;

    Ok(Format::LoadType {
        op: LoadOp::Lw,
        rd,
        rs1: 2,
        imm: reassembled as i32
    })
}

/// `C.SWSP` (CSS format, quadrant 2, funct3 `110`): stores `rs2` (full
/// 5-bit, `inst[6:2]`) to `offset(x2)`. Expands to `sw rs2, offset(x2)`.
fn parse_c_swsp(raw_word: u32) -> Result<Format, TrapCause> {
    let rs2 = mask_and_shift(raw_word, masks::C_REG_FULL) as usize;

    let offset_five = mask(raw_word, 1 << 12) >> 7;
    let offset_four = mask(raw_word, 1 << 11) >> 7;
    let offset_three = mask(raw_word, 1 << 10) >> 7;
    let offset_two = mask(raw_word, 1 << 9) >> 7;
    let offset_seven = mask(raw_word, 1 << 8) >> 1;
    let offset_six = mask(raw_word, 1 << 7) >> 1;
    let reassembled = offset_two | offset_three | offset_four | offset_five | offset_six | offset_seven;

    Ok(Format::SType {
        op: SOp::Sw,
        rs1: 2,
        rs2,
        imm: reassembled as i32
    })
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
    let bit_twelve = mask_and_shift(raw_word, 1 << 12);
    let rs1_or_rd = mask_and_shift(raw_word, masks::REG_DESTINATION) as usize;
    let rs2 = mask_and_shift(raw_word, masks::C_REG_FULL) as usize;
    let bit_twelve_is_zero = bit_twelve == 0;
    let rs2_is_zero = rs2 == 0;
    let rs1_is_zero = rs1_or_rd == 0;
    match (bit_twelve_is_zero, rs2_is_zero, rs1_is_zero) {
        (true, true, false) => Ok(Format::JalrType { // C.JR -- rs1=0 is reserved (p.156)
            rd: 0,
            rs1: rs1_or_rd,
            imm: 0
        }),
        (false, true, false) => Ok(Format::JalrType {  // C.JALR
            rd: 1,
            rs1: rs1_or_rd,
            imm: 0
        }),
        (true, false, _) => Ok(Format::RType { // C.MV
            op: AluOp::Add,
            rd: rs1_or_rd,
            rs1: 0,
            rs2
        }),
        (false, false, _) => Ok(Format::RType { // C.ADD
            op: AluOp::Add,
            rd: rs1_or_rd,
            rs1: rs1_or_rd,
            rs2

        }),
        (false, true, true) => Ok(Format::SystemType { // C.EBREAK
            op: SystemOp::EBreak
        }),
        (_, _, _) => Err(TrapCause::IllegalInstruction { instruction: Some(raw_word)})
    }
}

/// Quadrant 1 (`inst[1:0] == 01`): the largest and most varied quadrant
/// — 8 `funct3` values, mixing full 5-bit registers (`C.ADDI`/`C.LI`/
/// `C.LUI`/`C.ADDI16SP`/`C.J`/`C.JAL`) with the x8-x15 compressed
/// register fields (`C.SRLI`/`C.SRAI`/`C.ANDI`/`C.SUB`/`C.XOR`/`C.OR`/
/// `C.AND`/`C.BEQZ`/`C.BNEZ`) — check which each instruction uses
/// individually rather than assuming, unlike quadrants 0 and 2, which
/// were each consistently one or the other.
fn parse_c_quadrant_one(half_word: u32) -> Result<Format, TrapCause> {
    let funct3 = mask_and_shift(half_word, masks::C_FUNCT_THREE);
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
    let rd = mask_and_shift(half_word, masks::REG_DESTINATION) as usize;
    let imm_low = mask_and_shift(half_word, masks::C_SHAMT_LOW);
    let imm_high = mask(half_word, 1 << 12) >> 7;
    let imm_unsigned = imm_high | imm_low;
    let imm = shake_to_signed(imm_unsigned, 6);
    Ok(Format::AluImmType {
        op: AluImmOp::Addi,
        rd,
        rs1: rd,
        imm
    })
}

/// `C.JAL` (CJ format, funct3 `001`, RV32-only — this opcode is
/// `C.ADDIW` on RV64, which this emulator doesn't implement anyway): an
/// 11-bit sign-extended offset, heavily scrambled across `inst[12:2]`
/// (see Figure 4, §28.8, p.165 for the exact bit order), added to `pc`
/// to form the jump target; the return address (`pc+2`) is written to
/// `x1` unconditionally — no register field to extract, the destination
/// is implicit. Expands to `jal x1, offset`.
fn parse_c_jal(half_word: u32) -> Result<Format, TrapCause> {
    let offset_eleven = mask(half_word, 1 << 12) >> 1;
    let offset_four = mask(half_word, 1 << 11) >> 7;
    let offset_nine = mask(half_word, 1 << 10) >> 1;
    let offset_eight = mask(half_word, 1 << 9) >> 1;
    let offset_ten = mask(half_word, 1 << 8) << 2;
    let offset_six = mask(half_word, 1 << 7) >> 1;
    let offset_seven = mask(half_word, 1 << 6) << 1;
    let offset_three = mask(half_word, 1 << 5) >> 2;
    let offset_two = mask(half_word, 1 << 4) >> 2;
    let offset_one = mask(half_word, 1 << 3) >> 2;
    let offset_five = mask(half_word, 1 << 2) << 3;

    let reassembled = offset_one | offset_two | offset_three | offset_four | offset_five
        | offset_six | offset_seven | offset_eight | offset_nine | offset_ten | offset_eleven;
    let imm = shake_to_signed(reassembled, 12);

    Ok(Format::JType { op: JOp::Jal, rd: 1, imm })
}

/// `C.LI` (CI format, funct3 `010`): loads a sign-extended 6-bit
/// immediate into `rd` (full 5-bit, `inst[11:7]`) — same immediate
/// layout as `C.ADDI` (`inst[12]` = sign bit, `inst[6:2]` = rest).
/// `rd == 0` is a HINT (handled for free, same reasoning as `C.NOP`
/// above). Expands to `addi rd, x0, imm`.
fn parse_c_li(half_word: u32) -> Result<Format, TrapCause> {
    let rd = mask_and_shift(half_word, masks::REG_DESTINATION) as usize;
    let imm_low = mask_and_shift(half_word, masks::C_SHAMT_LOW);
    let imm_high = mask(half_word, 1 << 12) >> 7;
    let imm_unsigned = imm_high | imm_low;
    let imm = shake_to_signed(imm_unsigned, 6);
    Ok(Format::AluImmType {
        op: AluImmOp::Addi,
        rd,
        rs1: 0,
        imm
    })
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
    let rd = mask_and_shift(half_word, masks::REG_DESTINATION) as usize;
    if rd == 2 {
        // C.ADDI16SP: nzimm[9|4|6|8:7|5] scrambled across inst[12,6,5,4:3,2]
        let nzimm_nine = mask(half_word, 1 << 12) >> 3;
        let nzimm_eight_seven = mask(half_word, 0b11 << 3) << 4;
        let nzimm_six = mask(half_word, 1 << 5) << 1;
        let nzimm_five = mask(half_word, 1 << 2) << 3;
        let nzimm_four = mask(half_word, 1 << 6) >> 2;
        let reassembled = nzimm_nine | nzimm_eight_seven | nzimm_six | nzimm_five | nzimm_four;
        let imm = shake_to_signed(reassembled, 10);
        if imm == 0 {
            return Err(TrapCause::IllegalInstruction { instruction: Some(half_word) });
        }
        Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 2, rs1: 2, imm })
    } else {
        let imm_low = mask_and_shift(half_word, masks::C_SHAMT_LOW);
        let imm_high = mask(half_word, 1 << 12) >> 7;
        let imm_unsigned = imm_high | imm_low;
        let imm6 = shake_to_signed(imm_unsigned, 6);
        if rd == 0 || imm6 == 0 {
            return Err(TrapCause::IllegalInstruction { instruction: Some(half_word) });
        }
        let imm_upper = imm6 << 12;
        Ok(Format::UType { op: UOp::Lui, rd, imm_upper })
    }
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

fn parse_c_misc_alu(half_word: u32) -> Result<Format, TrapCause> {
    let rd = 8 + mask_and_shift(half_word, masks::C_REG_BASE) as usize;
    let bits_eleven_ten = mask_and_shift(half_word, 0b11 << 10);
    match bits_eleven_ten {
        0b00 | 0b01 => {
            let shamt_low = mask_and_shift(half_word, masks::C_SHAMT_LOW);
            let shamt_high = mask(half_word, 1 << 12) >> 7;
            if shamt_high != 0 {
                return Err(TrapCause::IllegalInstruction { instruction: Some(half_word) });
            }
            let op = if bits_eleven_ten == 0b00 { IShOp::Srli } else { IShOp::Srai };
            Ok(Format::IShiftType { op, rd, rs1: rd, shamt: shamt_low as usize })
        },
        0b10 => {
            let imm_low = mask_and_shift(half_word, masks::C_SHAMT_LOW);
            let imm_high = mask(half_word, 1 << 12) >> 7;
            let imm_unsigned = imm_high | imm_low;
            let imm = shake_to_signed(imm_unsigned, 6);
            Ok(Format::AluImmType { op: AluImmOp::Andi, rd, rs1: rd, imm })
        },
        _ => {
            // `11`: a *second* register-register op, sharing `rs2'` at
            // `inst[4:2]`, dispatched further by `inst[6:5]` (and `inst[12]`,
            // which must be 0 here — a set bit selects RV64-only `C.SUBW`/
            // `C.ADDW`, not applicable to this emulator):
            // `00`: `C.SUB`, `01`: `C.XOR`, `10`: `C.OR`, `11`: `C.AND`
            let bit_twelve = mask(half_word, 1 << 12);
            if bit_twelve != 0 {
                return Err(TrapCause::IllegalInstruction { instruction: Some(half_word) });
            }
            let rs2 = 8 + mask_and_shift(half_word, masks::C_REG) as usize;
            let bits_six_five = mask_and_shift(half_word, 0b11 << 5);
            let op = match bits_six_five {
                0b00 => AluOp::Sub,
                0b01 => AluOp::Xor,
                0b10 => AluOp::Or,
                _ => AluOp::And
            };
            Ok(Format::RType { op, rd, rs1: rd, rs2 })
        }
    }
}

/// `C.J` (CJ format, funct3 `101`): identical to `C.JAL` (same
/// scrambled 11-bit offset, same field layout) except it does *not*
/// write a return address anywhere — expands to `jal x0, offset`.
fn parse_c_j(half_word: u32) -> Result<Format, TrapCause> {
    let offset_eleven = mask(half_word, 1 << 12) >> 1;
    let offset_four = mask(half_word, 1 << 11) >> 7;
    let offset_nine = mask(half_word, 1 << 10) >> 1;
    let offset_eight = mask(half_word, 1 << 9) >> 1;
    let offset_ten = mask(half_word, 1 << 8) << 2;
    let offset_six = mask(half_word, 1 << 7) >> 1;
    let offset_seven = mask(half_word, 1 << 6) << 1;
    let offset_three = mask(half_word, 1 << 5) >> 2;
    let offset_two = mask(half_word, 1 << 4) >> 2;
    let offset_one = mask(half_word, 1 << 3) >> 2;
    let offset_five = mask(half_word, 1 << 2) << 3;

    let reassembled = offset_one | offset_two | offset_three | offset_four | offset_five
        | offset_six | offset_seven | offset_eight | offset_nine | offset_ten | offset_eleven;
    let imm = shake_to_signed(reassembled, 12);

    Ok(Format::JType { op: JOp::Jal, rd: 0, imm })
}

/// `C.BEQZ` (CB format, funct3 `110`): branches if `rs1'` (x8-x15,
/// `inst[9:7]`) is zero. An 8-bit sign-extended offset scrambled across
/// `inst[12]`, `inst[11:10]`, `inst[6:5]`, `inst[4:3]`, `inst[2]` (see
/// §28.4, p.159 for the exact bit order). Expands to `beq rs1', x0,
/// offset`.
fn parse_c_beqz(half_word: u32) -> Result<Format, TrapCause> {
    let (rs1, imm) = parse_c_branch_fields(half_word);
    Ok(Format::BType { op: BOp::Beq, imm, rs1, rs2: 0 })
}

/// `C.BNEZ` (CB format, funct3 `111`): identical field layout to
/// `C.BEQZ`, branches on nonzero instead of zero. Expands to `bne
/// rs1', x0, offset`.
fn parse_c_bnez(half_word: u32) -> Result<Format, TrapCause> {
    let (rs1, imm) = parse_c_branch_fields(half_word);
    Ok(Format::BType { op: BOp::Bne, imm, rs1, rs2: 0 })
}

/// Shared field extraction for `C.BEQZ`/`C.BNEZ` (CB format): `rs1'`
/// (`8 + inst[9:7]`) and the 9-bit sign-extended offset scrambled as
/// `imm[8|4:3]` (`inst[12:10]`), `imm[7:6|2:1|5]` (`inst[6:2]`).
fn parse_c_branch_fields(half_word: u32) -> (usize, i32) {
    let rs1 = 8 + mask_and_shift(half_word, masks::C_REG_BASE) as usize;

    let imm_eight = mask(half_word, 1 << 12) >> 4;
    let imm_four = mask(half_word, 1 << 11) >> 7;
    let imm_three = mask(half_word, 1 << 10) >> 7;
    let imm_seven = mask(half_word, 1 << 6) << 1;
    let imm_six = mask(half_word, 1 << 5) << 1;
    let imm_two = mask(half_word, 1 << 4) >> 2;
    let imm_one = mask(half_word, 1 << 3) >> 2;
    let imm_five = mask(half_word, 1 << 2) << 3;

    let reassembled = imm_one | imm_two | imm_three | imm_four | imm_five
        | imm_six | imm_seven | imm_eight;
    let imm = shake_to_signed(reassembled, 9);
    (rs1, imm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utility::types::ByteType;

    // --- quadrant 0 ---

    #[test]
    fn test_parse_c_addi4spn() {
        // rd'=3 (real x11), nzuimm[9:2] bits 3:2 set -> immediate 12
        let result = parse_c_addi4spn(0x006C);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 11, rs1: 2, imm: 12 }));
    }

    #[test]
    fn test_parse_c_addi4spn_reserved_zero_imm_is_illegal() {
        // rd'=3, but every immediate bit is 0 -- reserved code point
        let result = parse_c_addi4spn(0x000C);
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: Some(0x000C) }));
    }

    #[test]
    fn test_parse_c_lw() {
        // rs1'=2 (real x10), rd'=5 (real x13), offset=4
        let result = parse_c_lw(0x4154);
        assert_eq!(result, Ok(Format::LoadType { op: LoadOp::Lw, rd: 13, rs1: 10, imm: 4 }));
    }

    #[test]
    fn test_parse_c_sw() {
        // same raw word as C.LW's test -- CS and CL share this field layout,
        // just with rs1'/rs2' swapped roles
        let result = parse_c_sw(0x4154);
        assert_eq!(result, Ok(Format::SType { op: SOp::Sw, rs1: 10, rs2: 13, imm: 4 }));
    }

    #[test]
    fn test_parse_c_quadrant_zero_reserved_funct3_is_illegal() {
        // funct3 = 001 (inst[15:13]) isn't one of the three RV32-without-FP
        // instructions this quadrant supports
        let result = parse_c_quadrant_zero(0x2000);
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: None }));
    }

    // --- quadrant 2 ---

    #[test]
    fn test_parse_c_slli() {
        // rd=5, shamt=7 (shamt[5]=0)
        let result = parse_c_slli(0x029E);
        assert_eq!(result, Ok(Format::IShiftType { op: IShOp::Slli, rd: 5, rs1: 5, shamt: 7 }));
    }

    #[test]
    fn test_parse_c_slli_reserved_shamt_five_is_illegal() {
        // same as the valid case above, but shamt[5]=1 -- reserved for
        // custom extensions on RV32C
        let result = parse_c_slli(0x129E);
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: Some(0x129E) }));
    }

    #[test]
    fn test_parse_c_lwsp() {
        // rd=9, offset=4
        let result = parse_c_lwsp(0x4492);
        assert_eq!(result, Ok(Format::LoadType { op: LoadOp::Lw, rd: 9, rs1: 2, imm: 4 }));
    }

    #[test]
    fn test_parse_c_lwsp_reserved_rd_zero_is_illegal() {
        // same offset as the valid case above, but rd=0
        let result = parse_c_lwsp(0x4012);
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: Some(0x4012) }));
    }

    #[test]
    fn test_parse_c_swsp() {
        // rs2=7, offset=8
        let result = parse_c_swsp(0xC41E);
        assert_eq!(result, Ok(Format::SType { op: SOp::Sw, rs1: 2, rs2: 7, imm: 8 }));
    }

    #[test]
    fn test_parse_c_cr_group_jr() {
        // bit12=0, rs2=0, rs1=5 (nonzero)
        let result = parse_c_cr_group(0x8282);
        assert_eq!(result, Ok(Format::JalrType { rd: 0, rs1: 5, imm: 0 }));
    }

    #[test]
    fn test_parse_c_cr_group_jr_reserved_rs1_zero_is_illegal() {
        // bit12=0, rs2=0, rs1=0 -- reserved per the spec's own text: "C.JR
        // is valid only when rs1≠x0"
        let result = parse_c_cr_group(0x8002);
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: Some(0x8002) }));
    }

    #[test]
    fn test_parse_c_cr_group_mv() {
        // bit12=0, rs2=6 (nonzero), rd=3
        let result = parse_c_cr_group(0x819A);
        assert_eq!(result, Ok(Format::RType { op: AluOp::Add, rd: 3, rs1: 0, rs2: 6 }));
    }

    #[test]
    fn test_parse_c_cr_group_mv_hint_rd_zero_is_not_illegal() {
        // bit12=0, rs2=6 (nonzero), rd=0 -- a HINT, not a reserved code
        // point; must still decode as an ordinary (if useless) C.MV
        let result = parse_c_cr_group(0x801A);
        assert_eq!(result, Ok(Format::RType { op: AluOp::Add, rd: 0, rs1: 0, rs2: 6 }));
    }

    #[test]
    fn test_parse_c_cr_group_add() {
        // bit12=1, rs2=6 (nonzero), rd=3
        let result = parse_c_cr_group(0x919A);
        assert_eq!(result, Ok(Format::RType { op: AluOp::Add, rd: 3, rs1: 3, rs2: 6 }));
    }

    #[test]
    fn test_parse_c_cr_group_jalr() {
        // bit12=1, rs2=0, rs1=5 (nonzero)
        let result = parse_c_cr_group(0x9282);
        assert_eq!(result, Ok(Format::JalrType { rd: 1, rs1: 5, imm: 0 }));
    }

    #[test]
    fn test_parse_c_cr_group_ebreak() {
        // bit12=1, rs2=0, rs1=0
        let result = parse_c_cr_group(0x9002);
        assert_eq!(result, Ok(Format::SystemType { op: SystemOp::EBreak }));
    }

    #[test]
    fn test_parse_c_quadrant_two_reserved_funct3_is_illegal() {
        // funct3 = 001 (inst[15:13]) isn't one of C.SLLI/C.LWSP/the CR group/C.SWSP
        let result = parse_c_quadrant_two(0x2002);
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: Some(0x2002) }));
    }

    // --- quadrant 1 ---

    #[test]
    fn test_parse_c_addi() {
        // rd=10, imm=5
        let result = parse_c_addi_or_nop(0x0515);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 10, rs1: 10, imm: 5 }));
    }

    #[test]
    fn test_parse_c_addi_negative_immediate() {
        // rd=5, imm=-1 -- exercises the sign-extension path specifically
        let result = parse_c_addi_or_nop(0x12FD);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 5, rs1: 5, imm: -1 }));
    }

    #[test]
    fn test_parse_c_nop() {
        // rd=0, imm=0 -- the canonical C.NOP encoding
        let result = parse_c_addi_or_nop(0x0001);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 0, rs1: 0, imm: 0 }));
    }

    #[test]
    fn test_parse_c_addi_hint_rd_nonzero_imm_zero_is_not_illegal() {
        // rd=7 (nonzero), imm=0 -- a HINT, must still decode as an
        // ordinary (if useless) addi, not fault
        let result = parse_c_addi_or_nop(0x0381);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 7, rs1: 7, imm: 0 }));
    }

    #[test]
    fn test_parse_c_jal_negative_offset() {
        // offset[11] set, every other offset bit 0 -- the exact case that
        // exposed the earlier width=11-vs-12 sign-extension bug
        let result = parse_c_jal(0x3001);
        assert_eq!(result, Ok(Format::JType { op: JOp::Jal, rd: 1, imm: -2048 }));
    }

    #[test]
    fn test_parse_c_jal_positive_offset() {
        let result = parse_c_jal(0x2011);
        assert_eq!(result, Ok(Format::JType { op: JOp::Jal, rd: 1, imm: 4 }));
    }

    #[test]
    fn test_parse_c_li() {
        // rd=9, imm=-1
        let result = parse_c_li(0x54FD);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 9, rs1: 0, imm: -1 }));
    }

    #[test]
    fn test_parse_c_li_hint_rd_zero_is_not_illegal() {
        // rd=0, imm=5 -- a HINT, must still decode as an ordinary addi
        let result = parse_c_li(0x4015);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 0, rs1: 0, imm: 5 }));
    }

    #[test]
    fn test_parse_c_lui() {
        // rd=5 (!=2), imm6=3 -> imm_upper = 3 << 12
        let result = parse_c_lui_or_addi16sp(0x628D);
        assert_eq!(result, Ok(Format::UType { op: UOp::Lui, rd: 5, imm_upper: 3 << 12 }));
    }

    #[test]
    fn test_parse_c_lui_reserved_rd_zero_is_illegal() {
        let result = parse_c_lui_or_addi16sp(0x600D);
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: Some(0x600D) }));
    }

    #[test]
    fn test_parse_c_lui_reserved_imm_zero_is_illegal() {
        let result = parse_c_lui_or_addi16sp(0x6281);
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: Some(0x6281) }));
    }

    #[test]
    fn test_parse_c_addi16sp() {
        // rd=2 -> C.ADDI16SP, nzimm[4]=1 -> imm=16
        let result = parse_c_lui_or_addi16sp(0x6141);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 2, rs1: 2, imm: 16 }));
    }

    #[test]
    fn test_parse_c_addi16sp_negative_immediate() {
        // rd=2, nzimm[9]=1 (sign bit) only -- exercises sign extension
        let result = parse_c_lui_or_addi16sp(0x7101);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 2, rs1: 2, imm: -512 }));
    }

    #[test]
    fn test_parse_c_addi16sp_reserved_zero_imm_is_illegal() {
        let result = parse_c_lui_or_addi16sp(0x6101);
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: Some(0x6101) }));
    }

    #[test]
    fn test_parse_c_misc_alu_srli() {
        // bits[11:10]=00, rd'=3 (real x11), shamt=5
        let result = parse_c_misc_alu(0x8195);
        assert_eq!(result, Ok(Format::IShiftType { op: IShOp::Srli, rd: 11, rs1: 11, shamt: 5 }));
    }

    #[test]
    fn test_parse_c_misc_alu_srai() {
        // same as the SRLI case, bits[11:10]=01 instead
        let result = parse_c_misc_alu(0x8595);
        assert_eq!(result, Ok(Format::IShiftType { op: IShOp::Srai, rd: 11, rs1: 11, shamt: 5 }));
    }

    #[test]
    fn test_parse_c_misc_alu_srli_reserved_shamt_five_is_illegal() {
        // SRLI's encoding above with shamt[5] (bit 12) also set
        let result = parse_c_misc_alu(0x9195);
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: Some(0x9195) }));
    }

    #[test]
    fn test_parse_c_misc_alu_andi() {
        // bits[11:10]=10, rd'=3 (real x11), imm=5
        let result = parse_c_misc_alu(0x8995);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Andi, rd: 11, rs1: 11, imm: 5 }));
    }

    #[test]
    fn test_parse_c_misc_alu_sub() {
        // bits[11:10]=11, bits[6:5]=00, rd'=3 (real x11), rs2'=2 (real x10)
        let result = parse_c_misc_alu(0x8D89);
        assert_eq!(result, Ok(Format::RType { op: AluOp::Sub, rd: 11, rs1: 11, rs2: 10 }));
    }

    #[test]
    fn test_parse_c_misc_alu_xor() {
        let result = parse_c_misc_alu(0x8DA9);
        assert_eq!(result, Ok(Format::RType { op: AluOp::Xor, rd: 11, rs1: 11, rs2: 10 }));
    }

    #[test]
    fn test_parse_c_misc_alu_or() {
        let result = parse_c_misc_alu(0x8DC9);
        assert_eq!(result, Ok(Format::RType { op: AluOp::Or, rd: 11, rs1: 11, rs2: 10 }));
    }

    #[test]
    fn test_parse_c_misc_alu_and() {
        let result = parse_c_misc_alu(0x8DE9);
        assert_eq!(result, Ok(Format::RType { op: AluOp::And, rd: 11, rs1: 11, rs2: 10 }));
    }

    #[test]
    fn test_parse_c_misc_alu_reg_reg_reserved_bit_twelve_is_illegal() {
        // C.SUB's encoding above with bit 12 also set -- selects RV64-only
        // C.SUBW, not implemented here
        let result = parse_c_misc_alu(0x9D89);
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: Some(0x9D89) }));
    }

    #[test]
    fn test_parse_c_j() {
        // same offset encoding as the C.JAL negative-offset test, but
        // rd=0 (no link register written)
        let result = parse_c_j(0xB001);
        assert_eq!(result, Ok(Format::JType { op: JOp::Jal, rd: 0, imm: -2048 }));
    }

    #[test]
    fn test_parse_c_beqz() {
        // rs1'=1 (real x9), offset=2
        let result = parse_c_beqz(0xC089);
        assert_eq!(result, Ok(Format::BType { op: BOp::Beq, imm: 2, rs1: 9, rs2: 0 }));
    }

    #[test]
    fn test_parse_c_bnez() {
        // same fields as the C.BEQZ test, funct3=111 instead of 110
        let result = parse_c_bnez(0xE089);
        assert_eq!(result, Ok(Format::BType { op: BOp::Bne, imm: 2, rs1: 9, rs2: 0 }));
    }

    #[test]
    fn test_parse_c_beqz_negative_offset() {
        // offset[8] (the sign bit) set, every other offset bit 0
        let result = parse_c_beqz(0xD081);
        assert_eq!(result, Ok(Format::BType { op: BOp::Beq, imm: -256, rs1: 9, rs2: 0 }));
    }

    #[test]
    fn test_parse_c_quadrant_one_dispatches_every_funct3() {
        // sanity-checks the dispatcher itself wires all 8 funct3 values to
        // the right parser, independent of the field-level tests above
        assert!(matches!(parse_c_quadrant_one(0x0001), Ok(Format::AluImmType { .. }))); // C.NOP
        assert!(matches!(parse_c_quadrant_one(0x3001), Ok(Format::JType { .. })));      // C.JAL
        assert!(matches!(parse_c_quadrant_one(0x4015), Ok(Format::AluImmType { .. }))); // C.LI
        assert!(matches!(parse_c_quadrant_one(0x6141), Ok(Format::AluImmType { .. }))); // C.ADDI16SP
        assert!(matches!(parse_c_quadrant_one(0x8195), Ok(Format::IShiftType { .. }))); // C.SRLI
        assert!(matches!(parse_c_quadrant_one(0xB001), Ok(Format::JType { .. })));      // C.J
        assert!(matches!(parse_c_quadrant_one(0xC089), Ok(Format::BType { .. })));      // C.BEQZ
        assert!(matches!(parse_c_quadrant_one(0xE089), Ok(Format::BType { .. })));      // C.BNEZ
    }

    // --- top-level entry point ---

    #[test]
    fn test_parse_c_inst_routes_by_quadrant() {
        assert!(matches!(parse_c_inst(Instruction(0x006C, ByteType::HalfWord)), Ok(Format::AluImmType { .. }))); // quadrant 0
        assert!(matches!(parse_c_inst(Instruction(0x0515, ByteType::HalfWord)), Ok(Format::AluImmType { .. }))); // quadrant 1
        assert!(matches!(parse_c_inst(Instruction(0x8195, ByteType::HalfWord)), Ok(Format::IShiftType { .. }))); // quadrant 2
    }

    #[test]
    fn test_parse_c_inst_reserved_quadrant_is_illegal() {
        // inst[1:0] == 11 means "not actually compressed" -- the fetcher
        // should never hand this to parse_c_inst, but the function must
        // still fail closed if it happens
        let result = parse_c_inst(Instruction(0x0003, ByteType::HalfWord));
        assert_eq!(result, Err(TrapCause::IllegalInstruction { instruction: Some(0x0003) }));
    }
}
