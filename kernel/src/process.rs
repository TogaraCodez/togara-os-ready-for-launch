//! Process Management and ELF Loading
//! 
//! Parts 11-13 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Loads and runs ELF binaries in userspace.

use crate::elf::{self, Elf64Header, LoadedElf};
use crate::memory::vmm::{Page, PagePerm, VirtAddr};
use crate::memory::pmm::{PhysFrame, PAGE_SIZE};
use crate::scheduler::{Task, TaskId};

/// Process control block
pub struct Process {
    pub pid: TaskId,
    pub state: ProcessState,
    pub entry: u64,
    pub stack_top: u64,
}

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Ready,
    Blocked,
    Exited(i32),
}

impl Process {
    /// Create a new process from an ELF binary
    /// 
    /// # Safety
    /// - elf_data must point to valid ELF binary
    /// - Caller must ensure memory is properly mapped
    pub unsafe fn from_elf(elf_data: &[u8], pid: TaskId) -> Result<Self, ProcessError> {
        // Load ELF
        let loaded = elf::load_elf(elf_data)?;
        
        // For now, use a simple stack allocation
        let stack_top = 0x7FFF_FFFF_F000; // High userspace address
        
        Ok(Self {
            pid,
            state: ProcessState::Ready,
            entry: loaded.entry,
            stack_top,
        })
    }

    /// Create a task for this process
    pub fn create_task(&self) -> Task {
        use crate::context::create_userspace_task;
        
        unsafe {
            create_userspace_task(
                self.pid.0,
                self.entry,
                self.stack_top,
            )
        }
    }
}

/// Process error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessError {
    ElfError(elf::ElfError),
    OutOfMemory,
    InvalidAddress,
}

impl From<elf::ElfError> for ProcessError {
    fn from(err: elf::ElfError) -> Self {
        ProcessError::ElfError(err)
    }
}

/// Simple "Hello World" ELF binary (x86_64, statically linked)
/// 
/// This is a minimal ELF that writes "Hello from userspace!" to stdout
/// and exits. In a real implementation, this would be loaded from disk.
pub const HELLO_WORLD_ELF: &[u8] = include_bytes!("../hello.elf");

/// Load and run the hello world program
/// 
/// # Safety
/// - Requires kernel to be initialized
pub unsafe fn run_hello_world() -> Result<(), ProcessError> {
    // Create process from ELF
    let process = Process::from_elf(HELLO_WORLD_ELF, TaskId(1))?;
    
    // In real implementation, would add to scheduler and run
    // For now, just validate it loads
    assert!(process.entry != 0);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world_elf_exists() {
        assert!(!HELLO_WORLD_ELF.is_empty());
        assert!(HELLO_WORLD_ELF.len() > 64); // At least ELF header size
    }

    #[test]
    fn test_hello_world_elf_magic() {
        // Check ELF magic
        assert_eq!(&HELLO_WORLD_ELF[0..4], b"\x7fELF");
    }
}
