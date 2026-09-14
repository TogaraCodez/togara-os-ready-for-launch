//! Multiboot2 Bootloader Support
//! 
//! Priority 7: Bootloader Integration
//! Classification: RUNTIME
//! 
//! Parses Multiboot2 information structure from bootloader.

/// Multiboot2 magic number
pub const MULTIBOOT2_MAGIC: u32 = 0x36D76289;

/// Multiboot2 boot information structure
#[repr(C)]
pub struct MultibootInfo {
    pub total_size: u32,
    pub reserved: u32,
}

/// Multiboot2 tag header
#[repr(C)]
pub struct TagHeader {
    pub typ: u32,
    pub size: u32,
}

/// Multiboot2 tag types
pub mod tag_types {
    pub const END: u32 = 0;
    pub const BOOT_LOADER_NAME: u32 = 2;
    pub const MMAP: u32 = 6;
    pub const MODULE: u32 = 9;
    pub const MEMORY_MAP: u32 = 6;
}

/// Memory map tag
#[repr(C)]
pub struct MemoryMapTag {
    pub typ: u32,
    pub size: u32,
    pub entry_size: u32,
    pub entry_version: u32,
}

/// Memory map entry
#[repr(C)]
pub struct MemoryMapEntry {
    pub typ: u32,
    pub reserved: u32,
    pub addr: u64,
    pub len: u64,
}

/// Memory types
pub mod memory_types {
    pub const AVAILABLE: u32 = 1;
    pub const RESERVED: u32 = 2;
    pub const ACPI_RECLAIMABLE: u32 = 3;
    pub const ACPI_NVS: u32 = 4;
    pub const BAD_MEMORY: u32 = 5;
}

/// Module tag (for loaded ELF files)
#[repr(C)]
pub struct ModuleTag {
    pub typ: u32,
    pub size: u32,
    pub mod_start: u32,
    pub mod_end: u32,
    pub cmdline: u8,
}

/// Parse multiboot2 information
/// 
/// # Safety
/// - info must point to valid multiboot2 structure
/// - Called by bootloader before kernel_main
pub unsafe fn parse_multiboot_info(info: *const u8) -> Result<(), &'static str> {
    if info.is_null() {
        return Err("No multiboot info provided");
    }

    let multiboot = &*(info as *const MultibootInfo);
    
    // Iterate through tags
    let mut current = info.add(8) as *const TagHeader; // Skip multiboot info header
    
    loop {
        if (*current).typ == tag_types::END {
            break;
        }
        
        match (*current).typ {
            tag_types::MEMORY_MAP => {
                // Parse memory map
                parse_memory_map(current as *const MemoryMapTag)?;
            }
            tag_types::MODULE => {
                // Parse loaded modules (e.g., hello.elf)
                parse_module(current as *const ModuleTag)?;
            }
            _ => {
                // Unknown tag, skip
            }
        }
        
        // Move to next tag (aligned to 8 bytes)
        let next_offset = ((*current).size + 7) & !7;
        current = (current as *const u8).add(next_offset as usize) as *const TagHeader;
    }
    
    Ok(())
}

/// Parse memory map from multiboot2
/// 
/// # Safety
/// - tag must point to valid memory map tag
unsafe fn parse_memory_map(tag: *const MemoryMapTag) -> Result<(), &'static str> {
    let entry_size = (*tag).entry_size as usize;
    let entry_count = ((*tag).size as usize - 16) / entry_size; // Subtract header
    
    let entry_ptr = (tag as *const u8).add(16) as *const MemoryMapEntry;
    
    for i in 0..entry_count {
        let entry = &*entry_ptr.add(i);
        
        if entry.typ == memory_types::AVAILABLE {
            // This memory is available for use
            // In real implementation, would add to frame allocator
        }
    }
    
    Ok(())
}

/// Parse module (loaded file)
/// 
/// # Safety
/// - tag must point to valid module tag
unsafe fn parse_module(tag: *const ModuleTag) -> Result<(), &'static str> {
    let start = (*tag).mod_start as usize;
    let end = (*tag).mod_end as usize;
    let size = end - start;
    
    // Module is loaded at physical address `start` with size `size`
    // In real implementation, would parse as ELF and load
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiboot_magic() {
        assert_eq!(MULTIBOOT2_MAGIC, 0x36D76289);
    }
}
