# TOGARA OS Trinity

A bootable, multitasking operating system kernel written in Rust.

## Features

✅ **Boot & Output**
- VGA text mode driver (80x25, 16 colors)
- Serial port debug output (COM1, 38400 baud)
- Limine bootloader integration
- Multiboot2 compliance

✅ **Hardware Support**
- PIT timer (IRQ0, 100Hz)
- PS/2 keyboard (IRQ1)
- PIC interrupt controller (8259)

✅ **Userspace**
- Ring 3 execution via IRETQ
- SYSCALL/SYSRET fast syscalls
- ELF64 binary loader
- Hello World userspace program

✅ **Kernel**
- Preemptive scheduler
- Context switching (assembly)
- Physical memory manager (bitmap)
- Virtual memory manager (page tables)

## Prerequisites

### Rust Toolchain

```bash
rustup install nightly
rustup default nightly
rustup target add x86_64-unknown-none
rustup component add rust-src llvm-tools-preview
```

### Build Tools

```bash
# Ubuntu/Debian
sudo apt install nasm binutils xorriso qemu-system-x86

# macOS
brew install nasm binutils xorriso qemu

# Arch Linux
sudo pacman -S nasm binutils libisoburn qemu
```

### Limine Bootloader

```bash
git clone https://github.com/limine-bootloader/limine.git
cd limine
git checkout v6.x
make
```

## Building

### 1. Build Hello World

```bash
cd kernel
./build_hello.sh
```

### 2. Build Bootable ISO

```bash
./build-iso.sh
```

This creates `togara-os.iso` in the kernel directory.

### 3. Run in QEMU

```bash
./run.sh
```

### Expected Output

```
TOGARA OS TRINITY v0.3.0

[ 0.00] Validating boot context... OK
[ 0.01] Initializing PMM... OK
[ 0.02] Initializing VMM... OK
[ 0.03] Initializing heap... OK
[ 0.04] Initializing GDT... OK
[ 0.05] Initializing TSS... OK
[ 0.06] Initializing IDT... OK
[ 0.07] Enabling interrupts... OK
[ 0.08] Initializing timer (100Hz)... OK
[ 0.09] Initializing scheduler... OK
[ 0.10] Initializing syscalls... OK
[ 0.11] Initializing VGA... OK
[ 0.12] Initializing serial... OK
[ 0.13] Initializing PIC... OK
[ 0.14] Initializing keyboard... OK

[    OK] Boot complete!
[    OK] Memory map OK
[    OK] Framebuffer OK

[ 0.15] Loading userspace program... OK
[ 0.16] Starting scheduler with hello world...

Hello from TOGARA OS userspace!
```

## Debugging

### Serial Output

TOGARA OS outputs debug messages to both VGA and serial port:

```bash
# Run with serial output to file
qemu-system-x86_64 -cdrom togara-os.iso -serial file:serial.log -m 512M

# View serial output
 tail -f serial.log
```

### GDB Debugging

```bash
# Build with debug symbols
cargo build --target x86_64-unknown-none.json

# Run QEMU with GDB
qemu-system-x86_64 -cdrom togara-os.iso -s -S -m 512M

# In another terminal
gdb target/x86_64-unknown-none/debug/togara-kernel
(gdb) target remote :1234
(gdb) continue
```

## Architecture

```
TOGARA OS Kernel
│
├── Boot
│   ├── Multiboot2 header
│   ├── Limine bootloader
│   └── Boot context parsing
│
├── Hardware
│   ├── VGA text mode (0xB8000)
│   ├── Serial port (0x3F8)
│   ├── PIT timer (IRQ0)
│   ├── Keyboard (IRQ1)
│   └── PIC (8259)
│
├── Memory
│   ├── Physical (bitmap allocator)
│   ├── Virtual (4-level page tables)
│   └── Heap (global allocator)
│
├── Interrupts
│   ├── IDT (256 entries)
│   ├── PIC (IRQ 0-15)
│   └── Handlers (#PF, #GP, #DF, etc.)
│
├── Userspace
│   ├── Ring 3 (IRETQ transition)
│   ├── Syscalls (SYSCALL/SYSRET)
│   └── ELF loader
│
└── Scheduler
    ├── Context switching (assembly)
    ├── Task control blocks
    └── Preemptive (timer IRQ)
```

## Project Structure

```
kernel/
├── src/
│   ├── main.rs              # Kernel entry point
│   ├── boot.rs              # Boot context
│   ├── vga.rs               # VGA driver
│   ├── serial.rs            # Serial driver
│   ├── timer_pit.rs         # PIT timer
│   ├── keyboard.rs          # Keyboard driver
│   ├── pic.rs               # PIC controller
│   ├── gdt.rs               # GDT
│   ├── tss.rs               # TSS
│   ├── idt.rs               # IDT
│   ├── interrupts.rs        # Interrupt handlers
│   ├── memory/
│   │   ├── pmm.rs           # Physical memory
│   │   └── vmm.rs           # Virtual memory
│   ├── heap.rs              # Heap allocator
│   ├── scheduler.rs         # Scheduler
│   ├── context.rs           # Context switching
│   ├── syscall.rs           # Syscalls
│   ├── elf.rs               # ELF loader
│   ├── process.rs           # Process management
│   ├── bootloader.rs        # Multiboot2
│   └── multiboot2_header.rs # Multiboot header
├── arch/x86_64/
│   ├── context_switch.asm   # Context switch
│   ├── iretq.asm            # Ring 3 transition
│   └── syscall_entry.asm    # Syscall entry
├── hello.asm                # Hello world source
├── limine.cfg               # Bootloader config
├── Makefile                 # Build automation
├── build-iso.sh             # ISO builder
└── run.sh                   # QEMU runner
```

## Next Steps

### In Progress
- [ ] Working Ring 3 execution
- [ ] Syscall implementation with VGA output
- [ ] Page fault handling
- [ ] Process creation (fork/spawn)

### Roadmap
- [ ] Filesystem (TrinityFS)
- [ ] Block device drivers
- [ ] Network stack
- [ ] Desktop environment

See `TOGARA_OS_IMPLEMENTATION_CHECKLISTS.md` for complete roadmap.

## License

MIT License - See LICENSE file for details.
