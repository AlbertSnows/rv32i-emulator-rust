# Bug Log

This is to keep track of bugs I encountered while developing

## List

Found while working on the timer/UART chain, deferred briefly since
they weren't blocking, then fixed on 2026-09-04:

- **`src/cpu/definitions/cpu/csr.rs`**,  `guest_write`'s `SIP` arm had
  the *exact* bug the `MIP` arm had before today's earlier fix: it
  computed a masked return value but never assigned to `*property`, so
  any S-mode write to its own `sip` (e.g. clearing `SSIP` for a
  software interrupt) was silently discarded. Fixed the same shape as
  `MIP`'s arm (mask `value` to the writable bits via the new
  `masks::PER_SOURCE_SIP`, merge with the preserved bits, assign back).
- **`src/cpu/definitions/cpu/bus.rs`**,  `direct_write`'s out-of-bounds
  catch-all returned `TrapCause::LoadAccessFault` on the *write* path;
  now correctly returns `StoreAccessFault`. The one test asserting the
  old (wrong) fault type was updated, not deleted,  its actual intent
  (an out-of-range write falls through cleanly rather than being
  swallowed as a UART access) still holds.


- **The `in_trap` bug itself.** Why a single boolean can't represent
  trap nesting; why real hardware doesn't need an equivalent concept
  at all (`sstatus.SIE`/`mstatus.MIE` already do the whole job); why
  cross-privilege-level nesting (an M-mode interrupt firing while
  S-mode code, including an S-mode handler, runs) is safe without any
  extra bookkeeping (separate `mepc`/`mcause` vs `sepc`/`scause`) while
  same-level nesting isn't. This is genuinely one of the harder
  pieces of RISC-V privileged-mode reasoning in the whole project,
  worth a real walkthrough with a concrete timeline diagram of the
  nested-trap corruption, not just "we removed a line and it worked."


`tests/harness.rs`'s `test_rv32ui_p_add_passes` now
genuinely passes. Getting there surfaced four separate,
previously-invisible bugs in a row, each found by tracing a real
riscv-tests binary with temporary debug prints (same pattern every
time, reverted after each diagnosis):

1. `mhartid` (CSR `0xF14`) unmapped. Real boot code reads it
   immediately after zeroing registers (`csrr a0,mhartid; bnez
   a0,<spin>`,  "am I hart 0" multi-hart check), *before* installing
   its own trap vector,  the resulting trap had nowhere valid to go,
   landing pc at 0. Fixed: added to `CSRState`, read-only, always 0
   (single-hart emulator).

2. The `in_trap` double-trap design itself was too broad (see the
   "backlog: mhartid" section this replaced, and the design discussion
   around it),  real boot code deliberately traps to probe optional
   CSRs (`mnstatus`, `satp`, `pmpaddr0`, `pmpcfg0`) without ever
   running MRET, which the old "any second trap halts" rule couldn't
   tell apart from a genuine double-fault. Fixed: `step()`'s interrupt
   check gained `&& !cpu.flags.in_trap` (defer the interrupt, don't
   refire it); `handle_trap` lost its blanket `in_trap` check entirely
   (synchronous traps now nest freely, matching real hardware,
   overwriting `mepc`/`mcause` is fine, preserving it is software's
   job). Two existing tests had their *expectations* deliberately
   changed to match the corrected model, not just their setup:
   `test_handle_trap_returns_halt_on_double_trap` ->
   `test_handle_trap_allows_nested_synchronous_traps` (now expects
   `Continue` + overwritten `mtval`, not `Halt`);
   `test_interrupt_arriving_while_already_in_trap_halts` ->
   `test_step_defers_interrupt_while_already_in_trap` (now expects a
   real instruction to execute, not `Halt`).

3. MRET's own pc write was being silently overwritten. `perform_step()`
   calls `advance_pc()` unconditionally after every instruction; MRET
   sets pc directly (to `mepc`) inside its own `execute()`, but
   `advance_pc`'s default case then added 4 on top, since MRET isn't
   one of its special-cased `Format` variants (`JType`/`JalrType`/`BType`).
   Landed on `mepc+4` instead of `mepc` every time,  skipped the first
   instruction of wherever MRET returned to. Invisible before now
   because no earlier test checked pc's value after a real MRET. Fixed:
   added `Format::SystemType { op: SystemOp::MRet } => pc_value` (no
   addition) to `advance_pc`'s match, same pattern as the other three.

4. `inst_s_sb`/`sh`/`sw` (`s.rs`) wrote `rs2`,  the *register index*,
   directly, instead of `reg_file.read(rs2)`,  the register's actual
   *value*. Real, pervasive bug in every store instruction, invisible
   until now because the existing unit tests shared the identical
   wrong assumption (they passed `rs2` as if it were already a value,
   e.g. `let rs2 = 0x12345678`, rather than a register index with a
   value written into it first),  implementation and its own tests
   agreed with each other, so nothing caught it. This is exactly the
   failure mode adopting riscv-tests was meant to catch: an
   independently-authored suite has no way to inherit a codebase's own
   blind spot. Fixed both the implementation and all 7 affected tests
   in `s.rs`.

- `jalr`: `Fail(3)`. Same class of risk flagged when `advance_pc`'s
  `pc_value` cast got fixed from `i32` to `u32`,  `JalrType`'s arm does
  `(rs1_val as i32).wrapping_add(*imm)`, its own separate signed cast
  that may need the identical bit-reinterpretation treatment
  (`rs1_val.wrapping_add(*imm as u32)`, no `as i32` at all) rather than
  the same fix having been applied there too. Not yet confirmed,
  worth checking directly rather than assuming.
- `ma_data`: `Fail(668)`. Misaligned-access behavior,  flagged earlier
  (see the 42-file listing note above) as needing a real decision
  about how this emulator handles unaligned loads/stores, not
  necessarily a small bug.