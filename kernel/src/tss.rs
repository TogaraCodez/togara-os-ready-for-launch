//! Task State Segment (TSS)
//! 
//! Part 6 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Provides stack switching for exceptions and interrupts.

/// Task State Segment for x86_64
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved1: u32,
    pub rsp0: u64,  // Stack pointer for ring 0
    pub rsp1: u64,  // Stack pointer for ring 1 (unused)
    pub rsp2: u64,  // Stack pointer for ring 2 (unused)
    reserved2: u64,
    pub ist1: u64,  // Interrupt stack table 1 (e.g., double fault)
    pub ist2: u64,
    pub ist3: u64,
    pub ist4: u64,
    pub ist5: u64,
    pub ist6: u64,
    pub ist7: u64,
    reserved3: u64,
    reserved4: u16,
    pub iomap_base: u16,  // I/O map base (0xFFFF = no I/O map)
}

impl TaskStateSegment {
    /// Create a new TSS
    pub const fn new() -> Self {
        Self {
            reserved1: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved2: 0,
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            reserved3: 0,
            reserved4: 0,
            iomap_base: 0xFFFF,  // No I/O permission map
        }
    }

    /// Set the ring 0 stack pointer
    pub fn set_kernel_stack(&mut self, stack: u64) {
        self.rsp0 = stack;
    }

    /// Set the double-fault stack (IST1)
    pub fn set_double_fault_stack(&mut self, stack: u64) {
        self.ist1 = stack;
    }

    /// Get the TSS size
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl Default for TaskStateSegment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tss_size() {
        // TSS should be 104 bytes on x86_64
        assert_eq!(TaskStateSegment::size(), 104);
    }

    #[test]
    fn test_tss_initialization() {
        let tss = TaskStateSegment::new();
        assert_eq!(tss.rsp0, 0);
        assert_eq!(tss.ist1, 0);
        assert_eq!(tss.iomap_base, 0xFFFF);
    }
}
