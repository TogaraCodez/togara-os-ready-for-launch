//! Architecture-specific code for x86_64
//! 
//! Priority 2: Userspace Isolation
//! Classification: RUNTIME

pub mod context_switch;
pub mod iretq;
pub mod syscall_entry;

extern "C" {
    /// IRETQ entry to Ring 3
    pub fn iretq_entry(
        entry_point: u64,
        user_stack: u64,
        code_selector: u64,
        stack_selector: u64,
        flags: u64,
    ) -> !;
    
    /// Syscall handler entry
    pub fn syscall_entry();
}

/// Enter userspace via IRETQ
/// 
/// # Safety
/// - Entry point must be valid userspace address
/// - Stack must be properly allocated
/// - Segment selectors must be valid
pub unsafe fn enter_userspace(
    entry: u64,
    stack_top: u64,
    code_selector: u16,
    stack_selector: u16,
    flags: u64,
) -> ! {
    iretq::iretq_entry(
        entry,
        stack_top,
        code_selector as u64,
        stack_selector as u64,
        flags,
    )
}

/// Configure SYSCALL MSRs
/// 
/// # Safety
/// - Writes to model-specific registers
pub unsafe fn configure_syscalls(
    kernel_cs: u16,
    kernel_ss: u16,
    user_cs: u16,
    user_ss: u16,
    syscall_entry: u64,
) {
    use x86_64::registers::model_specific::{Msr, Efer, EferFlags};
    
    // IA32_STAR: Syscall target CS/SS
    // Bits 47:32 = kernel CS
    // Bits 31:16 = kernel SS
    // Bits 63:48 = user CS (for SYSRET)
    // Bits 15:0 = user SS (for SYSRET)
    let star = ((kernel_cs as u64) << 32)
        | ((kernel_ss as u64) << 16)
        | ((user_cs as u64) << 48)
        | ((user_ss as u64) << 0);
    
    Msr::new(0xC000_0081).write(star); // IA32_STAR
    
    // IA32_LSTAR: Syscall entry point (RIP)
    Msr::new(0xC000_0082).write(syscall_entry);
    
    // IA32_FMASK: Flags to mask on syscall
    Msr::new(0xC000_0084).write(0x0020_0200); // Mask TF and NT
    
    // Enable SYSCALL in EFER
    Efer::update(|efer| *efer |= EferFlags::SYSTEM_CALL_EXTENSIONS);
}
