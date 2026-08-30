# Hardware modes

## What are the modes?

- S
- M
- U

## Implementation steps

- add mode to cpu state
- add mode reader, mode writer

Default state: M mode.

## Move state

Questions:

- How does the cpu move state? What are all the ways it can move state?
  - MPP = machine previous Privilege
    - what is MPP?
      - 2 bits, inside `mstatus` (what is `mstatus`?)
      - parallel to `mepc`
      - MPP remembers what the cpu was in (M, S, U) when trap happens

### What is `mstatus`?

- a CSR
- same as `mepc`, `mcause`, etc.
- `mstatus` is a single 32-bit slot
- carved into many named fields
  - MIE
  - MPIE
  - MPP
- writing to MPP is messing with bits 11-12?

### Two means to update mode

**Trap entry**

- save the current mode into MPP
- set the current mode to M

**Trap return**

- `mret`
  - what is `mret`?
    - op code
    - same as `ecall`/`ebreak`
    - sets pc from `mepc`
    - sets privilege mode to MPP

#### Handle trap

- save mode to MPP
- switch to M

#### Return from trap

- `mret`
- restores the mode from MPP
- restores pc from `mepc`

### When it does move state, does anything change except the cpu state?

- no, just state

### What are the effects of moving state?

- CSR access control
  - reading/writing from CSR below required minimum privilege should
    raise `IllegalInstruction`
- `mret` legality
  - xRET instruction can be executed in privilege mode x or higher
  - if below x, illegal instruction exception
  - `mret` needs a mode check before it's allowed to run

## Notes from the book

> To support nested traps, each privilege mode x that can respond to
> interrupts has a two-level stack of interrupt-enable bits and
> privilege modes. xPIE holds the value of the interrupt-enable bit
> active prior to the trap, and xPP holds the previous privilege mode.
> The xPP fields can only hold privilege modes up to x, so MPP is two
> bits wide and SPP is one bit wide.
>
> When a trap is taken from privilege mode y into privilege mode x,
> xPIE is set to the value of xIE; xIE is set to 0; and xPP is set to y

xIE = x interrupt enabled. This is a bit, it answers this question: "is
this privilege level currently willing to be interrupted?"

xPIE = previous interrupt enable

- MPIE for now
- saves a record of xIE before xIE is changed
- meant to be a restore point for xIE

xPP = x previous privilege, maps onto MPP for us.
