#!/bin/bash
set -e

echo "Building hello.elf..."

# Assemble with x86_64-elf assembler
x86_64-elf-as hello.asm -o hello.o

# Link with x86_64-elf linker (no C runtime)
x86_64-elf-ld hello.o -o hello.elf

echo "hello.elf built successfully"
