//! Multiboot2 Header
//! 
//! Priority 7: Bootloader Integration
//! Classification: RUNTIME
//! 
//! Defines multiboot2 header that must be in first 32KB of kernel binary.

/// Multiboot2 magic
const MULTIBOOT2_MAGIC: u32 = 0xE85250D6;

/// Architecture (i386 protected mode)
const ARCH_I386: u32 = 0;

/// Header length
const HEADER_LENGTH: u32 = 16 + 8 + 8; // header + architecture tag + end tag

/// Multiboot2 header (must be aligned to 8 bytes)
#[repr(C, align(8))]
pub struct Multiboot2Header {
    pub magic: u32,
    pub architecture: u32,
    pub header_length: u32,
    pub checksum: u32,
    pub tags: [u8; 16], // Architecture tag + End tag
}

impl Multiboot2Header {
    /// Create a new multiboot2 header
    pub const fn new() -> Self {
        let mut header = Self {
            magic: MULTIBOOT2_MAGIC,
            architecture: ARCH_I386,
            header_length: HEADER_LENGTH,
            checksum: 0,
            tags: [0; 16],
        };
        
        // Calculate checksum
        header.checksum = !(MULTIBOOT2_MAGIC
            .wrapping_add(ARCH_I386)
            .wrapping_add(HEADER_LENGTH));
        
        // Architecture tag (type=5, flags=0, size=12, value=0)
        header.tags[0] = 5;  // Type
        header.tags[4] = 12; // Size (little endian)
        
        // End tag (type=0, flags=0, size=8)
        header.tags[8] = 0;  // Type
        header.tags[12] = 8; // Size
        
        header
    }
}

/// Global multiboot2 header
#[no_mangle]
#[used]
pub static MULTIBOOT2_HEADER: Multiboot2Header = Multiboot2Header::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_magic() {
        let header = Multiboot2Header::new();
        assert_eq!(header.magic, MULTIBOOT2_MAGIC);
    }

    #[test]
    fn test_header_checksum() {
        let header = Multiboot2Header::new();
        // Checksum should make total sum zero
        let sum = header.magic
            .wrapping_add(header.architecture)
            .wrapping_add(header.header_length)
            .wrapping_add(header.checksum);
        assert_eq!(sum, 0);
    }
}
