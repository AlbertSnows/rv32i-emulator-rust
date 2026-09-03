#!/bin/sh
# Runs this project's full verification suite, in order: unit tests,
# riscv-tests (both via `cargo test`), then riscv-arch-test. Fails fast
# (set -e) on the first failure, matching each tool's own exit code --
# nothing here is guessed or paraphrased. EXTENSIONS overrides which
# arch-test suites run (default: the full currently-declared set).
set -e

cd "$(dirname "$0")/.."

echo "== cargo test (unit tests + riscv-tests) =="
cargo test

echo "== cargo build --bin arch_test_runner =="
cargo build --bin arch_test_runner

echo "== riscv-arch-test =="
EXTENSIONS="${EXTENSIONS:-I,M,Zmmul,A,Zicntr,Zicsr,Zifencei,Zaamo,Zalrsc}" ./scripts/build_arch_test.sh
