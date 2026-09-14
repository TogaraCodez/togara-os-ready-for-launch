//! Interrupt Descriptor Table (IDT)
//! 
//! Part 6 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Defines exception and interrupt handlers.

/// IDT entry (16 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    options: u16,
    offset_middle: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    /// Create a new IDT entry
    pub const fn new(handler: usize, selector: u16, options: u16) -> Self {
        Self {
            offset_low: handler as u16,
            selector,
            options,
            offset_middle: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            reserved: 0,
        }
    }

    /// Create a null entry
    pub const fn null() -> Self {
        Self::new(0, 0, 0)
    }
}

/// IDT pointer for lidt instruction
#[repr(C, packed)]
pub struct IdtPointer {
    limit: u16,
    base: u64,
}

/// Interrupt Descriptor Table
#[repr(C, align(16))]
pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    /// Create a new empty IDT
    pub const fn new() -> Self {
        const EMPTY_ENTRY: IdtEntry = IdtEntry::null();
        Self {
            entries: [EMPTY_ENTRY; 256],
        }
    }

    /// Set an IDT entry
    pub fn set_entry(&mut self, index: usize, handler: usize, selector: u16, options: u16) {
        self.entries[index] = IdtEntry::new(handler, selector, options);
    }

    /// Load the IDT
    /// 
    /// # Safety
    /// - This modifies CPU state
    pub unsafe fn load(&'static self) {
        let ptr = IdtPointer {
            limit: (core::mem::size_of::<Self>() - 1) as u16,
            base: self as *const Self as u64,
        };
        
        // In actual implementation:
        // core::arch::asm!("lidt [{}]", in(reg) &ptr);
    }

    /// Get entries
    pub fn entries(&self) -> &[IdtEntry; 256] {
        &self.entries
    }
}

impl Default for Idt {
    fn default() -> Self {
        Self::new()
    }
}

/// Exception handler function type
pub type ExceptionHandler = fn(&ExceptionFrame);

/// Exception stack frame
#[repr(C)]
#[derive(Debug)]
pub struct ExceptionFrame {
    pub instruction_pointer: u64,
    pub code_segment: u64,
    pub cpu_flags: u64,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}

/// Exception information
#[derive(Debug)]
pub struct ExceptionInfo {
    pub vector: u8,
    pub error_code: Option<u64>,
    pub frame: ExceptionFrame,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idt_size() {
        // IDT should be 256 * 16 = 4096 bytes
        assert_eq!(core::mem::size_of::<Idt>(), 4096);
    }

    #[test]
    fn test_idt_entry_creation() {
        let entry = IdtEntry::new(0xDEADBEEF, 0x08, 0x8E);
        assert_ne!(entry.offset_low, 0);
    }
}
