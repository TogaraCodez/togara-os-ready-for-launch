# TOGARA OS — Actual Bootable Kernel MVP

This repository is the **real low-level TOGARA OS prototype**, not the Next.js website. It contains a `#![no_std]` Rust x86_64 kernel, a bootloader integration, framebuffer graphics, memory-map inspection, and BIOS/UEFI disk-image generation.

## What it is

The kernel boots through the Rust `bootloader` crate and receives `BootInfo` from the bootloader. It then draws the TOGARA neon command-center screen directly into the machine framebuffer.

Current kernel MVP:

- x86_64 `no_std` kernel
- BIOS boot image generation
- UEFI boot image generation
- Framebuffer graphics
- Neon TOGARA Core visualization
- Realm nodes + event-fabric visual connections
- Real boot-time framebuffer availability detection
- Real memory-map region detection
- Custom 5x7 kernel font
- Panic handler
- QEMU runner commands

## What it is NOT yet

This is a genuine bootable kernel MVP, but it is **not yet a complete general-purpose desktop OS**. It does not yet have a filesystem, userspace, scheduler, virtual memory manager, keyboard/mouse drivers, network stack, process isolation, permissions database, or AI runtime.

Those are the next kernel milestones.

## Requirements

The current `bootloader` 0.11.17 requires nightly Rust and the `llvm-tools-preview` component. The project pins `nightly-2026-07-06` because the bootloader project documents nightly compatibility around that release. See the official bootloader documentation for the current requirements.

Install:

```bash
rustup toolchain install nightly-2026-07-06
rustup component add llvm-tools-preview --toolchain nightly-2026-07-06
rustup target add x86_64-unknown-none --toolchain nightly-2026-07-06
```

Install QEMU on macOS with Homebrew:

```bash
brew install qemu
```

## Build

From the project root:

```bash
cargo build
```

The bootloader build script creates BIOS and UEFI disk images under Cargo's build output directory.

## Run in QEMU

BIOS:

```bash
cargo run -- bios
```

UEFI:

```bash
cargo run -- uefi
```

## Roadmap

### Kernel 0.2

- IDT / exception handling
- PIT/APIC timer
- cooperative scheduler
- heap allocator
- keyboard driver
- mouse driver
- serial diagnostics

### Kernel 0.3

- virtual memory manager
- process abstraction
- IPC/event bus
- VFS/filesystem
- persistent configuration

### TOGARA Runtime

- Identity subsystem
- Realm engine
- permissions
- application model
- agent runtime
- requirement lifecycle engine
- telemetry/event fabric

### TOGARA Shell

- GPU accelerated compositor
- glass/neon windows
- 3D realm navigation
- command center
- system settings
- application launcher

## Important

This project intentionally separates the **actual kernel** from the earlier Next.js visual prototype. The Next.js project can later become the TOGARA graphical shell running above the native OS/runtime rather than pretending to be the kernel.
# togara-os.rebound
# togaraOSbound2
# togaraOSbound2
# togaraOSbound2
