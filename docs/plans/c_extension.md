# Plan: "C" Extension for Compressed Instructions

## Why this is happening now

This emulator has never implemented the C extension — RV32I plus M
(multiply/divide) and A (atomics) only. That was a deliberate,
low-cost choice for OpenSBI and the Linux kernel: both are built from
source under this project's own control, so their build systems could
simply be told not to emit compressed instructions
(`PLATFORM_RISCV_ISA`/`CONFIG_RISCV_ISA_C`, see
`docs/dev/boot_files_setup.md`).

Userspace software doesn't offer that escape hatch. Building even a
minimal busybox-based root filesystem (the current goal — reaching an
interactive shell) requires a real Linux userspace toolchain, and
every one actually available (Fedora's `riscv32-linux-gnu-gcc`, and by
extension any real distro's RISC-V toolchain) bundles prebuilt startup
glue (`crtbegin.o`, `libgcc.a`) compiled with the C extension
unconditionally — confirmed directly: `riscv32-linux-gnu-gcc
-print-multi-lib` offers exactly two 32-bit variants, `rv32imac` and
`rv32imafdc`, both including `c`. There is no `-march` flag that avoids
recompiling those pieces from source, and building an entire custom
GCC toolchain from source (musl-cross-make or similar) to dodge one
compiler flag is a bigger, less durable undertaking than implementing
the extension once, permanently, in the emulator itself. See the
session notes around 2026-09-03 for the full path that led here
(OpenSBI/kernel ISA narrowing, then hitting this exact wall for
busybox).

## Scope: what RV32C actually needs, for this emulator specifically

Source: `.claude/riscv-unprivileged.txt`, Chapter 28, "C" Extension for
Compressed Instructions, Version 2.0 (same local copy of the RISC-V
Unprivileged ISA manual already used elsewhere in this project's
docs).

The full RVC encoding space (Figures 3-5, §28.8, pp.164-166) includes
RV64-only forms (`C.LD`/`C.SD`/`C.ADDIW`/`C.ADDW`/`C.SUBW`/`C.LDSP`/
`C.SDSP`) and F/D-only forms (`C.FLD`/`C.FSD`/`C.FLW`/`C.FSW`/
`C.FLDSP`/`C.FSDSP`/`C.FLWSP`/`C.FSWSP`) — this emulator needs neither
(RV32 only, no floating-point). Excluding those, the real scope is
**24 instructions**, not the ~40 the full spec defines:

| Quadrant | Instructions |
|---|---|
| 0 (`inst[1:0]=00`) | `C.ADDI4SPN`, `C.LW`, `C.SW` |
| 1 (`inst[1:0]=01`) | `C.NOP`, `C.ADDI`, `C.JAL`, `C.LI`, `C.ADDI16SP`, `C.LUI`, `C.SRLI`, `C.SRAI`, `C.ANDI`, `C.SUB`, `C.XOR`, `C.OR`, `C.AND`, `C.J`, `C.BEQZ`, `C.BNEZ` |
| 2 (`inst[1:0]=10`) | `C.SLLI`, `C.LWSP`, `C.JR`, `C.MV`, `C.EBREAK`, `C.JALR`, `C.ADD`, `C.SWSP` |

`C.NOP` is really just `C.ADDI` with `rd=x0, imm=0` — same opcode,
degenerate case, not separate decode logic.

**HINTs (§28.7, p.163):** several code points in this list are defined
as HINTs when specific operands are used (e.g., `C.ADDI` with `rd≠0,
imm=0`; `C.LI`/`C.MV`/`C.ADD` with `rd=x0`) — architecturally required
to execute as ordinary no-op-equivalent instructions, not fault.
Expanding them through the normal instruction path (below) satisfies
this for free, since e.g. `C.ADDI rd=x0` naturally expands to `addi
x0, x0, 0`, which already behaves as a no-op through the existing
`AluImmType` execution path — x0 writes are already discarded
(`RegisterFile::write`, `cpu_definition.rs:86`). No special-casing
needed.

**Reserved/illegal code points** (all-zero instruction, RV64-only
opcodes when encountered in this RV32 emulator, etc.) should decode to
`TrapCause::IllegalInstruction`, matching how the existing 32-bit
decoder already handles unrecognized encodings.

## Architecture: decode-time expansion, zero new execution code

The spec's own design principle (§28.1, p.153): "RVC was designed
under the constraint that each RVC instruction expands into a single
32-bit instruction in the base ISA... hardware designs can simply
expand RVC instructions during decode." This project's existing
`Format` enum (`src/cpu/instructions/mod.rs`) already covers every
operation the 24-instruction scope above needs:

| RVC instruction(s) | Expands to existing `Format` variant |
|---|---|
| `C.LW`, `C.LWSP` | `LoadType` |
| `C.SW`, `C.SWSP` | `SType` |
| `C.ADDI`, `C.ADDI4SPN`, `C.ADDI16SP`, `C.LI`, `C.ANDI`, `C.NOP` | `AluImmType` |
| `C.SLLI`, `C.SRLI`, `C.SRAI` | `IShiftType` |
| `C.LUI` | `UType` |
| `C.J`, `C.JAL` | `JType` |
| `C.JR`, `C.JALR` | `JalrType` |
| `C.BEQZ`, `C.BNEZ` | `BType` |
| `C.SUB`, `C.XOR`, `C.OR`, `C.AND`, `C.MV`, `C.ADD` | `RType` |
| `C.EBREAK` | `SystemType` |

This means **no new execution logic, no new `Format` variants** — the
entire feature is a new decode path that produces the same `Format`
values the rest of the emulator already knows how to run correctly
(and which are already covered by the existing 191-test suite for
their execution semantics). The work is confined to:

1. Fetching correctly when an instruction might be 2 or 4 bytes.
2. A new decoder that reads a 16-bit compressed instruction and
   produces the right `Format` value, with immediates/registers
   correctly expanded per the tables in §28.3-28.5.
3. Advancing `pc` by 2 instead of 4 for compressed instructions.

### 1. Variable-width fetch

`fetch_word_from_memory` (`src/cpu/fetcher.rs`) currently always reads
a full 4-byte word via `bus.guest_fetch(pc_value, ByteType::Word.as_num(), ...)`.
With C enabled, "no instructions can raise instruction-address
-misaligned exceptions" and instructions may start on any 2-byte
boundary (§28.1, p.152, `IALIGN=16`) — fetch has to become: read a
16-bit halfword first, check its low 2 bits (`11` = this is actually
the first half of a 4-byte instruction, fetch the second halfword too;
anything else = this is a complete 2-byte instruction, decode it as
-is). This is the load-bearing check for the entire feature — every
other piece depends on knowing the instruction's real width before
trying to decode it.

### 2. New decoder module (`src/cpu/instructions/c.rs`, following the
existing per-format module convention — `a.rs`, `b.rs`, `s.rs`, etc.)

`decode_word_to_instruction` (`src/cpu/decoder.rs`) dispatches purely
on `opcode = mask(raw_word.0, masks::OP_CODE)` (bits 6:0) — for every
existing 32-bit instruction, bits `[1:0]` are always `11`. A new check
ahead of that dispatch (bits `[1:0] != 11`) routes to a new
`parse_c_inst`-equivalent function, which itself dispatches further on
`inst[15:13]` (funct3) and `inst[1:0]` (quadrant) per the opcode map
in Table 39 (§28.8, p.165) and the three per-quadrant figures that
follow it.

Two real subtleties to get right here, both already flagged in the
spec text:

- **Register field encoding differs by format.** `CR`/`CI`/`CSS` use
  the full 5-bit register space (any of x0-x31); `CIW`/`CL`/`CS`/`CA`/
  `CB` use a 3-bit field that maps to only 8 registers, x8-x15 (§28.2,
  p.154, Table 37) — i.e. `real_register = 8 + field_value`. Getting
  this mapping wrong for the wrong format is the single most likely
  source of a subtle bug here.
- **Immediates are bit-scrambled, not stored in order** (§28.2, p.154:
  "Immediate fields have been scrambled... to reduce the number of
  immediate multiplexers required"). Each instruction's own figure in
  §28.3-28.5 spells out exactly which source bit maps to which
  destination bit (e.g. `C.ADDI4SPN`'s `nzuimm[5:4|9:6|2|3]`) — these
  have to be reassembled bit-by-bit per instruction; there's no
  shortcut, and copy-pasting one instruction's shuffle pattern for
  another will silently miscompute (already the shape of bug that
  turned up repeatedly with the DTB's own byte-swap logic earlier this
  session).

### 3. PC advancement

`advance_pc` (`src/cpu/instructions/pc.rs:8`) hardcodes `+4` as the
fallthrough amount for every format that doesn't branch/jump (and as
the "not taken" case for `BType`). This needs to become "+2 or +4"
depending on the real instruction width — the cleanest approach is
probably passing the actual instruction length (2 or 4) into
`advance_pc` alongside the already-expanded `Format`, since by the
time `advance_pc` runs, the original 16-vs-32-bit distinction has
already been erased by step 2's expansion. Jump/branch target
computation itself (`pc_value.wrapping_add(imm)`) doesn't need to
change — compressed jump/branch immediates already encode the real
byte offset, so once expanded into `JType`/`BType`/`JalrType` they
behave identically to their 32-bit counterparts.

## Suggested implementation order


2. Quadrant 0 (`C.ADDI4SPN`, `C.LW`, `C.SW`) — smallest quadrant, only
   two formats (`CIW`, `CL`/`CS`), good first slice to validate the
   general approach (bit-scrambled immediate reassembly, x8-x15
   mapping) before the bigger quadrants.
3. Quadrant 2 (`C.SLLI`, `C.LWSP`, `C.JR`, `C.MV`, `C.EBREAK`,
   `C.JALR`, `C.ADD`, `C.SWSP`) — uses full 5-bit registers throughout
   (`CR`/`CI`/`CSS`), no x8-x15 mapping needed, a useful contrast case.
4. Quadrant 1 (the largest and most varied — constant generation,
   register-immediate ops, the `MISC-ALU` sub-group, jumps, branches).
5. PC-advancement fix in `pc.rs`, wiring the real instruction length
   through.
6. HINT/reserved/illegal code-point behavior — confirm via targeted
   tests that these fall out correctly from the expansion approach
   rather than needing special-casing (see the HINT note above).

## Testing

Every entry in Figures 3-5 has a fully worked example in the spec
text's prose (e.g. "`C.LW` loads a 32-bit value... expands to `lw rd',
offset(rs1')`") — cross-referencing this project's own established
pattern (`programs/instructions.rs`-style raw hex constants exercised
against the real decoder, matching how the base ISA's own tests work)
is the most direct way to build confidence per-instruction, rather
than relying on the eventual real-Linux-boot test alone. The riscv-
arch-test suite (already integrated, `docs/plans/arch_test.md`) does
**not** cover the C extension at all in this project's current 71/71
passing set — worth checking whether `riscv-arch-test` upstream has a
compressed-instruction test suite that could be pulled in the same way,
once the hand-written tests pass.

The real, end-to-end validation is still what triggered this work in
the first place: successfully compiling and running the busybox
-based initramfs this emulator currently can't execute at all.
