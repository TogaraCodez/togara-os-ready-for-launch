//! ELF64 Binary Loader
//! 
//! Part 13 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Loads and validates ELF64 executables for userspace.

/// ELF magic number
const ELF_MAGIC: &[u8; 4] = b"\x7fELF";

/// ELF class
const ELFCLASS64: u8 = 2;

/// ELF data encoding
const ELFDATA2LSB: u8 = 1;  // Little endian

/// ELF OS/ABI
const ELFOSABI_NONE: u8 = 0;

/// ELF type
const ET_EXEC: u16 = 2;  // Executable
const ET_DYN: u16 = 3;   // Shared object

/// ELF machine
const EM_X86_64: u16 = 62;

/// Program header type
const PT_LOAD: u32 = 1;  // Loadable segment

/// ELF64 header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

/// ELF64 program header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64ProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

/// ELF loading error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    TooSmall,
    BadMagic,
    WrongClass,
    WrongEndian,
    WrongArch,
    WrongType,
    InvalidHeader,
    InvalidProgramHeader,
    SegmentOutOfBounds,
    InvalidAlignment,
}

/// Loaded ELF program
#[derive(Debug)]
pub struct LoadedElf {
    pub entry: u64,
    pub segments: Vec<LoadedSegment>,
}

/// Loaded segment
#[derive(Debug)]
pub struct LoadedSegment {
    pub vaddr: u64,
    pub memsz: u64,
    pub filesz: u64,
    pub flags: u32,
}

/// Validate and load an ELF binary
/// 
/// # Safety
/// - data must point to valid memory containing an ELF binary
/// - Caller must ensure data remains valid during loading
pub unsafe fn load_elf(data: &[u8]) -> Result<LoadedElf, ElfError> {
    // Check minimum size
    if data.len() < core::mem::size_of::<Elf64Header>() {
        return Err(ElfError::TooSmall);
    }

    // Parse header
    let header = &*(data.as_ptr() as *const Elf64Header);

    // Validate ELF
    validate_elf(data, header)?;

    // Validate program headers
    if header.e_phentsize as usize != core::mem::size_of::<Elf64ProgramHeader>() {
        return Err(ElfError::InvalidProgramHeader);
    }

    // Check program header table doesn't overflow
    let ph_end = header.e_phoff
        .checked_add((header.e_phnum as u64)
            .checked_mul(header.e_phentsize as u64).ok_or(ElfError::InvalidHeader)?)
        .ok_or(ElfError::InvalidHeader)?;

    if ph_end > data.len() as u64 {
        return Err(ElfError::SegmentOutOfBounds);
    }

    // Load segments
    let mut segments = Vec::new();
    
    for i in 0..header.e_phnum as usize {
        let ph_offset = header.e_phoff as usize + i * header.e_phentsize as usize;
        let ph = &*(data.as_ptr().add(ph_offset) as *const Elf64ProgramHeader);

        // Only load PT_LOAD segments
        if ph.p_type != PT_LOAD {
            continue;
        }

        // Validate segment bounds
        let segment_end = ph.p_offset
            .checked_add(ph.p_filesz)
            .ok_or(ElfError::SegmentOutOfBounds)?;

        if segment_end > data.len() as u64 {
            return Err(ElfError::SegmentOutOfBounds);
        }

        // Validate alignment
        if ph.p_align != 0 && (ph.p_vaddr % ph.p_align) != 0 {
            return Err(ElfError::InvalidAlignment);
        }

        segments.push(LoadedSegment {
            vaddr: ph.p_vaddr,
            memsz: ph.p_memsz,
            filesz: ph.p_filesz,
            flags: ph.p_flags,
        });
    }

    Ok(LoadedElf {
        entry: header.e_entry,
        segments,
    })
}

/// Validate ELF header
fn validate_elf(data: &[u8], header: &Elf64Header) -> Result<(), ElfError> {
    // Check magic
    if &header.e_ident[0..4] != ELF_MAGIC {
        return Err(ElfError::BadMagic);
    }

    // Check class (64-bit)
    if header.e_ident[4] != ELFCLASS64 {
        return Err(ElfError::WrongClass);
    }

    // Check endianness (little endian)
    if header.e_ident[5] != ELFDATA2LSB {
        return Err(ElfError::WrongEndian);
    }

    // Check version
    if header.e_ident[6] != 1 {
        return Err(ElfError::InvalidHeader);
    }

    // Check OS/ABI
    if header.e_ident[7] != ELFOSABI_NONE {
        // Allow other ABIs for compatibility
    }

    // Check type (executable or shared object)
    if header.e_type != ET_EXEC && header.e_type != ET_DYN {
        return Err(ElfError::WrongType);
    }

    // Check machine (x86_64)
    if header.e_machine != EM_X86_64 {
        return Err(ElfError::WrongArch);
    }

    Ok(())
}

/// Calculate segment start (page-aligned)
pub const fn segment_start(vaddr: u64) -> u64 {
    (vaddr / 4096) * 4096
}

/// Calculate segment end (page-aligned, rounded up)
pub const fn segment_end(vaddr: u64, memsz: u64) -> u64 {
    let end = vaddr + memsz;
    if end % 4096 == 0 {
        end
    } else {
        ((end / 4096) + 1) * 4096
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_alignment() {
        assert_eq!(segment_start(0x1000), 0x1000);
        assert_eq!(segment_start(0x1234), 0x1000);
        assert_eq!(segment_start(0x2000), 0x2000);

        assert_eq!(segment_end(0x1000, 0x100), 0x2000);
        assert_eq!(segment_end(0x1000, 0x1000), 0x2000);
        assert_eq!(segment_end(0x1000, 0x1001), 0x3000);
    }

    #[test]
    fn test_minimal_elf_validation() {
        // Create a minimal valid ELF header (manually)
        let mut data = [0u8; 64];
        
        // Magic
        data[0..4].copy_from_slice(ELF_MAGIC);
        // Class: 64-bit
        data[4] = ELFCLASS64;
        // Data: little endian
        data[5] = ELFDATA2LSB;
        // Version
        data[6] = 1;
        // OS/ABI
        data[7] = ELFOSABI_NONE;
        
        // Type: executable
        data[16] = ET_EXEC as u8;
        data[17] = (ET_EXEC >> 8) as u8;
        
        // Machine: x86_64
        data[18] = EM_X86_64 as u8;
        data[19] = (EM_X86_64 >> 8) as u8;
        
        // Version
        data[20..24].copy_from_slice(&1u32.to_le_bytes());
        
        // Entry point
        data[24..32].copy_from_slice(&0x1000u64.to_le_bytes());
        
        // Program header offset
        data[32..40].copy_from_slice(&64u64.to_le_bytes());
        
        // Program header entry size
        data[54..56].copy_from_slice(&64u16.to_le_bytes());
        
        // Number of program headers
        data[56..58].copy_from_slice(&1u16.to_le_bytes());
        
        let header = unsafe { &*(data.as_ptr() as *const Elf64Header) };
        assert!(validate_elf(&data, header).is_ok());
    }

    #[test]
    fn test_bad_magic() {
        let data = [0u8; 64];
        let header = unsafe { &*(data.as_ptr() as *const Elf64Header) };
        assert_eq!(validate_elf(&data, header), Err(ElfError::BadMagic));
    }

    #[test]
    fn test_wrong_class() {
        let mut data = [0u8; 64];
        data[0..4].copy_from_slice(ELF_MAGIC);
        data[4] = 1; // 32-bit
        
        let header = unsafe { &*(data.as_ptr() as *const Elf64Header) };
        assert_eq!(validate_elf(&data, header), Err(ElfError::WrongClass));
    }

    #[test]
    fn test_wrong_arch() {
        let mut data = [0u8; 64];
        data[0..4].copy_from_slice(ELF_MAGIC);
        data[4] = ELFCLASS64;
        data[5] = ELFDATA2LSB;
        data[6] = 1;
        data[16] = ET_EXEC as u8;
        data[18] = 3; // x86 (not x86_64)
        
        let header = unsafe { &*(data.as_ptr() as *const Elf64Header) };
        assert_eq!(validate_elf(&data, header), Err(ElfError::WrongArch));
    }
}
