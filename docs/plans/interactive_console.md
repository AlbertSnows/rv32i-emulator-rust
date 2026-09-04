# Reaching an interactive shell: what's left

## Where this comes from

Busybox now boots successfully as `/init` (PID 1) — the timer/interrupt
hang and the `jalr`/`j` misalignment bug are both fixed, and `ash`
starts. Debugging the resulting "silent hang" (instrumented via a
throwaway `src/bin/debug_boot.rs`, see method below) traced it to:

- `distinct_u_pcs`/`ecalls`/`faults` counters frozen solid between the
  200M and 400M step checkpoints — zero forward progress, not just
  slow.
- A raw PC trace at that point showed the CPU sitting in the kernel's
  own idle loop (`do_idle`/`cpuidle_not_available`), not user code —
  meaning the scheduler genuinely has nothing runnable.
- The last syscall `ash` made before going idle: `a7=414`
  (`ppoll_time64`, confirmed against
  `include/uapi/asm-generic/unistd.h`) — a blocking poll on stdin,
  waiting for terminal input, right after `exec /bin/sh`.

That poll can never be satisfied, for three compounding reasons below.
None of these are CPU-correctness bugs (unlike everything else fixed
today) — this is missing I/O plumbing that was never built.

## The three gaps, in the order to tackle them

### 1. Nothing calls `receive_uart_byte` — host stdin never reaches the emulator

`src/cpu/definitions/cpu/bus.rs` already has a correctly-wired
`receive_uart_byte(&mut self, byte: u8)`: it pushes the byte into the
UART's `rx_buffer` *and* sets the PLIC pending bit for the UART's IRQ
source (`UART_SOURCE_ID = 10`). Grepping the whole codebase, it has
**zero callers**. `src/bin/run_os.rs`'s main loop is just
`boot_kernel()` followed by a bare `while ... { step(&mut cpu) }` —
there is no code anywhere that reads host stdin and feeds it in. Every
`sleep 15; echo "ls" | run_os` test this session ran, the piped input
never actually reached the guest.

### 2. Even if wired up, `uart.rs`'s LSR read hides the data

`src/peripherals/uart.rs`, `UartState::read()`:

```rust
pub fn read(&mut self, offset: u32, _num_bytes: usize) -> u32 {
    if offset == 5 {
        0x60
    } else if offset == 0 {
        self.rx_buffer.pop_front().unwrap_or(0) as u32
    } else {
        0
    }
}
```

Offset 5 is the 16550's LSR (Line Status Register). `0x60` is THRE |
TEMT (bits 5/6 — "transmitter ready") hardcoded on always. Bit 0 (DR —
"data ready to read") is never set based on whether `rx_buffer`
actually has bytes. The kernel's UART driver checks DR before ever
attempting to read the data register, so it would never notice
incoming characters even if they were already sitting in the buffer.
Needs: `if !self.rx_buffer.is_empty() { base | 0x01 } else { base }`
(exact bit layout worth double-checking against a real 16550 LSR
reference before writing this).

### 3. Reading host stdin without blocking the CPU emulation loop

Once (1) and (2) are fixed, `run_os.rs`'s main loop still needs an
actual mechanism to check for available host input on every iteration
(or close to it) without stalling instruction execution while waiting
for a keypress. Options to weigh: non-blocking/raw stdin mode polled
each iteration, or a separate reader thread feeding a channel that the
main loop drains non-blockingly. This is the one item here that's a
real design decision, not just a bugfix — worth deciding deliberately
rather than bolting on the first thing that compiles.

## Method note, for next time this kind of bug shows up

The diagnostic path that found this: build a throwaway
`src/bin/debug_boot.rs` that runs `boot_kernel()` + a manual step loop
instrumented with whatever's relevant (mode transitions, `in_trap`,
CLINT `mtimecmp` changes, `scause`/`stval`/`a7` at U→S transitions,
symbol resolution against the kernel's own `System.map`). Cheaper and
faster to iterate on than attaching GDB, and this session used it to
find three separate, unrelated bugs (the missing `STIE`/`STIP` check,
the `jalr`/`j` alignment bug, and this one) by watching what changed —
or didn't — between two step counts far apart, rather than guessing
from a single snapshot. Delete or keep this file as wanted; it's not
part of the real emulator, just scratch tooling.
