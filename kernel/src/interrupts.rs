//! Interrupt and Exception Handling
//! 
//! Part 7 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Sets up IDT and handles exceptions.

use crate::idt::{Idt, ExceptionFrame};

/// Initialize the IDT with exception handlers
pub fn init_idt(idt: &'static mut Idt) {
    // Divide Error (#DE)
    idt.set_entry(0, divide_error_handler as usize, 0x08, 0x8E);
    
    // Debug (#DB)
    idt.set_entry(1, debug_handler as usize, 0x08, 0x8E);
    
    // Non-Maskable Interrupt
    idt.set_entry(2, nmi_handler as usize, 0x08, 0x8E);
    
    // Breakpoint (#BP)
    idt.set_entry(3, breakpoint_handler as usize, 0x08, 0x8E);
    
    // Overflow (#OF)
    idt.set_entry(4, overflow_handler as usize, 0x08, 0x8E);
    
    // Bound Range Exceeded (#BR)
    idt.set_entry(5, bound_handler as usize, 0x08, 0x8E);
    
    // Invalid Opcode (#UD)
    idt.set_entry(6, invalid_opcode_handler as usize, 0x08, 0x8E);
    
    // Device Not Available (#NM)
    idt.set_entry(7, device_not_available_handler as usize, 0x08, 0x8E);
    
    // Double Fault (#DF) - with error code
    idt.set_entry(8, double_fault_handler as usize, 0x08, 0x8E);
    
    // Invalid TSS (#TS) - with error code
    idt.set_entry(10, invalid_tss_handler as usize, 0x08, 0x8E);
    
    // Segment Not Present (#NP) - with error code
    idt.set_entry(11, segment_not_present_handler as usize, 0x08, 0x8E);
    
    // Stack-Segment Fault (#SS) - with error code
    idt.set_entry(12, stack_fault_handler as usize, 0x08, 0x8E);
    
    // General Protection Fault (#GP) - with error code
    idt.set_entry(13, general_protection_handler as usize, 0x08, 0x8E);
    
    // Page Fault (#PF) - with error code
    idt.set_entry(14, page_fault_handler as usize, 0x08, 0x8E);
    
    // x87 FPU Error (#MF)
    idt.set_entry(16, fpu_error_handler as usize, 0x08, 0x8E);
    
    // Alignment Check (#AC) - with error code
    idt.set_entry(17, alignment_check_handler as usize, 0x08, 0x8E);
    
    // Machine Check (#MC)
    idt.set_entry(18, machine_check_handler as usize, 0x08, 0x8E);
    
    // SIMD FPU Exception (#XM)
    idt.set_entry(19, simd_fpu_handler as usize, 0x08, 0x8E);
}

// Exception handlers (stubs - would panic or handle in real impl)

#[no_mangle]
extern "C" fn divide_error_handler(_frame: &ExceptionFrame) {
    panic!("Divide Error!");
}

#[no_mangle]
extern "C" fn debug_handler(_frame: &ExceptionFrame) {
    panic!("Debug Exception!");
}

#[no_mangle]
extern "C" fn nmi_handler(_frame: &ExceptionFrame) {
    panic!("Non-Maskable Interrupt!");
}

#[no_mangle]
extern "C" fn breakpoint_handler(_frame: &ExceptionFrame) {
    panic!("Breakpoint!");
}

#[no_mangle]
extern "C" fn overflow_handler(_frame: &ExceptionFrame) {
    panic!("Overflow!");
}

#[no_mangle]
extern "C" fn bound_handler(_frame: &ExceptionFrame) {
    panic!("Bound Range Exceeded!");
}

#[no_mangle]
extern "C" fn invalid_opcode_handler(_frame: &ExceptionFrame) {
    panic!("Invalid Opcode!");
}

#[no_mangle]
extern "C" fn device_not_available_handler(_frame: &ExceptionFrame) {
    panic!("Device Not Available!");
}

#[no_mangle]
extern "C" fn double_fault_handler(_frame: &ExceptionFrame) {
    panic!("Double Fault! This is catastrophic.");
}

#[no_mangle]
extern "C" fn invalid_tss_handler(_frame: &ExceptionFrame, _error_code: u64) {
    panic!("Invalid TSS!");
}

#[no_mangle]
extern "C" fn segment_not_present_handler(_frame: &ExceptionFrame, _error_code: u64) {
    panic!("Segment Not Present!");
}

#[no_mangle]
extern "C" fn stack_fault_handler(_frame: &ExceptionFrame, _error_code: u64) {
    panic!("Stack Fault!");
}

#[no_mangle]
extern "C" fn general_protection_handler(_frame: &ExceptionFrame, _error_code: u64) {
    panic!("General Protection Fault!");
}

#[no_mangle]
extern "C" fn page_fault_handler(_frame: &ExceptionFrame, _error_code: u64) {
    // In real implementation, would check if user/kernel fault
    // and handle accordingly
    panic!("Page Fault!");
}

#[no_mangle]
extern "C" fn fpu_error_handler(_frame: &ExceptionFrame) {
    panic!("FPU Error!");
}

#[no_mangle]
extern "C" fn alignment_check_handler(_frame: &ExceptionFrame, _error_code: u64) {
    panic!("Alignment Check!");
}

#[no_mangle]
extern "C" fn machine_check_handler(_frame: &ExceptionFrame) {
    panic!("Machine Check!");
}

#[no_mangle]
extern "C" fn simd_fpu_handler(_frame: &ExceptionFrame) {
    panic!("SIMD FPU Exception!");
}

/// Page fault information
#[derive(Debug)]
pub struct PageFaultInfo {
    pub address: u64,
    pub present: bool,
    pub write: bool,
    pub user: bool,
    pub instruction_fetch: bool,
}

impl PageFaultInfo {
    /// Parse from error code
    pub fn from_error_code(error_code: u64, faulting_address: u64) -> Self {
        Self {
            address: faulting_address,
            present: (error_code & 0b1) == 0,
            write: (error_code & 0b10) != 0,
            user: (error_code & 0b100) != 0,
            instruction_fetch: (error_code & 0b10000) != 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_fault_info_parsing() {
        // User write to non-present page
        let info = PageFaultInfo::from_error_code(0b110, 0xDEADBEEF);
        assert!(!info.present);
        assert!(info.write);
        assert!(info.user);
        assert!(!info.instruction_fetch);
        assert_eq!(info.address, 0xDEADBEEF);
    }
}
