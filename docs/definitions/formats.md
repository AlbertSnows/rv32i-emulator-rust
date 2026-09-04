# Instruction Formats

Every RV32I instruction is 32 bits. To keep decoding simple in
hardware (and in this emulator), the same fields land in the same bit
positions across as many formats as possible:

- `opcode` is always bits `6:0`.
- `rd` (when present) is always bits `11:7`.
- `funct3` (when present) is always bits `14:12`.
- `rs1` (when present) is always bits `19:15`.
- `rs2` (when present) is always bits `24:20`.

Only the immediate's shape changes per format — and even then, the
immediate's *low-order* bits tend to reuse the same wire positions
across formats where possible. There are six base formats.

## R-type (register-register)

```
31        25 24   20 19   15 14  12 11    7 6      0
| funct7    | rs2   | rs1   | funct3| rd    | opcode |
```

No immediate — both operands come from registers. Used for
register-register ALU ops (`ADD`, `SUB`, `AND`, `OR`, `XOR`, `SLL`,
`SRL`, `SRA`, `SLT`, `SLTU`) and the `M`-extension multiply/divide
instructions. `funct7` mostly just distinguishes `ADD` from `SUB` and
`SRL` from `SRA` — everything else is pinned down by `funct3` alone.

Example: `add x1, x2, x3` (`x1 = x2 + x3`)

`funct7=0000000 rs2=00011 rs1=00010 funct3=000 rd=00001 opcode=0110011`
→ `0x003100B3`

## I-type (immediate / loads / jalr)

```
31            20 19   15 14  12 11    7 6      0
| imm[11:0]     | rs1   | funct3| rd    | opcode |
```

A 12-bit sign-extended immediate, plus one register operand. This one
format covers several semantically different instruction groups that
all happen to share the same wire shape: ALU-immediate ops (`ADDI`,
`SLTI`, `ANDI`, ...), shifts-by-immediate (`SLLI`/`SRLI`/`SRAI`, which
repurpose part of the immediate field as `shamt` plus a `funct7`-like
tag bit), loads (`LB`/`LH`/`LW`/...), `JALR`, and `ECALL`/`EBREAK`/CSR
instructions. This project's `Format` enum splits these into separate
variants (`AluImmType`, `IShiftType`, `LoadType`, `JalrType`,
`SystemType`, `CsrType`) even though they share one wire encoding,
because their *execution* semantics have nothing in common beyond the
decode shape.

Example: `addi x1, x2, 5` (`x1 = x2 + 5`)

`imm=000000000101 rs1=00010 funct3=000 rd=00001 opcode=0010011`
→ `0x00510093`

## S-type (store)

```
31        25 24   20 19   15 14  12 11    7 6      0
| imm[11:5] | rs2   | rs1   | funct3| imm[4:0]| opcode |
```

Stores need *two* register operands (the base address in `rs1`, the
value to write in `rs2`) but produce no result, so there's no `rd`
field to spare — the immediate gets split into a high chunk
(`imm[11:5]`, sharing R-type's `funct7` position) and a low chunk
(`imm[4:0]`, sharing R-type's `rd` position) so `rs1`/`rs2`/`funct3`
still land in their usual spots.

Example: `sw x3, 8(x2)` (store `x3` to address `x2 + 8`)

`imm[11:5]=0000000 rs2=00011 rs1=00010 funct3=010 imm[4:0]=01000 opcode=0100011`
→ `0x00312423`

## B-type (branch)

```
31 30      25 24   20 19   15 14  12 11    8 7 6      0
|12| imm[10:5]| rs2   | rs1   | funct3| imm[4:1]|11| opcode |
```

Same register layout as S-type (`rs1`/`rs2` are the two values being
compared), but the immediate encodes a branch *offset*, always an even
number of bytes — so bit 0 is never stored at all (it's implicitly
0), buying one extra bit of range for free. The remaining 12 stored
bits are scattered out of numerical order (`imm[12]`, then
`imm[10:5]`, then later `imm[4:1]`, then `imm[11]`) specifically so
that the sign bit (`imm[12]`) always lands at the same wire position
(bit 31) as every other format's sign bit — hardware can sign-extend
an immediate before it even knows which format it's decoding.

Example: `beq x1, x2, 16` (branch to `PC + 16` if `x1 == x2`)

offset 16 as a 13-bit value: bit 12=`0`, bits 10:5=`000000`,
bits 4:1=`1000`, bit 11=`0`
→ `imm[12]=0 imm[10:5]=000000 rs2=00010 rs1=00001 funct3=000 imm[4:1]=1000 imm[11]=0 opcode=1100011`
→ `0x00208863`

## U-type (upper immediate)

```
31                    12 11    7 6      0
| imm[31:12]            | rd    | opcode |
```

The simplest format: one 20-bit immediate occupying the *upper* 20
bits of a value, one destination register, no source registers at
all. Used for `LUI` (load the immediate into the upper 20 bits,
zeroing the lower 12) and `AUIPC` (same, but added to `PC`) — both
exist specifically to let a 32-bit constant or address be built in two
instructions (a `U`-type for the top 20 bits, an `I`-type `ADDI`/load
for the bottom 12), since no format has room for a full 32-bit
immediate.

Example: `lui x1, 0x12345` (`x1 = 0x12345 << 12 = 0x12345000`)

`imm=0x12345 rd=00001 opcode=0110111`
→ `0x123450B7`

## J-type (jump)

```
31 30        21 20 19          12 11    7 6      0
|20| imm[10:1] |11| imm[19:12]   | rd    | opcode |
```

Used only by `JAL`. Like B-type, the offset is always even (bit 0
implicit), and the sign bit (`imm[20]`) sits at bit 31 for the same
uniform-sign-extension reason. Unlike every other format, `JAL` has no
source registers at all — the jump target is `PC + offset`, computed
entirely from the immediate — so bits 19:12, which would otherwise be
wasted padding, are packed with immediate bits instead (this is also
exactly why those bits, and only those bits, are cheap for `AUIPC` to
share the position of: an `AUIPC`+`JALR` pair can synthesize a
branch anywhere in the 32-bit address space).

Example: `jal x1, 8` (jump to `PC + 8`, save return address in `x1`)

offset 8 as a 21-bit value: bit 20=`0`, bits 10:1=`0000000100`,
bit 11=`0`, bits 19:12=`00000000`
→ `imm[20]=0 imm[10:1]=0000000100 imm[11]=0 imm[19:12]=00000000 rd=00001 opcode=1101111`
→ `0x008000EF`

## Compressed (`C` extension) formats

The `C` extension adds nine more, much smaller, 16-bit formats (`CR`,
`CI`, `CSS`, `CIW`, `CL`, `CS`, `CA`, `CB`, `CJ`) that decode-time
-expand back onto the six formats above. `docs/plans/c_extension.md`
covers the *why* (decode-time expansion, no new execution code, which
formats use the full 5-bit register space vs. the cramped 3-bit
`x8`-`x15` one) — the actual per-instruction bit layouts and scrambled
-immediate reassembly aren't written up separately; they live as the
real reference in each `parse_c_*` function's own code and tests in
`src/cpu/instructions/c.rs`.
