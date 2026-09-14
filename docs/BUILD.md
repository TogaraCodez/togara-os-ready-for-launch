# TOGARA OS Kernel Build Instructions

## Prerequisites

- Rust nightly toolchain
- `rust-src` component
- `llvm-tools-preview` component
- QEMU for testing (optional)

## Setup

```bash
# Install Rust nightly
rustup install nightly
rustup default nightly

# Install required components
rustup component add rust-src
rustup component add llvm-tools-preview

# Install QEMU (optional, for testing)
# Ubuntu/Debian:
sudo apt install qemu-system-x86

# macOS:
brew install qemu
```

## Build

```bash
cd kernel

# Build the kernel
cargo build --release

# The kernel binary will be at:
# target/x86_64-unknown-none/release/togara_kernel
```

## Test (QEMU)

```bash
# Create a bootable disk image (requires additional bootloader setup)
# For now, you can test with a minimal bootloader

# Run with QEMU (BIOS)
qemu-system-x86_64 \
  -drive format=raw,file=target/x86_64-unknown-none/release/togara_kernel \
  -serial stdio \
  -m 512M

# Run with QEMU (UEFI)
qemu-system-x86_64 \
  -bios /path/to/OVMF_CODE.fd \
  -drive format=raw,file=target/x86_64-unknown-none/release/togara_kernel \
  -serial stdio \
  -m 512M
```

## Expected Output

```
TOGARA OS TRINITY v0.1.0

Validating boot context... OK
Initializing PMM... OK
Initializing VMM... OK
Initializing heap... OK
Initializing GDT... OK
Initializing TSS... OK
Initializing IDT... OK
Initializing timer... OK
Initializing scheduler... OK

BOOT OK
MEMORY MAP OK
FRAMEBUFFER OK

Kernel initialized successfully!
```

## Architecture

The kernel implements:

- **Part 0:** Repository baseline
- **Part 1:** Architecture directory structure
- **Part 2:** Boot context validation
- **Part 3:** Physical Memory Manager (bitmap allocator)
- **Part 4:** Virtual Memory Manager (page tables)
- **Part 5:** Kernel heap allocator
- **Part 6:** GDT, TSS, IDT
- **Part 7:** Interrupt handlers
- **Part 8:** PIT timer (100Hz)
- **Part 9-10:** Preemptive scheduler

## Next Steps

1. Implement actual bootloader integration
2. Add proper panic output
3. Implement context switching assembly
4. Add userspace support
5. Implement syscalls

See `TOGARA_OS_IMPLEMENTATION_CHECKLISTS.md` for the complete roadmap.
