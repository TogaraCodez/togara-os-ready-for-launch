//! System Call Interface
//! 
//! Part 14 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Provides syscall ABI for userspace programs.

use crate::idt::ExceptionFrame;

/// Syscall numbers
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallNumber {
    Exit = 0,
    Write = 1,
    Read = 2,
    Open = 3,
    Close = 4,
    Spawn = 5,
    Wait = 6,
    Sleep = 7,
    Yield = 8,
}

impl SyscallNumber {
    /// Convert from u64
    pub fn from_number(n: u64) -> Option<Self> {
        match n {
            0 => Some(Self::Exit),
            1 => Some(Self::Write),
            2 => Some(Self::Read),
            3 => Some(Self::Open),
            4 => Some(Self::Close),
            5 => Some(Self::Spawn),
            6 => Some(Self::Wait),
            7 => Some(Self::Sleep),
            8 => Some(Self::Yield),
            _ => None,
        }
    }
}

/// Syscall result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SyscallResult(pub i64);

impl SyscallResult {
    pub const fn success(value: i64) -> Self {
        Self(value)
    }

    pub const fn error(code: i64) -> Self {
        Self(-code)
    }

    pub const fn is_error(&self) -> bool {
        self.0 < 0
    }

    pub const fn value(&self) -> i64 {
        self.0
    }
}

/// Error codes
pub mod errno {
    pub const SUCCESS: i64 = 0;
    pub const EPERM: i64 = 1;      // Operation not permitted
    pub const ENOENT: i64 = 2;     // No such file or directory
    pub const ESRCH: i64 = 3;      // No such process
    pub const EINTR: i64 = 4;      // Interrupted system call
    pub const EIO: i64 = 5;        // I/O error
    pub const EINVAL: i64 = 22;    // Invalid argument
    pub const EFAULT: i64 = 14;    // Bad address
    pub const ENOMEM: i64 = 12;    // Out of memory
}

/// Syscall arguments
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SyscallArgs {
    pub number: u64,
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: u64,
    pub arg4: u64,
    pub arg5: u64,
}

/// Syscall handler function type
pub type SyscallHandler = fn(&SyscallArgs) -> SyscallResult;

/// Initialize syscalls
pub fn init_syscalls() {
    // In actual implementation, would set up syscall entry points
    // For x86_64, this would configure STAR, LSTAR, SFMASK, KERNEL_GS_BASE MSRs
}

/// Handle a syscall from userspace
/// 
/// # Safety
/// - All user pointers must be validated before dereferencing
pub unsafe fn handle_syscall(args: &SyscallArgs, frame: &ExceptionFrame) -> SyscallResult {
    match SyscallNumber::from_number(args.number) {
        Some(SyscallNumber::Exit) => sys_exit(args.arg0 as i32),
        Some(SyscallNumber::Write) => sys_write(args.arg0, args.arg1 as *const u8, args.arg2),
        Some(SyscallNumber::Read) => sys_read(args.arg0, args.arg1 as *mut u8, args.arg2),
        Some(SyscallNumber::Yield) => sys_yield(),
        _ => SyscallResult::error(errno::EINVAL),
    }
}

/// SYS_EXIT - Terminate the current process
fn sys_exit(status: i32) -> SyscallResult {
    // In real implementation, would clean up process and schedule next
    SyscallResult::success(status as i64)
}

/// SYS_WRITE - Write to a file descriptor
/// 
/// # Safety
/// - buf must be validated as a valid user pointer
fn sys_write(fd: u64, buf: *const u8, count: u64) -> SyscallResult {
    // Validate user pointer
    if !validate_user_pointer(buf as u64, count) {
        return SyscallResult::error(errno::EFAULT);
    }

    // For now, only support stdout (fd 1) and stderr (fd 2)
    if fd != 1 && fd != 2 {
        return SyscallResult::error(errno::EBADF);
    }

    // In real implementation, would write to the file descriptor
    // For now, just return success with bytes "written"
    SyscallResult::success(count as i64)
}

/// SYS_READ - Read from a file descriptor
/// 
/// # Safety
/// - buf must be validated as a valid user pointer
fn sys_read(fd: u64, buf: *mut u8, count: u64) -> SyscallResult {
    // Validate user pointer
    if !validate_user_pointer(buf as u64, count) {
        return SyscallResult::error(errno::EFAULT);
    }

    // For now, only support stdin (fd 0)
    if fd != 0 {
        return SyscallResult::error(errno::EBADF);
    }

    // In real implementation, would read from the file descriptor
    SyscallResult::success(0) // EOF for now
}

/// SYS_YIELD - Yield the CPU to another task
fn sys_yield() -> SyscallResult {
    // In real implementation, would call scheduler
    SyscallResult::success(0)
}

/// Validate a user pointer
/// 
/// # Safety
/// - This function checks if a pointer is in userspace
/// - Caller must still be careful when dereferencing
fn validate_user_pointer(ptr: u64, size: u64) -> bool {
    // Check if pointer is in userspace (lower half)
    if ptr >= 0xFFFF8000_00000000 {
        return false;
    }

    // Check for overflow
    let end = match ptr.checked_add(size) {
        Some(e) => e,
        None => return false,
    };

    // Check that end is also in userspace
    if end >= 0xFFFF8000_00000000 {
        return false;
    }

    true
}

/// Enter userspace
/// 
/// # Safety
/// - This is the initial transition from kernel to userspace
/// - The entry point must be a valid userspace address
/// - Stack must be properly set up
pub unsafe fn enter_userspace(entry: u64, stack_top: u64) -> ! {
    // In actual implementation, would use IRETQ to transition to Ring 3
    // with user code segment and stack
    
    // For now, this is a stub
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_number_conversion() {
        assert_eq!(SyscallNumber::from_number(0), Some(SyscallNumber::Exit));
        assert_eq!(SyscallNumber::from_number(1), Some(SyscallNumber::Write));
        assert_eq!(SyscallNumber::from_number(999), None);
    }

    #[test]
    fn test_syscall_result() {
        let ok = SyscallResult::success(42);
        assert!(!ok.is_error());
        assert_eq!(ok.value(), 42);

        let err = SyscallResult::error(errno::EINVAL);
        assert!(err.is_error());
        assert_eq!(err.value(), -22);
    }

    #[test]
    fn test_user_pointer_validation() {
        // Valid userspace pointers
        assert!(validate_user_pointer(0x1000, 100));
        assert!(validate_user_pointer(0x7FFF_FFFF_F000, 0x1000));
        
        // Kernel pointers (invalid)
        assert!(!validate_user_pointer(0xFFFF8000_00000000, 100));
        assert!(!validate_user_pointer(0xFFFF_FFFF_FFFF_FFFF, 100));
        
        // Overflow (invalid)
        assert!(!validate_user_pointer(0xFFFF_FFFF_FFFF_F000, 0x2000));
    }
}
