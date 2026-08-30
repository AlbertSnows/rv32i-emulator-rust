# The `mcounteren`/`scounteren` masking bug (Zicsr/Zicntr arch-test failures)

## Summary

Running the `Zicsr`/`Zicntr` arch-test suites against this emulator
initially produced 6 failures out of 71, all on CSR instructions
(`csrrc`/`csrrs`/`csrrci`/`csrrsi`) touching `instret`. The baked
"expected" value in the self-checking ELF was garbage (e.g.
`0x02ca56cc` where the test logic can only ever produce a small
number). Root cause: `tests/arch_test_config/sail.json`'s
`mcounteren_writable_bits` and `scounteren_writable_bits` were both
left at `0x0`, copied verbatim from an example config and never
edited. That silently zeroed every write to `mcounteren`/`scounteren`,
making `cycle`/`time`/`instret` illegal to read from S-mode in sail's
own reference run — the "expected" value wasn't a computed answer at
all, it was leftover register state from a trap handler.

Fixed by setting both masks to `0x7`. Result: 65/71 → 69/71. The
remaining 2 failures (`Zicntr-csrrc-00`, `Zicntr-csrrs-00`) were a
separate, real, unrelated gap: `timeh` (CSR `0xC81`) had never been
implemented in this emulator at all. Fixed (see `plans.md` item #13) —
**71/71** now passes.

## How the failure looked

Test logic (`Zicsr-csrrs-00.S`, coverpoint `cp_rs1_b1`):

```asm
csrrs x1, instret, x0
csrrs x30, instret, x0
sub   x30, x30, x1        # difference should be a small number
```

This emulator computed `1` — correct, since exactly one instruction
(the second `csrrs`) retires between the two reads. The self-check
ELF's baked expected value was `0x02ca56cc`. That number looks like
sail computed something bizarre for this arithmetic, which is the
wrong lead to follow.

## Why the expected value was garbage: how ACT4 actually builds a test

ACT4 compiles each test twice:

1. A plain `-DSIGNATURE` build. `sail_riscv_sim` executes this one for
   real; whatever ends up in the signature region gets dumped to a
   `.sig` file. This is the *only* place sail's own execution matters.
2. A `-DRVTEST_SELFCHECK` build that `#include`s the `.sig` file's
   contents as compile-time constants and compares them against live
   values when the DUT runs it. There is no re-verification step —
   whatever sail produced in step 1 is trusted as ground truth.

So `0x02ca56cc` was never a considered answer to "what should
`instret`'s diff be." It was just whatever register `x30` held after
something else happened during sail's step-1 run.

## Finding the actual root cause

`DEBUG=True make -k sail` (the triage command AGENTS.md itself
recommends) regenerates a `.sig.trap_report` alongside the `.sig` file
— a plain log of every trap sail took during that golden run. For
`Zicsr-csrrs-00` it showed, as trap #0:

```
Trap #0: Exception
  Mode:    S/HS
  XCAUSE:  0x00000002  (Illegal instruction)
  XEPC:    0x800000a0  (Zicsr_csrrs_cg_cp_rs1_b1)
  XTVAL:   0xc02020f3  (csrrs x1, instret, x0)
```

Sail itself faulted on the very first `csrrs x1, instret, x0`, in
S-mode. `0x02ca56cc` was collateral damage from a trap handler running
and returning — not a computed answer at all.

That reframes the question from "why doesn't sail's arithmetic come
out to 1" (a dead end — there's no arithmetic bug to find) to "why does
sail think reading `instret` from S-mode is illegal."

## The actual gate: reading sail's own source

`model/core/sys_regs.sail` (riscv/sail-riscv) defines:

```
let sys_mcounteren_writable_bits : bits(32) = config base.mcounteren_writable_bits

function legalize_mcounteren(_c : Counteren, v : xlenbits) -> Counteren =
  Mk_Counteren(v[31 .. 0] & sys_mcounteren_writable_bits)

function clause write_CSR(0x306, value) = { mcounteren = legalize_mcounteren(mcounteren, value); ... }
```

Every write to `mcounteren` gets ANDed against a mask pulled straight
from `sail.json`'s `base.mcounteren_writable_bits` field. `scounteren`
works identically off `scounteren_writable_bits`. Access to
`cycle`/`time`/`instret` from a lower privilege mode is gated by these
registers (`model/extensions/Zicntr/zicntr_control.sail`,
`counter_enabled(index, priv)`).

`tests/arch_test_config/sail.json` had:

```json
"mcounteren_writable_bits": { "len": 32, "value": "0x0" },
"scounteren_writable_bits": { "len": 32, "value": "0x0" },
```

With the mask at `0x0`, the boot code's `csrw mcounteren, -1` /
`csrw scounteren, -1` (`tests/env/rvtest_setup.h`, "Enable all counters
for access from next lower priv mode") got legalized straight down to
`0`. From that point on, `cycle`/`time`/`instret` were illegal to read
from any mode below M — permanently, regardless of what this project's
UDB config (`MCOUNTENABLE_EN`/`SCOUNTENABLE_EN`, both already correctly
set to enable indices 0-2) said. **The UDB config and `sail.json` are
two separate inputs; setting one does nothing to the other** — the UDB
yaml only drives the ACT4 test *generator*, `sail.json` only configures
the reference model binary itself.

### The misleading comment

The neighboring field, `writable_hpm_counters`, has this comment:

> The top 29 bits in this value control whether the corresponding HPM
> counters ... are supported. ... The lowest 3 bits ... are ignored.

`mcounteren_writable_bits` and `scounteren_writable_bits` have no such
carve-out — they mask *all 32 bits*, including CY/TM/IR (0-2). Reading
them by analogy with their neighbor is exactly the wrong intuition,
and it's a large part of why this was easy to leave broken after
copying an example config.

## The fix

```json
"mcounteren_writable_bits": { "len": 32, "value": "0x7" },
"scounteren_writable_bits": { "len": 32, "value": "0x7" },
```

Also fixed a leftover `"Zihpm": {"supported": true}` in the same file
(found while reading the same block), contradicting the UDB config's
"no Zihpm, no HPM counters" declaration — unrelated to this bug, same
copy-paste-from-example origin.

### Why `0x7` specifically

`mcounteren`/`scounteren` are one bit per counter: bit 0 = CY (cycle),
bit 1 = TM (time), bit 2 = IR (instret), bits 3-31 = HPM counters
3-31. `0x7` is `...00111` — bits 0-2 set, everything else 0. Every
write gets ANDed against this mask, so bits 0-2 take whatever's
written (letting boot code's `-1` actually enable CY/TM/IR) while bits
3-31 stay pinned at 0 forever — matching "Zicntr yes, Zihpm/HPM
counters no" exactly. `0x0` meant no bit was ever writable, period.

## Result

65/71 → 69/71. Confirms this emulator's own `instret` computation
(`1`) was correct the whole time — the bug was entirely in this
project's `sail.json`, not in sail, not in riscv-arch-test, and not in
this emulator.

Checked upstream first, per AGENTS.md's own troubleshooting order:
riscv-arch-test issues #1538/#1561 describe a *related* but
already-fixed bug (old `instret` tests compared the absolute value
instead of a two-read difference). The generated `.S` here already has
the fixed form (`sub x30,x30,x1`), confirming this was a fresh, local
config bug, not a recurrence of a known upstream one.

## Retrospective: what would have caught this faster

Two things were already available and weren't used until late in the
investigation:

1. **AGENTS.md's own triage order was skipped.** It says: "Triage
   failures in this order: config/UDB mismatch, Sail config mismatch,
   generated objdump/trace, then DUT behavior." The investigation
   instead jumped to guessing DUT/generator behavior (an early
   hypothesis about unimplemented `fflags`/`vxsat`) before ever diffing
   `sail.json` against the UDB config — step 4 was tried before step
   2. Following that order literally, in order, would have pointed at
   `sail.json` immediately.

2. **`--trace` on the boot sequence showed the bug directly, in one
   line** — the same technique already used once before, for an
   earlier mtvec/infinite-loop bug in this same adoption effort:
   `csrw mcounteren, t0` immediately followed by
   `CSR mcounteren (0x306) <- 0x00000000`. Any time a boot-code CSR
   write and its traced result differ, that's the generic signature of
   a WARL/legalize mask silently eating the write. Worth a standing
   habit — "does every early-boot CSR write in `--trace` actually
   stick?" — rather than something noticed only after already
   suspecting counters specifically.

General lesson: every field left at a copied-in template value is an
unverified assumption, and the ones most likely to bite are exactly
the ones that gate behavior invisibly (`*_writable_bits`, reset
values) rather than declaring a feature on/off via an obviously-named
boolean. Worth a deliberate line-by-line review of a copied
`sail.json` immediately after copying it, not just editing the fields
remembered to matter.
