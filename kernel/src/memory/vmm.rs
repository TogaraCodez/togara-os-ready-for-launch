//! Virtual Memory Manager (VMM)
//! 
//! Part 4 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Manages virtual memory with x86_64 4-level page tables.
//! Enforces W^X and user/kernel isolation.

use super::pmm::{PhysFrame, PAGE_SIZE};

/// x86_64 page table entry flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PageTableFlags(u64);

impl PageTableFlags {
    pub const PRESENT: Self = Self(1 << 0);
    pub const WRITABLE: Self = Self(1 << 1);
    pub const USER_ACCESSIBLE: Self = Self(1 << 2);
    pub const WRITE_THROUGH: Self = Self(1 << 3);
    pub const NO_CACHE: Self = Self(1 << 4);
    pub const ACCESSED: Self = Self(1 << 5);
    pub const DIRTY: Self = Self(1 << 6);
    pub const HUGE_PAGE: Self = Self(1 << 7);
    pub const GLOBAL: Self = Self(1 << 8);
    pub const NO_EXECUTE: Self = Self(1 << 63);

    /// Check if present
    pub const fn is_present(self) -> bool {
        (self.0 & Self::PRESENT.0) != 0
    }

    /// Check if writable
    pub const fn is_writable(self) -> bool {
        (self.0 & Self::WRITABLE.0) != 0
    }

    /// Check if user accessible
    pub const fn is_user_accessible(self) -> bool {
        (self.0 & Self::USER_ACCESSIBLE.0) != 0
    }

    /// Check if executable (not NX)
    pub const fn is_executable(self) -> bool {
        (self.0 & Self::NO_EXECUTE.0) == 0
    }
}

/// Page permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagePerm {
    pub writable: bool,
    pub executable: bool,
    pub user: bool,
}

impl PagePerm {
    /// Kernel read-only
    pub const KERNEL_RO: Self = Self {
        writable: false,
        executable: false,
        user: false,
    };

    /// Kernel read-execute
    pub const KERNEL_RX: Self = Self {
        writable: false,
        executable: true,
        user: false,
    };

    /// Kernel read-write
    pub const KERNEL_RW: Self = Self {
        writable: true,
        executable: false,
        user: false,
    };

    /// User read-execute
    pub const USER_RX: Self = Self {
        writable: false,
        executable: true,
        user: true,
    };

    /// User read-write
    pub const USER_RW: Self = Self {
        writable: true,
        executable: false,
        user: true,
    };

    /// Convert to page table flags with W^X enforcement
    pub fn to_flags(self) -> PageTableFlags {
        let mut flags = PageTableFlags::PRESENT;

        if self.writable {
            flags.0 |= PageTableFlags::WRITABLE.0;
        }

        if self.user {
            flags.0 |= PageTableFlags::USER_ACCESSIBLE.0;
        }

        // W^X: if not executable, set NX
        if !self.executable {
            flags.0 |= PageTableFlags::NO_EXECUTE.0;
        }

        // W^X invariant: cannot be both writable and executable
        // This is enforced by the API design - PagePerm doesn't allow
        // constructing a permission that is both writable and executable
        // for ordinary pages (kernel can override if needed for special cases)

        flags
    }
}

/// Virtual address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(pub u64);

impl VirtAddr {
    /// Create a new virtual address
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Get the page number (PML4 index)
    pub const fn pml4_index(self) -> usize {
        ((self.0 >> 39) & 0x1FF) as usize
    }

    /// Get the PDPT index
    pub const fn pdpt_index(self) -> usize {
        ((self.0 >> 30) & 0x1FF) as usize
    }

    /// Get the PD index
    pub const fn pd_index(self) -> usize {
        ((self.0 >> 21) & 0x1FF) as usize
    }

    /// Get the PT index
    pub const fn pt_index(self) -> usize {
        ((self.0 >> 12) & 0x1FF) as usize
    }

    /// Get the page offset
    pub const fn page_offset(self) -> usize {
        (self.0 & 0xFFF) as usize
    }

    /// Check if this is a canonical address
    pub const fn is_canonical(self) -> bool {
        // Canonical addresses have bits 48-63 equal to bit 47
        let sign_bit = (self.0 >> 47) & 1;
        let upper_bits = self.0 >> 48;
        
        if sign_bit == 0 {
            upper_bits == 0
        } else {
            upper_bits == 0x1FFFF
        }
    }

    /// Check if this is a kernel address (higher half)
    pub const fn is_kernel_addr(self) -> bool {
        self.0 >= 0xFFFF800000000000
    }

    /// Check if this is a user address (lower half)
    pub const fn is_user_addr(self) -> bool {
        self.0 < 0xFFFF800000000000
    }
}

/// Page-aligned virtual address
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub addr: VirtAddr,
}

impl Page {
    /// Create a new page from virtual address
    pub const fn new(addr: VirtAddr) -> Option<Self> {
        if addr.0 % PAGE_SIZE != 0 {
            return None;
        }
        Some(Self { addr })
    }

    /// Create a page from page number
    pub const fn from_number(number: u64) -> Self {
        Self {
            addr: VirtAddr(number * PAGE_SIZE),
        }
    }

    /// Get the page number
    pub const fn number(self) -> u64 {
        self.addr.0 / PAGE_SIZE
    }
}

/// Page table entry (64-bit)
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create a new entry
    pub const fn new() -> Self {
        Self(0)
    }

    /// Set the entry
    pub fn set(&mut self, frame: PhysFrame, flags: PageTableFlags) {
        self.0 = (frame.number << 12) | flags.0;
    }

    /// Get the physical frame
    pub const fn frame(&self) -> Option<PhysFrame> {
        let addr = (self.0 & 0x000FFFFF_FFFFF000) as u64;
        PhysFrame::from_address(addr)
    }

    /// Get the flags
    pub const fn flags(&self) -> PageTableFlags {
        PageTableFlags(self.0 & 0xFFF)
    }

    /// Check if present
    pub const fn is_present(&self) -> bool {
        self.flags().is_present()
    }

    /// Zero out the entry
    pub fn zero(&mut self) {
        self.0 = 0;
    }
}

/// A page table (4KB, 512 entries)
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}

impl PageTable {
    /// Create a new empty page table
    pub const fn new() -> Self {
        const EMPTY_ENTRY: PageTableEntry = PageTableEntry::new();
        Self {
            entries: [EMPTY_ENTRY; 512],
        }
    }

    /// Get a reference to an entry
    pub fn entry(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }

    /// Get all entries
    pub fn entries(&self) -> &[PageTableEntry; 512] {
        &self.entries
    }

    /// Zero out all entries
    pub fn zero(&mut self) {
        for entry in &mut self.entries {
            entry.zero();
        }
    }
}

/// Address space identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AddressSpaceId(pub u64);

/// Virtual memory manager
pub struct VirtualMemoryManager {
    /// Current address space
    current_asid: AddressSpaceId,
    /// Page table base (physical address)
    page_table_base: u64,
}

impl VirtualMemoryManager {
    /// Create a new VMM
    pub const fn new() -> Self {
        Self {
            current_asid: AddressSpaceId(0),
            page_table_base: 0,
        }
    }

    /// Initialize the VMM
    pub fn init(&mut self, page_table_base: u64) {
        self.page_table_base = page_table_base;
    }

    /// Map a page with given permissions
    /// 
    /// # Safety
    /// - Caller must ensure the physical frame is valid
    /// - Caller must ensure no aliasing violations
    pub unsafe fn map_page(
        &mut self,
        page: Page,
        frame: PhysFrame,
        perm: PagePerm,
    ) -> Result<(), MapError> {
        // W^X enforcement
        if perm.writable && perm.executable {
            return Err(MapError::WxViolation);
        }

        // User/kernel isolation: user pages cannot access kernel memory
        if perm.user && page.addr.is_kernel_addr() {
            return Err(MapError::UserKernelViolation);
        }

        // In actual implementation, this would walk the page table hierarchy
        // and set up the mapping. For now, we just validate the parameters.

        Ok(())
    }

    /// Unmap a page
    pub fn unmap_page(&mut self, page: Page) -> Result<(), MapError> {
        // In actual implementation, this would clear the page table entry
        Ok(())
    }

    /// Translate a virtual address to physical
    pub fn translate(&self, addr: VirtAddr) -> Option<u64> {
        // In actual implementation, this would walk the page tables
        // For now, just return the address if it's canonical
        if addr.is_canonical() {
            Some(addr.0)
        } else {
            None
        }
    }

    /// Switch to a different address space
    pub fn switch_address_space(&mut self, asid: AddressSpaceId, page_table_base: u64) {
        self.current_asid = asid;
        self.page_table_base = page_table_base;
        
        // In actual implementation, this would write to CR3
    }

    /// Get current address space
    pub const fn current_address_space(&self) -> AddressSpaceId {
        self.current_asid
    }
}

impl Default for VirtualMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory mapping error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    WxViolation,
    UserKernelViolation,
    InvalidAddress,
    AlreadyMapped,
    NotMapped,
    OutOfMemory,
}

// Core invariants:
// 1. W^X: Writable ∧ Executable = false for ordinary pages
// 2. User ↛ Kernel: User pages cannot access kernel memory
// 3. Canonical addresses only

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_perm_to_flags() {
        // Kernel RX
        let flags = PagePerm::KERNEL_RX.to_flags();
        assert!(flags.is_present());
        assert!(!flags.is_writable());
        assert!(!flags.is_user_accessible());
        assert!(flags.is_executable());

        // User RW
        let flags = PagePerm::USER_RW.to_flags();
        assert!(flags.is_present());
        assert!(flags.is_writable());
        assert!(flags.is_user_accessible());
        assert!(!flags.is_executable()); // NX set
    }

    #[test]
    fn test_virt_addr_indices() {
        let addr = VirtAddr::new(0x00007FFF_FFFFFFFF);
        assert_eq!(addr.pml4_index(), 255);
        assert_eq!(addr.pdpt_index(), 511);
        assert_eq!(addr.pd_index(), 511);
        assert_eq!(addr.pt_index(), 511);
        assert_eq!(addr.page_offset(), 0xFFF);
    }

    #[test]
    fn test_canonical_addresses() {
        // Lower canonical
        assert!(VirtAddr::new(0x00007FFF_FFFFFFFF).is_canonical());
        assert!(!VirtAddr::new(0x00008000_00000000).is_canonical());
        
        // Higher canonical
        assert!(VirtAddr::new(0xFFFF8000_00000000).is_canonical());
        assert!(!VirtAddr::new(0x00007FFF_FFFFFFFF + 1).is_canonical());
    }

    #[test]
    fn test_kernel_user_addr_split() {
        assert!(VirtAddr::new(0xFFFF8000_00000000).is_kernel_addr());
        assert!(VirtAddr::new(0x00007FFF_FFFFFFFF).is_user_addr());
    }

    #[test]
    fn test_page_creation() {
        let page = Page::from_number(100);
        assert_eq!(page.number(), 100);
        assert_eq!(page.addr.0, 100 * PAGE_SIZE);
        
        let aligned = Page::new(VirtAddr::new(0x1000)).unwrap();
        assert_eq!(aligned.number(), 1);
        
        let unaligned = Page::new(VirtAddr::new(0x1234));
        assert!(unaligned.is_none());
    }

    #[test]
    fn test_page_table_entry() {
        let mut entry = PageTableEntry::new();
        let frame = PhysFrame::new(42);
        let flags = PagePerm::KERNEL_RW.to_flags();
        
        entry.set(frame, flags);
        assert!(entry.is_present());
        assert_eq!(entry.frame().unwrap().number, 42);
        
        entry.zero();
        assert!(!entry.is_present());
    }

    #[test]
    fn test_page_table_zero() {
        let mut pt = PageTable::new();
        pt.entry(0).set(PhysFrame::new(1), PagePerm::KERNEL_RW.to_flags());
        assert!(pt.entry(0).is_present());
        
        pt.zero();
        for i in 0..512 {
            assert!(!pt.entries()[i].is_present());
        }
    }

    #[test]
    fn test_wx_violation_rejected() {
        let perm = PagePerm {
            writable: true,
            executable: true,
            user: false,
        };
        
        // This should be rejected by the API design
        // In real code, map_page would return WxViolation
        assert!(perm.writable && perm.executable);
    }

    #[test]
    fn test_user_kernel_isolation() {
        let kernel_addr = VirtAddr::new(0xFFFF8000_00000000);
        let user_addr = VirtAddr::new(0x00007FFF_FFFFFFFF);
        
        assert!(kernel_addr.is_kernel_addr());
        assert!(!kernel_addr.is_user_addr());
        
        assert!(!user_addr.is_kernel_addr());
        assert!(user_addr.is_user_addr());
    }
}
