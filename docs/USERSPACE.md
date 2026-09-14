# TOGARA OS Userspace Implementation

## Overview

This document describes the implementation of userspace support in TOGARA OS, including:
- Context switching assembly
- Ring 3 transition
- System call interface
- ELF binary loading
- Hello World program

## Architecture

### Context Switching (Part 10)

The context switch assembly (`arch/x86_64/context_switch.asm`) saves and restores:
- Callee-saved registers: rbx, rbp, r12-r15
- Stack pointer (rsp)
- Flags (rflags)
- Instruction pointer (via return address)

```asm
switch_context(current_context, next_context):
    ; Save current task registers
    mov [rdi + offset], r15
    ...
    
    ; Load next task registers
    mov r15, [rsi + offset]
    ...
    
    ; Switch stack and return
    mov rsp, [rsi + offset]
    ret
```

### Ring 3 Transition

The transition to userspace (Ring 3) requires:
1. Setting up proper segment selectors (user code/data)
2. Setting up a userspace stack
3. Using IRETQ to atomically switch to Ring 3

```rust
pub unsafe fn create_userspace_task(id: u64, entry: u64, stack_top: u64) -> Task {
    let mut task = Task::new(TaskId(id), entry as usize);
    
    // Set up userspace stack
    task.context.rsp = stack_top;
    
    // Set up segment selectors for Ring 3
    task.context.cs = selectors.user_code as u64;
    task.context.ss = selectors.user_data as u64;
    
    // Set flags for userspace (interrupts enabled)
    task.context.rflags = 0x202;
    
    task
}
```

### System Calls (Part 14)

The syscall interface provides:
- `SYS_EXIT` (0) - Terminate process
- `SYS_WRITE` (1) - Write to file descriptor
- `SYS_READ` (2) - Read from file descriptor
- `SYS_YIELD` (8) - Yield CPU to scheduler

User pointer validation ensures kernel safety:
```rust
fn validate_user_pointer(ptr: u64, size: u64) -> bool {
    // Check if pointer is in userspace (lower half)
    if ptr >= 0xFFFF8000_00000000 {
        return false;
    }
    
    // Check for overflow
    let end = match ptr.checked_add(size) {
        Some(e) => e,
        None => return false,
    };
    
    // Check that end is also in userspace
    if end >= 0xFFFF8000_00000000 {
        return false;
    }
    
    true
}
```

### ELF Loading (Part 13)

The ELF loader validates and loads x86_64 executables:
- Magic number check (`\x7fELF`)
- 64-bit, little endian
- x86_64 architecture
- Executable or shared object type
- Program header validation
- Segment bounds checking

## Building Hello World

### Prerequisites

```bash
# Install NASM and binutils
sudo apt install nasm binutils  # Ubuntu/Debian
brew install nasm               # macOS
```

### Build

```bash
cd kernel

# Build hello.elf
./build_hello.sh

# This runs:
nasm -f elf64 hello.asm -o hello.o
ld -o hello.elf hello.o
```

### Hello.asm Source

```asm
section .text
global _start

_start:
    ; sys_write(1, message, length)
    mov rax, 1          ; syscall number for write
    mov rdi, 1          ; file descriptor (stdout)
    mov rsi, message    ; pointer to message
    mov rdx, length     ; length of message
    syscall
    
    ; sys_exit(0)
    mov rax, 60         ; syscall number for exit
    xor rdi, rdi        ; exit code 0
    syscall

section .data
    message db "Hello from TOGARA OS userspace!", 10
    length equ $ - message
```

## Testing

### Build Kernel

```bash
cd kernel
cargo build --release
```

### Run in QEMU

```bash
# BIOS boot
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/release/togara-kernel \
  -serial stdio \
  -m 512M

# UEFI boot (if supported)
qemu-system-x86_64 \
  -bios /path/to/OVMF_CODE.fd \
  -kernel target/x86_64-unknown-none/release/togara-kernel \
  -serial stdio \
  -m 512M
```

### Expected Output

```
TOGARA OS TRINITY v0.2.0

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

[    OK] Boot complete!
[    OK] Memory map OK
[    OK] Framebuffer OK

[ 0.11] Loading userspace program... OK
[ 0.12] Starting scheduler with hello world...

Hello from TOGARA OS userspace!
```

## Implementation Status

### ✅ Implemented

- Context switching assembly
- Scheduler integration
- Ring 3 task creation
- Syscall interface (exit, write, read, yield)
- User pointer validation
- ELF64 loader
- Hello world program

### ❌ TODO

- Actual IRETQ Ring 3 transition (currently stubbed)
- Working timer interrupt handler
- Real context switching in scheduler loop
- File descriptor implementation (stdout, stdin)
- VGA/text mode output for write syscall
- Process cleanup on exit
- Multiple process support

## Next Steps

1. **Implement IRETQ transition** - Replace stub with actual Ring 3 entry
2. **Wire up timer interrupts** - Make scheduler preemptive
3. **Implement VGA output** - Make write syscall actually print
4. **Add more syscalls** - open, close, mmap, brk
5. **Implement filesystem** - So programs can be loaded from disk

See `TOGARA_OS_IMPLEMENTATION_CHECKLISTS.md` for the complete roadmap.
