#!/bin/bash
# Build bootable ISO for TOGARA OS
#
# Usage: ./build-iso.sh
#
# Prerequisites:
#   - Rust nightly with x86_64-unknown-none target
#   - NASM
#   - xorriso
#   - Limine bootloader

set -e

cd "$(dirname "$0")"

echo "=== TOGARA OS Build ==="
echo ""

# Build hello world
echo "Building hello.elf..."
./build_hello.sh

# Build kernel
echo ""
echo "Building kernel..."
cargo build --release --target x86_64-unknown-none.json

# Create ISO directory
echo ""
echo "Creating ISO..."
ISO_DIR="isodir"
rm -rf "$ISO_DIR"
mkdir -p "$ISO_DIR/boot/limine"
mkdir -p "$ISO_DIR/boot/togara"

# Copy files
cp target/x86_64-unknown-none/release/togara-kernel "$ISO_DIR/boot/togara/togara-kernel.elf"
cp hello.elf "$ISO_DIR/boot/togara/"
cp limine.cfg "$ISO_DIR/boot/togara/"

# Copy Limine (adjust path as needed)
if [ -d "../limine" ]; then
    cp ../limine/limine-bios.sys "$ISO_DIR/boot/limine/"
    cp ../limine/BOOTX64.EFI "$ISO_DIR/boot/limine/"
else
    echo "Warning: Limine bootloader not found. Skipping..."
fi

# Create ISO
echo "Creating togara-os.iso..."
xorriso -as mkisofs \
    -b boot/limine/limine-bios.sys \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot boot/limine/BOOTX64.EFI \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    "$ISO_DIR/" -o togara-os.iso

echo ""
echo "=== Build Complete ==="
echo "Created: togara-os.iso"
echo ""
echo "To run in QEMU:"
echo "  ./run.sh"
echo ""
echo "Or manually:"
echo "  qemu-system-x86_64 -cdrom togara-os.iso -serial stdio -m 512M"
