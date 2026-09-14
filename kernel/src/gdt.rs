//! Global Descriptor Table (GDT)
//! 
//! Part 6 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Defines kernel and user code/data segments.

/// GDT entry (8 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Descriptor {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl Descriptor {
    /// Create a new GDT entry
    pub const fn new(
        base: u32,
        limit: u32,
        access: u8,
        granularity: u8,
    ) -> Self {
        Self {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_middle: ((base >> 16) & 0xFF) as u8,
            access,
            granularity: ((limit >> 16) & 0xF) as u8 | (granularity & 0xF0),
            base_high: ((base >> 24) & 0xFF) as u8,
        }
    }

    /// Create a null descriptor
    pub const fn null() -> Self {
        Self::new(0, 0, 0, 0)
    }

    /// Create a code segment descriptor
    pub const fn code_segment(ring: u8) -> Self {
        // Access byte: Present (bit 7), Ring (bits 5-6), S=1 (bit 4),
        // Type=1010 (executable, readable, not conforming) (bits 3-0)
        let access = 0b1001_1010 | (ring & 0x03) << 5;
        // Granularity: 4KB pages, 32-bit protected mode
        let granularity = 0b1100_0000;
        Self::new(0, 0xFFFFFFFF, access, granularity)
    }

    /// Create a data segment descriptor
    pub const fn data_segment(ring: u8) -> Self {
        // Access byte: Present, Ring, S=1, Type=0010 (writable, not expand-down)
        let access = 0b1001_0010 | (ring & 0x03) << 5;
        let granularity = 0b1100_0000;
        Self::new(0, 0xFFFFFFFF, access, granularity)
    }
}

/// TSS descriptor (16 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TssDescriptor {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    limit_high: u8,
    base_high: u16,
    base_upper: u32,
    reserved: u32,
}

impl TssDescriptor {
    /// Create a new TSS descriptor
    pub const fn new(base: u64, limit: u32) -> Self {
        Self {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_middle: ((base >> 16) & 0xFF) as u8,
            access: 0b1000_1001, // Present, Ring 0, S=0, Type=9 (64-bit TSS)
            limit_high: ((limit >> 16) & 0xF) as u8,
            base_high: ((base >> 16) & 0xFFFF) as u16,
            base_upper: ((base >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }
}

/// GDT pointer for lgdt instruction
#[repr(C, packed)]
pub struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

/// GDT with predefined segments
#[repr(C, align(16))]
pub struct Gdt {
    null: Descriptor,
    kernel_code: Descriptor,
    kernel_data: Descriptor,
    user_code: Descriptor,
    user_data: Descriptor,
    tss: TssDescriptor,
}

impl Gdt {
    /// Create a new GDT with standard segments
    pub const fn new() -> Self {
        Self {
            null: Descriptor::null(),
            kernel_code: Descriptor::code_segment(0),
            kernel_data: Descriptor::data_segment(0),
            user_code: Descriptor::code_segment(3),
            user_data: Descriptor::data_segment(3),
            tss: TssDescriptor::new(0, 0),
        }
    }

    /// Set the TSS descriptor
    pub fn set_tss(&mut self, tss_base: u64, tss_limit: u32) {
        self.tss = TssDescriptor::new(tss_base, tss_limit);
    }

    /// Load the GDT
    /// 
    /// # Safety
    /// - This modifies CPU state
    /// - Must be called on each CPU
    pub unsafe fn load(&'static self) {
        let ptr = DescriptorTablePointer {
            limit: (core::mem::size_of::<Self>() - 1) as u16,
            base: self as *const Self as u64,
        };
        
        // In actual implementation, would use inline assembly:
        // core::arch::asm!("lgdt [{}]", in(reg) &ptr);
        
        // Load segment selectors
        // core::arch::asm!(
        //     "push 0x10",  // kernel data
        //     "pop %rax",
        //     "mov ds, %ax",
        //     "mov es, %ax",
        //     "mov ss, %ax",
        //     out("rax") _
        // );
    }

    /// Get segment selectors
    pub const fn selectors(&self) -> SegmentSelectors {
        SegmentSelectors {
            kernel_code: 0x08,
            kernel_data: 0x10,
            user_code: 0x18 | 3,
            user_data: 0x20 | 3,
            tss: 0x28,
        }
    }
}

impl Default for Gdt {
    fn default() -> Self {
        Self::new()
    }
}

/// Segment selectors
#[derive(Debug, Clone, Copy)]
pub struct SegmentSelectors {
    pub kernel_code: u16,
    pub kernel_data: u16,
    pub user_code: u16,
    pub user_data: u16,
    pub tss: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdt_size() {
        // GDT should be properly sized
        assert_eq!(core::mem::size_of::<Gdt>(), 6 * 8 + 16); // 5 descriptors + 1 TSS
    }

    #[test]
    fn test_descriptor_creation() {
        let code = Descriptor::code_segment(0);
        assert_ne!(code.access, 0);
        
        let data = Descriptor::data_segment(3);
        assert_ne!(data.access, 0);
    }
}
