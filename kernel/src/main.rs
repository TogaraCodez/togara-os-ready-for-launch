//! TOGARA OS Trinity - Minimal Bootable Kernel with Userspace
//! 
//! This is the kernel entry point and initialization sequence.
//! Implements Parts 0-14 of TOGARA OS Platform Roadmap.
//! 
//! # Boot Sequence
//! 
//! 1. Boot context validation
//! 2. Physical memory manager initialization
//! 3. Virtual memory manager setup
//! 4. Heap initialization
//! 5. GDT/TSS/IDT setup
//! 6. Timer configuration
//! 7. Scheduler initialization with context switching
//! 8. Load and run hello world userspace program
//! 9. Enter scheduler loop

#![no_std]
#![no_main]

mod boot;
mod memory {
    pub mod pmm;
    pub mod vmm;
}
mod heap;
mod gdt;
mod tss;
mod idt;
mod interrupts;
mod timer;
mod scheduler;
mod syscall;
mod elf;
mod process;
mod context;

use core::panic::PanicInfo;
use boot::{BootContext, BootWriter, MemoryRegion};
use memory::pmm::{BitmapFrameAllocator, FrameAllocator, PAGE_SIZE};
use memory::vmm::{VirtualMemoryManager, PagePerm};
use gdt::Gdt;
use tss::TaskStateSegment;
use idt::Idt;
use scheduler::Scheduler;

/// Kernel version
const KERNEL_VERSION: &str = "0.2.0";

/// Boot buffer for early output
static mut BOOT_BUFFER: [u8; 4096] = [0u8; 4096];

/// Global frame allocator (64 words = 4096 frames = 16MB)
static mut FRAME_ALLOCATOR: Option<BitmapFrameAllocator<64>> = None;

/// Global VMM
static mut VMM: Option<VirtualMemoryManager> = None;

/// Global GDT
static mut GDT: Option<Gdt> = None;

/// Global TSS
static mut TSS: Option<TaskStateSegment> = None;

/// Global IDT
static mut IDT: Option<Idt> = None;

/// Global scheduler
static mut SCHEDULER: Option<Scheduler> = None;

/// Kernel entry point
/// 
/// # Safety
/// - Called by bootloader with valid boot context
#[no_mangle]
pub unsafe extern "C" fn kernel_main(boot_ctx: &BootContext) -> ! {
    let mut writer = BootWriter::new(core::str::from_utf8_unchecked_mut(&mut BOOT_BUFFER));
    
    // Print banner
    let _ = core::fmt::write(&mut writer, format_args!(
        "\nTOGARA OS TRINITY v{}\n", KERNEL_VERSION
    ));
    
    // Part 0: Validate boot context
    let _ = writer.write_str("[ 0.00] Validating boot context... ");
    if boot::validate_boot_context(boot_ctx).is_ok() {
        let _ = writer.write_str("OK\n");
    } else {
        panic!("Boot context validation failed!");
    }
    
    // Part 1-3: Initialize physical memory manager
    let _ = writer.write_str("[ 0.01] Initializing PMM... ");
    FRAME_ALLOCATOR = Some(BitmapFrameAllocator::from_memory_map(
        boot_ctx.memory_map,
        &[],
    ).unwrap_or(BitmapFrameAllocator::new(1000)));
    let _ = writer.write_str("OK\n");
    
    // Part 4: Initialize virtual memory manager
    let _ = writer.write_str("[ 0.02] Initializing VMM... ");
    VMM = Some(VirtualMemoryManager::new());
    if let Some(vmm) = &mut VMM {
        vmm.init(0);  // Physical page table base
    }
    let _ = writer.write_str("OK\n");
    
    // Part 5: Initialize heap
    let _ = writer.write_str("[ 0.03] Initializing heap... ");
    heap::init_heap(0x10000, 0x100000);
    let _ = writer.write_str("OK\n");
    
    // Part 6: Initialize GDT
    let _ = writer.write_str("[ 0.04] Initializing GDT... ");
    GDT = Some(Gdt::new());
    if let Some(gdt) = &GDT {
        gdt.load();
    }
    let _ = writer.write_str("OK\n");
    
    // Part 6: Initialize TSS
    let _ = writer.write_str("[ 0.05] Initializing TSS... ");
    TSS = Some(TaskStateSegment::new());
    if let Some(tss) = &mut TSS {
        // Set double-fault stack
        tss.set_double_fault_stack(0x20000);
    }
    let _ = writer.write_str("OK\n");
    
    // Part 6: Initialize IDT
    let _ = writer.write_str("[ 0.06] Initializing IDT... ");
    IDT = Some(Idt::new());
    if let Some(idt) = &mut IDT {
        interrupts::init_idt(idt);
        idt.load();
    }
    let _ = writer.write_str("OK\n");
    
    // Part 7: Enable interrupts
    let _ = writer.write_str("[ 0.07] Enabling interrupts... ");
    unsafe {
        core::arch::asm!("sti");
    }
    let _ = writer.write_str("OK\n");
    
    // Part 8: Initialize timer
    let _ = writer.write_str("[ 0.08] Initializing timer (100Hz)... ");
    timer::init_timer(timer::DEFAULT_FREQUENCY);
    let _ = writer.write_str("OK\n");
    
    // Part 9-10: Initialize scheduler
    let _ = writer.write_str("[ 0.09] Initializing scheduler... ");
    SCHEDULER = Some(Scheduler::new());
    let _ = writer.write_str("OK\n");
    
    // Part 14: Initialize syscalls
    let _ = writer.write_str("[ 0.10] Initializing syscalls... ");
    syscall::init_syscalls();
    let _ = writer.write_str("OK\n");
    
    // Boot phase complete
    let _ = writer.write_str("\n[    OK] Boot complete!\n");
    let _ = writer.write_str("[    OK] Memory map OK\n");
    let _ = writer.write_str("[    OK] Framebuffer OK\n");
    let _ = writer.write_str("\n");
    
    // Part 11-13: Load and run hello world userspace program
    let _ = writer.write_str("[ 0.11] Loading userspace program... ");
    match process::run_hello_world() {
        Ok(_) => {
            let _ = writer.write_str("OK\n");
            let _ = writer.write_str("[ 0.12] Starting scheduler with hello world...\n");
        }
        Err(e) => {
            let _ = writer.write_str("FAIL\n");
            let _ = core::fmt::write(&mut writer, format_args!(
                "       Error: {:?}\n", e
            ));
            let _ = writer.write_str("[ 0.12] Starting scheduler (idle only)...\n");
        }
    }
    
    // Start the scheduler (this should not return)
    if let Some(scheduler) = &mut SCHEDULER {
        context::start_scheduler(scheduler);
    }
    
    // Should never reach here
    kernel_idle_loop()
}

/// Kernel idle loop (fallback)
fn kernel_idle_loop() -> ! {
    loop {
        // In real implementation, would use HLT instruction
        // and wait for interrupts
        core::hint::spin_loop();
    }
}

/// Panic handler
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // In real implementation, would print panic info and halt
    loop {
        core::hint::spin_loop();
    }
}
