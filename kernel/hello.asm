; Hello World for TOGARA OS
; x86_64 assembly that writes to stdout and exits
;
; Assemble and link with:
;   nasm -f elf64 hello.asm -o hello.o
;   ld -o hello.elf hello.o

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
    message db "Hello from TOGARA OS userspace!", 10  ; 10 = newline
    length equ $ - message
