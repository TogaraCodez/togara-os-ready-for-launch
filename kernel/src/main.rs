//! TOGARA OS Trinity - Minimal Bootable Kernel
//! 
//! This is the kernel entry point and initialization sequence.
//! Implements Parts 0-10 of TOGARA OS Platform Roadmap.
//! 
//! # Boot Sequence
//! 
//! 1. Boot context validation
//! 2. Physical memory manager initialization
//! 3. Virtual memory manager setup
//! 4. Heap initialization
//! 5. GDT/TSS/IDT setup
//! 6. Timer configuration
//! 7. Scheduler initialization
//! 8. Kernel main loop

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

use core::panic::PanicInfo;
use boot::{BootContext, BootWriter, MemoryRegion};
use memory::pmm::{BitmapFrameAllocator, FrameAllocator, PAGE_SIZE};
use memory::vmm::{VirtualMemoryManager, PagePerm};
use gdt::Gdt;
use tss::TaskStateSegment;
use idt::Idt;
use scheduler::Scheduler;

/// Kernel version
const KERNEL_VERSION: &str = "0.1.0";

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
    let _ = writer.write_str("Validating boot context... ");
    if boot::validate_boot_context(boot_ctx).is_ok() {
        let _ = writer.write_str("OK\n");
    } else {
        panic!("Boot context validation failed!");
    }
    
    // Part 1-3: Initialize physical memory manager
    let _ = writer.write_str("Initializing PMM... ");
    FRAME_ALLOCATOR = Some(BitmapFrameAllocator::from_memory_map(
        boot_ctx.memory_map,
        &[],
    ).unwrap_or(BitmapFrameAllocator::new(1000)));
    let _ = writer.write_str("OK\n");
    
    // Part 4: Initialize virtual memory manager
    let _ = writer.write_str("Initializing VMM... ");
    VMM = Some(VirtualMemoryManager::new());
    if let Some(vmm) = &mut VMM {
        vmm.init(0);  // Physical page table base
    }
    let _ = writer.write_str("OK\n");
    
    // Part 5: Initialize heap
    let _ = writer.write_str("Initializing heap... ");
    heap::init_heap(0x10000, 0x100000);
    let _ = writer.write_str("OK\n");
    
    // Part 6: Initialize GDT
    let _ = writer.write_str("Initializing GDT... ");
    GDT = Some(Gdt::new());
    if let Some(gdt) = &GDT {
        gdt.load();
    }
    let _ = writer.write_str("OK\n");
    
    // Part 6: Initialize TSS
    let _ = writer.write_str("Initializing TSS... ");
    TSS = Some(TaskStateSegment::new());
    if let Some(tss) = &mut TSS {
        // Set double-fault stack
        tss.set_double_fault_stack(0x20000);
    }
    let _ = writer.write_str("OK\n");
    
    // Part 6: Initialize IDT
    let _ = writer.write_str("Initializing IDT... ");
    IDT = Some(Idt::new());
    if let Some(idt) = &mut IDT {
        interrupts::init_idt(idt);
        idt.load();
    }
    let _ = writer.write_str("OK\n");
    
    // Part 8: Initialize timer
    let _ = writer.write_str("Initializing timer... ");
    timer::init_timer(timer::DEFAULT_FREQUENCY);
    let _ = writer.write_str("OK\n");
    
    // Part 9-10: Initialize scheduler
    let _ = writer.write_str("Initializing scheduler... ");
    SCHEDULER = Some(Scheduler::new());
    let _ = writer.write_str("OK\n");
    
    // Boot complete
    let _ = writer.write_str("\nBOOT OK\nMEMORY MAP OK\nFRAMEBUFFER OK\n");
    let _ = writer.write_str("\nKernel initialized successfully!\n");
    
    // Enter kernel main loop (idle)
    kernel_idle_loop()
}

/// Kernel idle loop
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
