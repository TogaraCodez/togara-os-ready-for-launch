; IRETQ Ring 3 Transition
;
; Priority 2: Userspace Isolation
; Classification: RUNTIME
;
; Implements the transition from kernel mode (Ring 0) to user mode (Ring 3)
; using the IRETQ instruction.

global iretq_entry

; iretq_entry(
;     u64 entry_point,     ; [rdi] - User code entry point
;     u64 user_stack,      ; [rsi] - User stack pointer
;     u64 code_selector,   ; [rdx] - User code segment selector
;     u64 stack_selector,  ; [rcx] - User stack segment selector
;     u64 flags            ; [r8]  - CPU flags (e.g., 0x202 for interrupts)
; )
;
; This function performs an IRETQ to transition from Ring 0 to Ring 3.
; It sets up the stack frame required by IRETQ and jumps to userspace.

iretq_entry:
    ; Save all registers we'll use
    push rdi
    push rsi
    push rdx
    push rcx
    push r8
    
    ; Disable interrupts during transition
    cli
    
    ; Set up IRETQ stack frame (from top to bottom):
    ; [rsp + 40] SS (stack selector)
    ; [rsp + 32] RSP (user stack pointer)
    ; [rsp + 24] RFLAGS
    ; [rsp + 16] CS (code selector)
    ; [rsp +  8] RIP (entry point)
    
    ; Move user stack pointer
    mov r12, rsi          ; Save user stack
    
    ; Push SS (stack segment selector)
    push rcx              ; SS
    
    ; Push RSP (user stack pointer)
    push r12              ; RSP
    
    ; Push RFLAGS
    push r8               ; RFLAGS
    
    ; Push CS (code segment selector)
    push rdx              ; CS
    
    ; Push RIP (entry point)
    push rdi              ; RIP
    
    ; Now stack looks like:
    ; [rsp]     = RIP
    ; [rsp+8]   = CS
    ; [rsp+16]  = RFLAGS
    ; [rsp+24]  = RSP
    ; [rsp+32]  = SS
    
    ; Restore saved registers (except we'll use them for IRETQ)
    pop rdi               ; Restore but don't use (RIP is on stack)
    
    ; Perform IRETQ - this switches to Ring 3
    iretq
    
    ; Should never reach here
    hlt
