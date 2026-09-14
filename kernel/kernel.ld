/* TOGARA OS Kernel Linker Script */

ENTRY(kernel_main)

SECTIONS
{
    /* Start at 1MB (typical bootloader load address) */
    . = 1M;

    /* Boot section */
    .boot : ALIGN(4K)
    {
        *(.boot)
    }

    /* Text section (code) */
    .text : ALIGN(4K)
    {
        *(.text .text.*)
    }

    /* Read-only data */
    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
    }

    /* Initialized data */
    .data : ALIGN(4K)
    {
        *(.data .data.*)
    }

    /* Uninitialized data (BSS) */
    .bss : ALIGN(4K)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* Kernel stack */
    .stack : ALIGN(16)
    {
        __stack_bottom = .;
        . += 0x10000; /* 64KB stack */
        __stack_top = .;
    }

    /* Heap */
    .heap : ALIGN(4K)
    {
        __heap_start = .;
        . += 0x100000; /* 1MB heap */
        __heap_end = .;
    }

    /* Discard unwanted sections */
    /DISCARD/ :
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
    }
}
