# Bug Log

Bugs found while building this project, kept here for the *why*
even after the code itself has moved on. Roughly chronological order.
The early ones were found fast, by tracing a failing test with
temporary debug prints. The later ones, once riscv-tests/riscv-
arch-test and then a real Linux boot were the thing being debugged
against, took much longer to run down,  some of those needed a
custom instrumented binary, billions of emulated steps, and
cross-checking against real QEMU/GDB before the real cause showed up.

## Early: first real riscv-tests pass (`rv32ui-p-add`)

Four separate, previously-invisible bugs found in a row while getting
the very first real riscv-tests binary to pass,  each one had been
sitting under identical wrong assumptions in this project's own
hand-written unit tests, so nothing had ever caught them:

1. **`mhartid` (CSR `0xF14`) unmapped.** Real boot code reads it
   immediately after zeroing registers ("am I hart 0?"), before
   installing its own trap vector,  the resulting trap had nowhere
   valid to go, landing PC at 0. Fixed by adding it, read-only,
   always 0 (single-hart emulator).
2. **The first `in_trap` design was too broad.** Real boot code
   deliberately traps to probe optional CSRs without ever running
   `MRET`, which the original "any second trap halts" rule couldn't
   tell apart from a genuine double-fault. Fixed at the time by making
   `step()`'s interrupt check defer instead of firing
   (`&& !cpu.flags.in_trap`) and letting synchronous traps nest freely.
   (This design didn't survive,  see the real, much deeper `in_trap`
   bug below, found much later once real interrupts started arriving.)
3. **`MRET`'s own PC write was silently overwritten.** `advance_pc()`
   runs unconditionally after every instruction and didn't know about
   `MRET`; it added 4 on top of the PC `MRET` had just set, skipping
   the first instruction after every trap return. Invisible until now
   because no earlier test had checked PC after a real `MRET`.
4. **Store instructions wrote the register *index*, not its value.**
   `inst_s_sb`/`sh`/`sw` passed `rs2` (a register number) directly to
   memory instead of `reg_file.read(rs2)`. A real, pervasive bug in
   every store instruction,  invisible because the existing unit tests
   shared the identical wrong assumption, passing `rs2` as if it were
   already a value. This is exactly the failure mode adopting an
   independently-authored test suite is meant to catch.

## The interrupt/UART chain (reaching a real Linux boot)

Four bugs in sequence, each one only surfacing once the previous was
fixed and the boot got further than it ever had before. This whole
chain is what motivated building `src/bin/debug_boot.rs`,  a copy of
the boot loop instrumented with symbol resolution, a trap-cause tally,
and PLIC/CSR state dumps,  since the failures involved real elapsed
guest time (billions of steps) rather than anything a unit test could
reproduce directly.

**1. Missing supervisor-timer-interrupt handling.** `TrapCause` had no
`SupervisorTimerInterrupt` variant at all, so `select_pending_interrupt`
could never select one no matter what `mip`/`mie` said. Fixed by adding
the variant plus its three call sites (`select_pending_interrupt`'s
priority array, `handle_trap`'s two delegation matches). Adding it
in only one of those three spots would have looked like it worked
right up until the specific case the missing spot covered actually
came up,  trap delegation for a cause needs its `mideleg`/`medeleg`
mask *and* its priority-selection entry *and* its `tval`-clearing
arm, not just one of the three.

**2. `MIP`/`SIP` guest CSR writes were silently discarded.**
`CSRState::guest_write`'s `MIP` (and later, identically, `SIP`) arm
computed a correctly-masked return value but never assigned it back to
`*property`,  so any S-mode software write to its own pending-interrupt
bits (e.g. clearing `SSIP`) had no effect at all. This is precisely the
kind of bug the `CSRState` refactor item exists
because of: per-CSR special-casing in `guest_write` had already grown
enough that a no-op path like this could hide in it.

**3. SIGBUS after the timer fix, from stale pre-`C`-extension
assumptions.** Once interrupts actually started firing correctly,
`j.rs`/`jalr.rs`'s leftover `% 4` alignment checks and hardcoded
`pc+4` link-register writes,  both correct back when every instruction
was 4 bytes wide, both stale the moment the `C` extension made 2-byte
-aligned jump targets real,  started producing real crashes. Fixed by
consolidating jump-target computation *and* the link-register write
into `advance_pc` (which already had the correct `% 2` check and the
real `advance_amount`), rather than patching the two alignment checks
in place.

**4. Silent post-`/init` hang, no crash at all.** Two independent gaps
compounded: nothing anywhere called `receive_uart_byte` (host keyboard
input never reached the guest at all), and `UartState::read`'s LSR
(line status register) always reported a hardcoded "no data waiting"
byte regardless of the receive buffer's real state. Fixed by adding a
stdin-reader-thread/channel (`utility/host_io.rs`, shared by `run_os`
and `debug_boot`) feeding bytes in via `receive_uart_byte`, and making
the LSR read reflect the receive buffer's actual state.

## The `in_trap` nested-trap bug,  the hardest one

Even after all four fixes above, the boot still hung,  but now making
real, extremely slow progress instead of failing outright, which took
many billions of emulated steps and several separate background runs
to even characterize.

**First hypothesis, wrong:** `debug_boot.rs` grew its own external
replica of `check_interrupt`'s supervisor-external-interrupt condition
to log when an SEI *should* have fired, and that replica reported it
never did,  pointing at "SEI can never fire" as the culprit. That
replica turned out to have a bug of its own (most likely a wrong
`SSTATUS` address assumption baked into the diagnostic, not the
emulator), and chasing it wasted real time before being set aside as
unreliable.

**What actually cracked it:** a trap-cause tally (a `HashMap<(mode,
cause), count>` incremented every time a new trap was entered) showed
that *both* `SupervisorExternalInterrupt` and the previously-reliable
`SupervisorTimerInterrupt` froze at the same moment,  not that one
specific interrupt source was starved, but that interrupt delivery
*itself* stopped working, all at once, right around when the first
real PLIC-routed interrupt in the project's history occurred.

**The real mechanism:** `cpu.flags.in_trap` was a single boolean
standing in for "are we currently inside a trap handler." But an
S-mode interrupt handler making a routine SBI `ecall` is itself a
*nested* trap,  an inner M-mode trap running while the outer S-mode
handler hasn't finished. That inner trap's own `mret` unconditionally
cleared `in_trap` back to `false`, even though the outer handler was
still mid-flight,  reopening the window for a second interrupt to be
selected and delivered right on top of the first, clobbering the
shared `sepc`/`scause` pair the outer handler still needed. Once that
happened, both the trap-signature and trap-return state were
corrupted, and interrupt delivery stayed wedged permanently.

**Fix:** removed the `in_trap` gate from `select_pending_interrupt`
entirely, rather than replacing it with a nesting counter. Real
hardware doesn't need an equivalent mechanism at all, 
`sstatus.SIE`/`mstatus.MIE`, already correctly cleared on trap entry
via `set_pie`, are the actual, sufficient, already-implemented
protection against this exact class of corruption. The single boolean
was not just insufficiently precise, it was solving a problem real
hardware doesn't have. (`cpu.flags.in_trap` is still *written* by
`handle_trap`/`inst_i_xret`, but nothing reads it anymore,  an open
question on whether to remove it outright.)

Two smaller bugs turned up alongside this investigation, same shape as
the earlier `MIP` bug: `guest_write`'s `SIP` arm had the identical
discarded-write bug `MIP`'s arm had (masked value computed, never
assigned back,  silently discarding S-mode writes to its own pending
-interrupt bits), and `bus.rs`'s out-of-bounds `direct_write` catch-all
returned `LoadAccessFault` on the *write* path instead of
`StoreAccessFault`.

## `C.LUI` HINT treated as illegal (found via riscv-arch-test)

Found only once riscv-arch-test (ACT4) was extended to cover the `C`
extension,  96 of 97 generated tests passed immediately; the one
failure looked, from its own diagnostic output, like a privilege
-delegation bug ("trap was being handled in S-Mode", expected a
different trap-signature slot entirely).

That diagnosis was a red herring. The actual failing instruction was
`c.lui x0, ...`,  and per the RVC spec, `C.LUI` with `rd=x0` is a
**HINT**, not a reserved/illegal encoding: HINTs must decode and
execute without ever trapping. `parse_c_lui_or_addi16sp` conflated the
two, raising `IllegalInstruction` for `rd==0` alongside the genuinely
-reserved `imm==0` case. The test never expected any trap at all *for
this instruction*,  which privilege mode would have handled it was
never the real question; the bug was one layer up, in decode, not in
delegation. Fixed by only treating `imm==0` as reserved when `rd != 0`;
`rd==0` now falls through to a real (no-op, since register writes to
`x0` are already discarded) `LUI`.

This is the exact kind of bug the three-layer test strategy
(`docs/dev/README.md`'s Design Decisions) exists to catch: 239 unit
tests and riscv-tests' own compressed-instruction suite
(`rv32uc-p-rvc`) both passed the whole time, because neither happened
to construct this specific reserved-encoding corner case.
