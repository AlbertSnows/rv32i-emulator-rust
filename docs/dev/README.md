# Dev Notes

## Prerequisites

- Rust toolchain (rustup/cargo) — already installed, see `cargo --version`


## Dev loop

```
bacon run
```

^ -> Allows you to essentially hot reload changed files

## Build & Run

```
cargo build
cargo run
```

## Test

```
cargo test
```

Runs unit tests and riscv-tests (both wired into the same `cargo test`
run). Doesn't touch riscv-arch-test — that's a separate, external
framework (`~/opt/riscv-arch-test`, see
`docs/dev/riscv_arch_test_setup.md`) not invoked by `cargo test` at
all.

### Full verification (all three test layers)

```
scripts/verify.sh
```

Runs, in order, exiting on the first failure: `cargo test` (unit tests
+ riscv-tests), then builds `arch_test_runner`, then runs
riscv-arch-test via `scripts/build_arch_test.sh`. Exit code reflects
whichever step failed — nothing is summarized or interpreted, the
script just runs the real commands and lets each one's own exit code
propagate.

`EXTENSIONS=<suites> scripts/verify.sh` restricts which arch-test
suites run (default: the full currently-declared set, `I,M,Zmmul,A,
Zicntr,Zicsr,Zifencei,Zaamo,Zalrsc`) — same override
`scripts/build_arch_test.sh` itself takes.

This exists so a "N/N passing" claim (in a doc, in conversation,
anywhere) can be independently re-run rather than taken on faith —
run it yourself rather than trusting a reported number.

## Project Structure

todo

## Design Decisions

todo


