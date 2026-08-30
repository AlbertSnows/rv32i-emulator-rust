# Interrupts — why and how

## What's involved

Two new machine-level CSRs (`riscv_privleged.txt`, CSR address table, p.9):

| Address | Type | Name  | Purpose                          |
| ------- | ---- | ----- | -------------------------------- |
| `0x304` | MRW  | `mie` | Machine interrupt-enable register. |
| `0x344` | MRW  | `mip` | Machine interrupt-pending register. |

Both share the same top nibble (`0x3`). `mip` has some bits that are
read-only even though the register as a whole is MRW (see "design
wrinkles" below).

A third piece, `mtime`/`mtimecmp`, is NOT a CSR at all. Section 3.2.1
(p.59-60):

> "Platforms provide a real-time counter, exposed as a memory-mapped
> machine-mode read-write register, mtime... Platforms provide a 64-bit
> memory-mapped machine-mode timer compare register (mtimecmp)."

These are platform-defined memory addresses, not part of the CSR
address space.

## When does an interrupt actually fire?

Section 3.1.9 (p.43-44), the exact three conditions, all of which must
hold:

> "(a) either the current privilege mode is M and the MIE bit in the
> mstatus register is set, or the current privilege mode has less
> privilege than M-mode; (b) bit i is set in both mip and mie; and (c) if
> register mideleg exists, bit i is not set in mideleg."

We're M-mode only right now (no S-mode), so (c) doesn't apply yet (no
`mideleg`). (a) reduces to: `mstatus.MIE` must be 1 (since everything we
run is M-mode, there's no lower-privilege mode to trivially satisfy the
"current mode has less privilege than M" branch). (b) is the actual
per-source enable/pending check.

This can't be checked only reactively. The same section:

> "These conditions... must be evaluated in a bounded amount of time
> from when an interrupt becomes... pending... and must also be
> evaluated immediately following the execution of an xRET instruction
> or an explicit write to a CSR on which these interrupt trap conditions
> expressly depend."

In other words, an interrupt can become deliverable between two
instructions that have nothing to do with it — `step()` needs its own
proactive check, not just `perform_step()`'s existing `Err` path.

Priority, when more than one interrupt is simultaneously pending and
enabled (p.45): "MEI, MSI, MTI, SEI, SSI, STI, LCOFI" (decreasing
priority). We won't have simultaneous sources for a while, but worth
knowing the order exists.

## Which bits, which sources

Table 16 (p.49-50), the Interrupt=1 half — bit i in `mcause` corresponds
to bit i in both `mip` and `mie`:

| Bit | Source                                                  |
| --- | -------------------------------------------------------- |
| 1   | Supervisor software interrupt (SSI)                       |
| 3   | Machine software interrupt (MSI)                           |
| 5   | Supervisor timer interrupt (STI)                            |
| 7   | Machine timer interrupt (MTI)                                |
| 9   | Supervisor external interrupt (SEI)                            |
| 11  | Machine external interrupt (MEI)                                 |
| 13  | Counter-overflow interrupt (LCOFI, needs Sscofpmf, not relevant)   |
| >=16 | platform use                                                      |

Since S-mode isn't implemented:

> "If supervisor mode is not implemented, bits SEIP, STIP, and SSIP of
> mip and SEIE, STIE, and SSIE of mie are read-only zeros" (p.44-45)

— so bits 1/5/9 are hardwired 0 for now.

That leaves bit 3 (MSI), bit 7 (MTI), bit 11 (MEI) as the realistic set.
MSI needs inter-hart signaling, irrelevant to a single-hart emulator,
defer indefinitely. MEI needs a real device generating external
interrupts, nothing does yet, defer until peripherals exist. MTI
(`mtime >= mtimecmp`) is the one with an actual forcing function right
now — it's what Linux's scheduler needs. **First concrete interrupt
source to implement: bit 7, MTI.**

## `mcause` encoding for interrupts

Section 3.1.15 (p.48-49):

> "The Interrupt bit in the mcause register is set if the trap was
> caused by an interrupt."

That's the MSB — bit 31 on RV32. So an interrupt's full `mcause` value
isn't just the Table 16 code (e.g. 7 for MTI) — it's that code with bit
31 also set (`0x8000_0007` for MTI), distinguishing it from an ordinary
exception with the same low bits. `TrapCause::mcause_code()` currently
only returns the low, exception-code half (see
`src/definitions/trap_cause.rs`) — every existing variant is an
exception, so this was never needed before. Adding interrupt variants
means deciding where the interrupt bit gets OR'd in — inside
`mcause_code()` itself (cleanest, keeps the encoding logic in one
place), or wherever the interrupt is raised.

## The `mstatus` MIE/MPIE dance: half-done

Section 3.1.6.1 (p.31-32), trap entry:

> "When a trap is taken from privilege mode y into privilege mode x,
> xPIE is set to the value of xIE; xIE is set to 0; and xPP is set to
> y."

Our current `set_mpp()` (`core.rs`) only does the xPP part — it
captures the pre-trap mode into MPP, but never touches MIE/MPIE at all.
This was fine before, because nothing checked MIE, so a stale/uncleared
MIE had no observable effect. It matters now: if MIE never gets cleared
on trap entry, and MPIE never captures what MIE was, then MRET's
existing restore step (`inst_i_mret` already does `mstatus` MIE=MPIE,
MPIE=1 — see "MRET then in mstatus/mstatush sets... MIE=MPIE, and
MPIE=1", same section) ends up restoring garbage. The masks already
exist (`masks::MIE`, `masks::MPIE`, `src/definitions/masks.rs`) and are
already used by `inst_i_mret` — what's missing is the entry-side
counterpart, which would presumably live alongside `set_mpp`'s existing
MPP-capture logic in `core.rs`.
