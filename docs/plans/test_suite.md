# Test suite adoption — toolchain, ELF loader, tohost signaling

## Why

Everything tested so far is hand-written: `store_in_mem` pokes raw
instruction bytes into memory directly from Rust test code. That's fine
for the specific behaviors this project has built deliberately, but it
means instruction coverage is only as broad as whatever's been thought
to write a test for. riscv-tests is a real, established suite covering
every RV32I instruction; running it is a much stronger correctness
signal than continuing to hand-roll coverage one instruction at a time.

## 1. Cross-compiler toolchain

Need a RISC-V compiler capable of building riscv-tests' assembly
sources into ELF binaries. Two real options, and one real pitfall.

Build from source: riscv-gnu-toolchain
(github.com/riscv-collab/riscv-gnu-toolchain) is the canonical source.

### 1a. Toolchain: installed locally (done)

Refer to guide doc.

## 2. Minimal ELF32 loading

riscv-tests binaries are simple, statically-linked, non-relocatable
RV32 ELFs. A minimal loader for this specific case needs:

- ELF header: `e_entry` (the initial pc — `_start`'s address), `e_phoff`
  (byte offset to the program header table), `e_phnum` (how many
  entries), `e_phentsize` (size of each entry).
- Program header table: walk `e_phnum` entries starting at `e_phoff`.
  For each entry where `p_type == PT_LOAD`, that segment needs loading:
  read `p_filesz` bytes from the file at `p_offset`, write them into
  the emulator's memory starting at `p_vaddr`.
- `p_memsz` vs `p_filesz`: `p_memsz` can be *larger* than `p_filesz`.
  The extra bytes (`p_memsz - p_filesz`) must be zero-filled in memory
  but aren't present in the file at all — this is `.bss`
  (uninitialized global/static data), doesn't need file space since
  it's all zeros anyway.
- Setting the initial pc: after loading every `PT_LOAD` segment, set
  `cpu.pc` to `e_entry` before starting execution.

Whole minimal path: parse header -> find `PT_LOAD` segments -> copy
file bytes to `p_vaddr`, zero-pad to `p_memsz` -> set `pc = e_entry`.
No relocation, no dynamic linking, no symbol resolution needed for
loading itself, these are static, position-fixed binaries. (Symbol
table lookup IS still needed separately, for finding `tohost`'s
address — see below.)

## 3. The tohost/fromhost signaling convention

- `gp` (x3) is repurposed as `TESTNUM`, a running test counter
  (`#define TESTNUM gp`).
- `RVTEST_PASS`: sets `TESTNUM = 1`, then does
  `li a7, 93; li a0, 0; ecall` — an ecall with syscall number 93 (the
  exit syscall convention) and exit code 0.
- `RVTEST_FAIL`: `TESTNUM = (TESTNUM << 1) | 1` (encodes which numbered
  test failed as an odd number), then the same ecall pattern with
  `a0=TESTNUM`. The harness reverses this to recover which sub-test
  failed: `tohost_value >> 1` (`>>` discards whatever bit falls off the
  end, so it doesn't matter that `| 1` forced that bit to 1 — `>> 1`
  and `<< 1` are exact inverses of each other otherwise). This is also
  why `TEST_RR_OP` numbering in files like `add.S` starts at 2, not 0
  or 1 — `(0 << 1) | 1 == 1`, the exact same value `RVTEST_PASS` writes
  on success, so sub-test 0 failing would be indistinguishable from the
  whole file passing.
- The ecall gets caught by the test's own `trap_vector` (installed at
  boot via `csrw mtvec, t0`): checks `mcause` against the three ecall
  causes (`CAUSE_USER_ECALL` / `SUPERVISOR_ECALL` / `MACHINE_ECALL`)
  and, if matched, jumps to `write_tohost`.
- `write_tohost`:
  `sw TESTNUM, tohost, t5; sw zero, tohost+4, t5; j write_tohost` —
  writes `TESTNUM` (1 for pass, the odd encoded value for fail) into
  the `tohost` symbol, zeroes the upper 32 bits, then spins forever.
- `tohost`/`fromhost` themselves: declared as 8-byte (`.dword`),
  64-byte-aligned symbols in a dedicated `.tohost` linker section (the
  `RVTEST_DATA_BEGIN` macro). Their actual runtime address is wherever
  the linker script places that section — not a fixed constant in the
  header — so a loader needs to resolve `tohost`'s address from the
  compiled ELF's *symbol table*, not assume a hardcoded number.

Important simplification: this does NOT need special in-CPU
memory-mapped device logic the way `mtime`/`mtimecmp` did. The `sw`
instructions writing to `tohost` are ordinary memory writes from the
CPU's perspective — nothing needs special-casing in
`read_bytes`/`write_bytes`. The "specialness" lives entirely in the
external test harness: after each `step()` (or every N steps), the
harness reads memory at `tohost`'s address and checks if it's become
nonzero — if so, the test finished (the odd/even encoding indicates
pass vs. which numbered test failed). The test binary itself just
writes and spins forever; watching for that is the harness's job, not
the CPU's.

### 3a. Section header entry layout (for symbol lookup)

Program headers describe segments (what `load_elf` needs — loadable
memory regions). Symbol/string tables are a different thing entirely:
sections, described by a separate table (the section header table,
located via `e_shoff`/`e_shnum`/`e_shentsize` in the ELF header, same
pattern as `e_phoff`/`e_phnum`/`e_phentsize` but for sections). Each
entry is 40 bytes:

| Field | `sh_name` | `sh_type` | `sh_flags` | `sh_addr` | `sh_offset` | `sh_size` | `sh_link` | `sh_info` | `sh_addralign` | `sh_entsize` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| offset | 0 | 4 | 8 | 12 | 16 | 20 | 24 | 28 | 32 | 36 |
| size (bytes) | 4 | 4 | 4 | 4 | 4 | 4 | 4 | 4 | 4 | 4 |

Only four of these matter for `find_symbol`:

- `sh_type`: 2 = `SHT_SYMTAB` (the symbol table), 3 = `SHT_STRTAB` (a
  string table). Walk all `e_shnum` entries looking for
  `sh_type == 2`.
- `sh_offset`: once the `SHT_SYMTAB` entry is found, this says where
  the actual symbol records live in the file (a separate region from
  the section header table itself).
- `sh_entsize` / `sh_size`: `sh_size / sh_entsize` gives the number of
  symbol records (each record is 16 bytes: `st_name` u32 @0,
  `st_value` u32 @4, `st_size` u32 @8, `st_info` u8 @12, `st_other` u8
  @13, `st_shndx` u16 @14).
- `sh_link`: for the `SHT_SYMTAB` entry specifically, this is the
  section *index* (not a byte offset) of the linked string table —
  jump straight to that entry (`e_shoff + sh_link * e_shentsize`), no
  searching needed, then read *its* `sh_offset` to find the raw name
  bytes. `st_name` in a symbol record is a byte offset into that
  string data, NUL-terminated — not a name itself, has to be resolved
  by reading forward from `strtab_offset + st_name` until a `\0` byte.

## Backlog: base address remapping (blocks steps 5-6)

Discovered while writing tests for `load_elf`/`find_symbol`: every real
riscv-tests binary links at `0x80000000` (`env/p/link.ld`'s doing,
confirmed across add/ma_data/lui/auipc/simple), but `mem.storage` is
only `FULL_MEM_SIZE` (65536) bytes — `load_elf` against a real binary
faults immediately, address way past the end of storage. Growing
storage to actually cover `0x80000000` (~2-4 GiB) isn't realistic — see
below.

The fix: treat `0x80000000` as a base offset (`BASE_ADDRESS`, already
added as a const in `memory.rs`) and translate real addresses down to
small array indices (`real_index = address - BASE_ADDRESS`) at the
exact point something touches storage, mirroring how a real memory
bus/interconnect routes a CPU-visible address to a small physical RAM
chip mapped starting at that address — not a workaround, a software
model of how real hardware already does this.

Superseded by a bigger decision (see "done: introduced BusState"
below): address translation doesn't belong inside `MemoryState` at
all — it belongs in a real Bus layer, mirroring how a CPU core never
knows how its own memory controller is wired, it just issues an
address. Where the translation still needs to happen, now that
`BusState` exists:

- `BUSState::direct_read` / `direct_write` (`bus.rs`), in the `_`
  fallthrough branch only — NOT the MTIME/MTIMECMP arms, which use
  their own fixed, much smaller addresses and must keep checking the
  untranslated address. Translate (`real_index = address - BASE_ADDRESS`)
  before delegating to `self.ram.read_bytes`/`write_bytes` —
  `MemoryState` itself stays completely unaware of `BASE_ADDRESS`, just
  a bounds-checked local array. Needs a lower-bound guard too: if
  `address < BASE_ADDRESS` (and it's not MTIME/MTIMECMP either), the
  subtraction would underflow (usize, panics) — that case needs to
  return a clean `TrapCause`, not attempt the subtraction.
- Not `fetch_word_from_memory` anymore — already routes through
  `bus.direct_read` (done, see below), so it gets this for free once
  the above lands. Nothing else needs touching.

Not yet done: growing storage further wasn't the answer either — see
"why not just make memory larger" reasoning below, which is also why
`Copy` got removed from `MemoryState`/`CPUState` (done, see next
section).

## Done: removed `Copy` from `MemoryState`/`CPUState`

`MemoryState` derived `Copy`, and so did `CPUState` (since it embeds
`MemoryState`) — meaning every assignment/pass/return of either
silently duplicated the *entire* storage array, with no visible cost at
the call site. Harmless at 4096/65536 bytes, but the wrong thing to
keep doing as storage potentially grows, and a real Rust anti-pattern
in general (`Copy` should signal "cheap," `Clone` should signal "this
might be expensive, and here's the explicit call site proving you meant
it"). Also matters given the distant boot-Linux goal
(`project_goal_boot_linux.md`) — copying all of memory on every
pass-by-value is obviously wrong once this is modeling something closer
to a real running OS.

Removed `Copy` from both derives (kept `Clone`). Turned out completely
painless — `cargo check --all-targets` and the full test suite both
stayed clean with zero changes needed elsewhere, since nearly
everything already takes `&CPUState`/`&mut CPUState` by reference
rather than by value.

## Done: introduced `BusState` (real bus/device separation)

Bigger call than just fixing the address gap: `CPUState` no longer
holds `MemoryState` directly. It holds `BusState`, which owns two
devices — `ram: MemoryState` (now just a bounds-checked byte array, no
address-range knowledge at all) and `clint: ClintState`
(`mtime`/`mtimecmp` + `update_time()`, moved out of `MemoryState` —
they're a separate device, CLINT, "Core-Local Interruptor," the real
SiFive/QEMU-virt hardware name, not invented for this project).
`BUSState::direct_read`/`direct_write` do the address-range routing
(MTIME/MTIMECMP vs RAM) that used to live inside `MemoryState`'s own
`read_bytes`/`write_bytes`.

Why: address decoding is a bus/interconnect responsibility on real
hardware, not something the CPU core or the RAM chip itself knows
about — modeling it as its own layer now, rather than patching
`BASE_ADDRESS` into `MemoryState` directly, means `MemoryState` can
stay genuinely dumb (no knowledge of any of this), and adding a third
memory-mapped device later (UART, PLIC, disk — already on the
distant-goal peripherals list) means adding a device + a routing arm,
not touching every existing one.

`fetch_word_from_memory` (`fetcher.rs`) now routes through
`bus.direct_read` instead of indexing storage directly — this was a
real fix, not optional, since it's what `perform_step()` calls every
instruction cycle; `BUSState` doesn't expose a bare storage field the
way `MemoryState` did. Every load/store instruction, `load_elf`, and
`store_in_mem`'s callers updated to go through `cpu.bus` instead of
`cpu.mem`. Full sweep, all 161+1+5 tests still pass.

## Done: `BASE_ADDRESS` actually wired in, plus a real bug it exposed

`BUSState::direct_read`/`direct_write`'s `_` arms now guard (address
must be `>= BASE_ADDRESS`, else a clean `TrapCause` instead of a
`usize` underflow panic) and translate before delegating to `self.ram`.
`ClintState` got its own
`read_mtime`/`read_mtimecmp`/`write_mtime`/`write_mtimecmp`, with the
offset+width bounds check that `direct_read`/`direct_write`'s inline
MTIME/MTIMECMP handling never had (`offset + num_bytes > 8` now returns
`Err` instead of `extract_sub_bytes`/a byte-slice panicking past the
real 8-byte register).

This forced every test exercising a real memory access (load/store
instructions, fetcher, `core::step()`) to move off small hand-picked
addresses (0, 4, 10...) onto real `BASE_ADDRESS`-relative ones — a
deliberate call, not a compromise: `BUSState` now always translates,
with no small-address passthrough special-cased for test convenience,
since that would have meant production code quietly behaving
differently depending on what looked like "a test's address" vs "a
real one."

That sweep surfaced a real, previously-invisible bug: `advance_pc`
(`instructions/pc.rs`) cast pc to `i32` for signed immediate
arithmetic. `0x80000000` (`BASE_ADDRESS`) is exactly `i32::MIN`'s bit
pattern, so every real pc value started reading as a huge negative
number the moment real addresses were used — invisible before now
because every pc value in every test was always small enough to stay
positive as `i32`. Fixed by keeping pc arithmetic in `u32` throughout
(RV32 addresses are unsigned by definition; overflow correctly wraps at
2^32, not something to avoid) and bit-reinterpreting the signed
immediate into `u32` before adding (`imm as u32`, then always
`wrapping_add` — two's complement means this produces the identical
result a signed subtraction would, no branching on sign needed).

Verified end to end: `load_elf` against the real `add-p-add` fixture
now succeeds — pc lands at `0x80000000`, `tohost` resolves to
`0x80001000` (matching `readelf` from way back), real nonzero
instruction bytes sit at the entry point. This is the thing that's been
blocked the whole time outline steps 5-6 needed.

## Done: `rv32ui-p-add` passes for real (four real bugs found and fixed)

`tests/harness.rs`'s `test_rv32ui_p_add_passes` (a real, permanent
test, not scratch — kept deliberately red until fixed, per "why don't
we just write an actual test that'll keep failing" a while back) now
genuinely passes. Getting there surfaced four separate,
previously-invisible bugs in a row, each found by tracing a real
riscv-tests binary with temporary debug prints (same pattern every
time, reverted after each diagnosis):

1. `mhartid` (CSR `0xF14`) unmapped. Real boot code reads it
   immediately after zeroing registers (`csrr a0,mhartid; bnez
   a0,<spin>` — "am I hart 0" multi-hart check), *before* installing
   its own trap vector — the resulting trap had nowhere valid to go,
   landing pc at 0. Fixed: added to `CSRState`, read-only, always 0
   (single-hart emulator).

2. The `in_trap` double-trap design itself was too broad (see the
   "backlog: mhartid" section this replaced, and the design discussion
   around it) — real boot code deliberately traps to probe optional
   CSRs (`mnstatus`, `satp`, `pmpaddr0`, `pmpcfg0`) without ever
   running MRET, which the old "any second trap halts" rule couldn't
   tell apart from a genuine double-fault. Fixed: `step()`'s interrupt
   check gained `&& !cpu.flags.in_trap` (defer the interrupt, don't
   refire it); `handle_trap` lost its blanket `in_trap` check entirely
   (synchronous traps now nest freely, matching real hardware —
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
   Landed on `mepc+4` instead of `mepc` every time — skipped the first
   instruction of wherever MRET returned to. Invisible before now
   because no earlier test checked pc's value after a real MRET. Fixed:
   added `Format::SystemType { op: SystemOp::MRet } => pc_value` (no
   addition) to `advance_pc`'s match, same pattern as the other three.

4. `inst_s_sb`/`sh`/`sw` (`s.rs`) wrote `rs2` — the *register index* —
   directly, instead of `reg_file.read(rs2)` — the register's actual
   *value*. Real, pervasive bug in every store instruction, invisible
   until now because the existing unit tests shared the identical
   wrong assumption (they passed `rs2` as if it were already a value,
   e.g. `let rs2 = 0x12345678`, rather than a register index with a
   value written into it first) — implementation and its own tests
   agreed with each other, so nothing caught it. This is exactly the
   failure mode adopting riscv-tests was meant to catch: an
   independently-authored suite has no way to inherit a codebase's own
   blind spot. Fixed both the implementation and all 7 affected tests
   in `s.rs`.

## Done: `riscv_test!` macro, added a real fixture per rv32ui test file

`tests/harness.rs`'s three near-identical `#[test]` functions
(add/addi/beq) turned into one `macro_rules!`
(`riscv_test!(test_name, fixture_path)`) that expands to the same
`#[test] fn` — still individually named/reported in `cargo test`
output, just without retyping the boilerplate per file. All remaining
`rv32ui/*.S` files (39 of them) compiled the same way `add.S` was (same
flags, see "1a"/"3a" above), added to `tests/fixtures/`, and wired up
with one `riscv_test!` line each — 42 real riscv-tests binaries total
now running through this emulator.

Result: 40/42 pass. Two real, likely genuine bugs, not test issues:

- `jalr`: `Fail(3)`. Same class of risk flagged when `advance_pc`'s
  `pc_value` cast got fixed from `i32` to `u32` — `JalrType`'s arm does
  `(rs1_val as i32).wrapping_add(*imm)`, its own separate signed cast
  that may need the identical bit-reinterpretation treatment
  (`rs1_val.wrapping_add(*imm as u32)`, no `as i32` at all) rather than
  the same fix having been applied there too. Not yet confirmed —
  worth checking directly rather than assuming.
- `ma_data`: `Fail(668)`. Misaligned-access behavior — flagged earlier
  (see the 42-file listing note above) as needing a real decision
  about how this emulator handles unaligned loads/stores, not
  necessarily a small bug.

## Outline of steps, in dependency order

5. [x] Write the external test-harness loop — done, `tests/harness.rs`
   (`run_tests`: loads via `load_elf`, resolves `tohost` via
   `find_symbol`, runs `step()` in a bounded loop, polls `tohost`,
   decodes pass/fail/timeout).
6. [x] Full rv32ui suite wired up and running (42/42 binaries, 40
   passing). `jalr` and `ma_data` still failing — see above.
