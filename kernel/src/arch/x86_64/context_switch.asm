; Context Switching Assembly for x86_64
;
; Part 10 of TOGARA OS Platform Roadmap
; Classification: RUNTIME
;
; Implements low-level context switching between tasks.
; Saves callee-saved registers and switches stacks.

global switch_context

; switch_context(
;     CpuContext* current_context,  ; [rdi]
;     CpuContext* next_context      ; [rsi]
; )
;
; This function saves the current CPU state into current_context,
; then restores the state from next_context and jumps to it.
switch_context:
    ; Save callee-saved registers from current task
    mov [rdi + 0x00], r15    ; r15
    mov [rdi + 0x08], r14    ; r14
    mov [rdi + 0x10], r13    ; r13
    mov [rdi + 0x18], r12    ; r12
    mov [rdi + 0x20], r11    ; r11
    mov [rdi + 0x28], r10    ; r10
    mov [rdi + 0x30], r9     ; r9
    mov [rdi + 0x38], r8     ; r8
    mov [rdi + 0x40], rdi    ; rdi (save after use)
    mov [rdi + 0x48], rsi    ; rsi (save after use)
    mov [rdi + 0x50], rbp    ; rbp
    mov [rdi + 0x58], rbx    ; rbx
    mov [rdi + 0x60], rdx    ; rdx
    mov [rdi + 0x68], rcx    ; rcx
    mov [rdi + 0x70], rax    ; rax
    
    ; Save current stack pointer (after pushing return address)
    lea rax, [rsp + 0x08]    ; Point to saved RSP (skip return address)
    mov [rdi + 0x98], rax    ; rsp
    
    ; Save flags and code segment
    pushfq
    pop rax
    mov [rdi + 0x88], rax    ; rflags
    
    ; Load next task's registers
    mov r15, [rsi + 0x00]    ; r15
    mov r14, [rsi + 0x08]    ; r14
    mov r13, [rsi + 0x10]    ; r13
    mov r12, [rsi + 0x18]    ; r12
    mov r11, [rsi + 0x20]    ; r11
    mov r10, [rsi + 0x28]    ; r10
    mov r9,  [rsi + 0x30]    ; r9
    mov r8,  [rsi + 0x38]    ; r8
    ; rdi, rsi loaded last
    mov rbp, [rsi + 0x50]    ; rbp
    mov rbx, [rsi + 0x58]    ; rbx
    mov rdx, [rsi + 0x60]    ; rdx
    mov rcx, [rsi + 0x68]    ; rcx
    mov rax, [rsi + 0x70]    ; rax
    
    ; Load next task's stack pointer
    mov rsp, [rsi + 0x98]    ; rsp
    
    ; Load flags
    mov rax, [rsi + 0x88]
    push rax
    popfq
    
    ; Load rdi and rsi last (before returning)
    mov rdi, [rsi + 0x40]    ; rdi
    mov rsi, [rsi + 0x48]    ; rsi
    
    ; Return to the next task's saved instruction pointer
    ret
