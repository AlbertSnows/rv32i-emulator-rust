#!/bin/sh
# Builds riscv-tests fixtures for one ISA extension into tests/fixtures/,
# reading the instruction list straight from that extension's own
# Makefrag instead of it being copied by hand.
# Usage: ./scripts/build_fixtures.sh <extension>   (e.g. rv32um, rv32uc)
set -e

EXT="$1"
if [ -z "$EXT" ]; then
    echo "usage: $0 <extension>  (e.g. rv32um, rv32uc)" >&2
    exit 1
fi

RISCV_TESTS="${RISCV_TESTS:-$HOME/Documents/programming/riscv/riscv-tests}"
GCC="${GCC:-$(ls -d "$HOME"/opt/xpack-riscv-none-elf-gcc-*/bin/riscv-none-elf-gcc | head -1)}"
DEST="$(cd "$(dirname "$0")/.." && pwd)/tests/fixtures"

MAKEFRAG="$RISCV_TESTS/isa/$EXT/Makefrag"
if [ ! -f "$MAKEFRAG" ]; then
    echo "no Makefrag at $MAKEFRAG" >&2
    exit 1
fi

TESTS=$(awk -v ext="$EXT" '
  $0 ~ "^"ext"_sc_tests" { grabbing=1; next }
  grabbing && NF == 0 { grabbing=0 }
  grabbing { gsub(/\\/, ""); print }
' "$MAKEFRAG")

cd "$RISCV_TESTS"
for t in $TESTS; do
    echo "building $EXT/$t -> $t-p-$t"
    "$GCC" -march=rv32g -mabi=ilp32 -static -mcmodel=medany -fvisibility=hidden \
        -nostdlib -nostartfiles \
        -Ienv/p -Iisa/macros/scalar -Tenv/p/link.ld \
        "isa/$EXT/$t.S" -o "$DEST/$t-p-$t"
done
