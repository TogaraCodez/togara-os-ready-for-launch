#!/bin/bash
# Build script for hello world userspace program
#
# Usage: ./build_hello.sh
#
# Prerequisites:
#   - nasm (Netwide Assembler)
#   - ld (GNU linker)

set -e

cd "$(dirname "$0")"

echo "Building hello.elf..."

# Assemble
nasm -f elf64 hello.asm -o hello.o

# Link
ld -o hello.elf hello.o

# Show info
echo "Built hello.elf:"
file hello.elf
ls -lh hello.elf

echo ""
echo "To test in QEMU:"
echo "  cargo build --release"
echo "  qemu-system-x86_64 -kernel target/x86_64-unknown-none/release/togara-kernel -serial stdio"
