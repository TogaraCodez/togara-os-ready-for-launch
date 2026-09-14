#!/bin/bash
# Run TOGARA OS in QEMU
#
# Usage: ./run.sh [options]
#
# Options:
#   --debug   Enable debug output
#   --efi     Boot with UEFI
#   --gdb     Wait for GDB connection

set -e

cd "$(dirname "$0")"

# Default options
QEMU_OPTS="-cdrom togara-os.iso -serial stdio -m 512M"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --debug)
            QEMU_OPTS="$QEMU_OPTS -d int -no-reboot -no-shutdown"
            shift
            ;;
        --efi)
            QEMU_OPTS="$QEMU_OPTS -bios /usr/share/OVMF/OVMF_CODE.fd"
            shift
            ;;
        --gdb)
            QEMU_OPTS="$QEMU_OPTS -s -S"
            echo "Waiting for GDB connection on port 1234..."
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check if ISO exists
if [ ! -f "togara-os.iso" ]; then
    echo "Error: togara-os.iso not found. Run ./build-iso.sh first."
    exit 1
fi

echo "=== Running TOGARA OS in QEMU ==="
echo ""

# Run QEMU
qemu-system-x86_64 $QEMU_OPTS
