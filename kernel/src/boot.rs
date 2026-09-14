//! Boot system - BIOS/UEFI boot context handling
//! 
//! Part 2 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME

use core::fmt::{self, Write};

/// Boot context provided by bootloader
#[repr(C)]
pub struct BootContext<'a> {
    pub memory_map: &'a [MemoryRegion],
    pub framebuffer: Option<FramebufferInfo>,
    pub physical_memory_offset: Option<u64>,
}

/// Memory region from bootloader
#[derive(Copy, Clone, Debug)]
pub struct MemoryRegion {
    pub start: u64,
    pub len: u64,
    pub usable: bool,
}

/// Framebuffer information
#[derive(Copy, Clone, Debug)]
pub struct FramebufferInfo {
    pub addr: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
}

/// Simple VGA text mode writer for early boot
pub struct BootWriter {
    buffer: &'static mut str,
    pos: usize,
}

impl BootWriter {
    pub const fn new(buffer: &'static mut str) -> Self {
        Self { buffer, pos: 0 }
    }
}

impl Write for BootWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if self.pos < self.buffer.len() - 1 {
                unsafe {
                    let ptr = self.buffer.as_mut_ptr().add(self.pos);
                    ptr.write(byte);
                }
                self.pos += 1;
            }
        }
        Ok(())
    }
}

/// Print boot status message
pub fn print_boot_status(writer: &mut BootWriter, subsystem: &str, ok: bool) {
    let status = if ok { "OK" } else { "FAIL" };
    let _ = write!(writer, "{}: {}\n", subsystem, status);
}

/// Validate boot context
pub fn validate_boot_context(ctx: &BootContext) -> Result<(), &'static str> {
    // Validate memory regions with integer safety
    for region in ctx.memory_map {
        // Check for overflow: start + len must not overflow
        if region.start.checked_add(region.len).is_none() {
            return Err("Memory region overflow detected");
        }
        
        // Usable regions must have non-zero length
        if region.usable && region.len == 0 {
            return Err("Usable memory region with zero length");
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_memory_region() {
        let regions = [MemoryRegion {
            start: 0x100000,
            len: 0x10000,
            usable: true,
        }];
        
        let ctx = BootContext {
            memory_map: &regions,
            framebuffer: None,
            physical_memory_offset: None,
        };
        
        assert!(validate_boot_context(&ctx).is_ok());
    }

    #[test]
    fn test_overflow_detection() {
        let regions = [MemoryRegion {
            start: u64::MAX,
            len: 1,
            usable: true,
        }];
        
        let ctx = BootContext {
            memory_map: &regions,
            framebuffer: None,
            physical_memory_offset: None,
        };
        
        assert!(validate_boot_context(&ctx).is_err());
    }
}
