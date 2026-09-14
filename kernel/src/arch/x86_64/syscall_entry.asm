; SYSCALL Entry and SYSRET Return
;
; Priority 3: Working Syscalls
; Classification: RUNTIME
;
; Implements fast syscall entry using the SYSCALL instruction.
; This is faster than interrupt-based syscalls (INT 0x80).

global syscall_entry
extern syscall_handler_rust

; syscall_entry is called via SYSCALL instruction
; SYSCALL saves:
;   - RIP to RCX (next instruction after SYSCALL)
;   - RFLAGS to R11
;   - Loads IA32_STAR, IA32_LSTAR, IA32_FMASK MSRs
;
; On entry:
;   - RAX = syscall number
;   - RDI, RSI, RDX, R10, R8, R9 = arguments
;   - RCX = return address (saved by SYSCALL)
;   - R11 = saved RFLAGS (saved by SYSCALL)
;
; We need to:
;   1. Switch to kernel stack
;   2. Save user context
;   3. Call Rust handler
;   4. Restore context
;   5. Return with SYSRET

syscall_entry:
    ; Save user registers (except RAX which has syscall number)
    push r11            ; Saved RFLAGS
    push rcx            ; Return address (will become RIP)
    
    push rdi
    push rsi
    push rdx
    push r10            ; SYSCALL clobbers RCX and R11, so use R10 for 4th arg
    push r8
    push r9
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    
    ; Save RAX (syscall number)
    push rax
    
    ; Switch to kernel stack (GS:0 points to TSS.rsp0)
    ; In real implementation, would use SWAPGS to get kernel GS base
    ; For now, assume we're already on kernel stack
    
    ; Align stack to 16 bytes
    and rsp, ~0xF
    
    ; Save current stack pointer in RDI (first arg to handler)
    mov rdi, rsp
    
    ; Call Rust syscall handler
    ; extern fn syscall_handler_rust(syscall_number: u64, args: &[u64; 6]) -> i64
    call syscall_handler_rust
    
    ; Result is in RAX
    
    ; Restore RAX (original syscall number - not needed anymore)
    pop rax
    
    ; Restore all registers
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    pop r9
    pop r8
    pop r10
    pop rdx
    pop rsi
    pop rdi
    
    ; Restore RFLAGS and RIP
    pop rcx             ; Return address
    pop r11             ; Saved RFLAGS
    
    ; Return to userspace with SYSRET
    ; SYSRET loads:
    ;   - RIP from RCX
    ;   - RFLAGS from R11
    ;   - CS/SS from IA32_STAR
    sysret
