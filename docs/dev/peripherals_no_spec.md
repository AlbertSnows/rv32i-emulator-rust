# Peripherals with no official spec: CLINT, UART

Unlike PLIC/SBI/virtio-mmio/Device Tree (`docs/dev/peripherals_specs.md`),
these two have no RISC-V International (or other standards body)
document at all — they're de facto conventions everyone (QEMU, Linux,
SiFive hardware) just agreed to reuse. There's no PDF to cite section
numbers from; the actual reference is source code. All addresses below
are QEMU's `virt` machine layout (`include/hw/riscv/virt.h`), the
target this emulator already implicitly follows.

## CLINT — Core-Local Interruptor

**What it is.** SiFive's own memory-mapped timer + software-interrupt
block. Two things live here: a free-running 64-bit `mtime` counter and
a per-hart `mtimecmp` compare register (a timer interrupt becomes
pending once `mtime >= mtimecmp`), plus a per-hart `msip` register for
inter-hart software interrupts.

**Why there's no spec.** It predates and was never folded into a
RISC-V-ratified document — the privileged spec only defines the CSRs
that *observe* time/instret (`time`/`timeh`, itself an item this
project just implemented), not the memory-mapped device that drives
them. RISC-V International's later "ACLINT" work formalizes a similar
device, but plain "CLINT" as QEMU/SiFive implement it stays a
convention, not a spec.

**Reference:** QEMU's [`hw/intc/sifive_clint.c`](https://github.com/qemu/qemu/blob/master/hw/intc/sifive_clint.c)
is the actual behavioral reference — what a "CLINT" needs to do is
defined by what this file does, not by a document.

**Status here: already done.** `src/cpu/definitions/addresses.rs`
already declares `MTIME = 0x0200BFF8` and `MTIMECMP = 0x02004000`,
which land exactly inside QEMU's CLINT region (`base 0x02000000`, size
`0x10000` — `mtimecmp` at `+0x4000`, `mtime` at `+0xBFF8`, per
`include/hw/riscv/virt.h`'s `VIRT_CLINT` entry). No new work needed
for `mtime`/`mtimecmp`; `msip` (software interrupts, offset `+0x0000`
for hart 0) isn't yet wired to `mip.MSIP`, worth doing once
inter-hart/IPI support (SBI's IPI extension) is actually needed.

## UART (ns16550a-compatible)

**What it is.** A byte-at-a-time serial port — the classic National
Semiconductor 16550, unchanged in its register layout since the 1980s.
Nothing about it is RISC-V-specific; RISC-V platforms just reuse it
because Linux (and every other OS) already has a rock-solid driver for
it.

**Why there's no spec.** It's an industry-standard hardware part, not
an ISA extension or platform convention RISC-V International would
ever need to define. The "spec" is the original PC16550D datasheet —
overkill for an emulator, since only a handful of its registers matter
to software.

**References:**
- QEMU's [`hw/char/serial.c`](https://github.com/qemu/qemu/blob/master/hw/char/serial.c)
  — what this emulator's own model needs to *behave like*.
- Linux's [`include/uapi/linux/serial_reg.h`](https://github.com/torvalds/linux/blob/master/include/uapi/linux/serial_reg.h)
  — the actual register offsets/bits software reads and writes; this
  is effectively the real "schema" for a from-scratch UART model, more
  useful here than the historical datasheet.

### Register layout (offsets from base, 1 byte apart — QEMU's `virt` uses byte spacing, not the 4-byte-strided variant some SoCs use)

| Offset | DLAB | Name | Dir | Purpose |
|---|---|---|---|---|
| 0 | 0 | RBR / THR | R / W | Receive buffer / Transmit holding |
| 0 | 1 | DLL | R/W | Divisor latch, low byte |
| 1 | 0 | IER | R/W | Interrupt enable |
| 1 | 1 | DLM | R/W | Divisor latch, high byte |
| 2 | - | IIR / FCR | R / W | Interrupt ID / FIFO control |
| 3 | - | LCR | R/W | Line control (bit 7 = DLAB) |
| 4 | - | MCR | R/W | Modem control |
| 5 | - | LSR | R | Line status |
| 6 | - | MSR | R | Modem status |
| 7 | - | SCR | R/W | Scratch (no function; software uses it to probe "is a UART even here") |

**LSR bits that actually matter** (`UART_LSR_*` in `serial_reg.h`):
`0x01` DR (receive data ready), `0x20` THRE (transmit holding register
empty — software polls this before writing a byte), `0x40` TEMT
(transmitter fully empty, FIFO and shift register both).

### What's needed (minimal, output-only, matches the staged plan already agreed on)

A "print boot messages, no interactivity yet" implementation needs
only three things at MMIO offset `0` and `5` from base `0x10000000`
(QEMU `virt`'s `VIRT_UART0`, size `0x100`):

1. Writes to `THR` (offset 0) — capture the byte, and, minimally, print
   it immediately (a real 16-byte FIFO with actual transmit latency is
   unnecessary for correctness — nothing in the boot path depends on
   *when* a byte drains, only that it eventually does).
2. `LSR` (offset 5) always reads with `THRE` (`0x20`) and `TEMT`
   (`0x40`) set — "always ready to accept another byte" is a
   completely legal (if maximally fast) implementation, and it's what
   most simple emulators do since there's no reason to model transmit
   delay.
3. Everything else (IER, FCR, LCR, MCR, DLL/DLM, SCR) can be plain
   read/write scratch storage with no side effects — Linux's 8250
   driver writes to several of these during initialization (setting
   baud rate via DLL/DLM, enabling FIFOs via FCR) and will work fine as
   long as those writes don't error, even if the emulator ignores their
   actual effect.

Interactive input (`RBR`/`IER`'s receive-data-interrupt bit, wired
through PLIC IRQ `10` per QEMU's `UART0_IRQ`) is stage 2, tied to PLIC
existing at all — no point implementing RX before there's an interrupt
controller to deliver it through.
