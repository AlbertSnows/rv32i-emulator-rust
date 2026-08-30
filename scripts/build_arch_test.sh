#!/bin/sh
# Builds the self-checking arch-test ELFs for this emulator's DUT config.
# EXTENSIONS restricts which suites get built (default: I). Pass a
# different value to override, e.g.: EXTENSIONS=I,M ./scripts/build_arch_test.sh
set -e

cd "$(dirname "$0")/.."

cp tests/arch_test_config/* \
   ~/opt/riscv-arch-test/config/cores/rv32i-emulator/rv32i-emulator/

export PATH="$HOME/opt/sail-riscv-Linux-x86_64/bin:$HOME/opt/xpack-riscv-none-elf-gcc-15.2.0-1/bin:$PATH"

cd ~/opt/riscv-arch-test
EXTENSIONS="${EXTENSIONS:-I}" make rv32i-emulator
