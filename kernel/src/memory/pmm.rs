//! Physical Memory Manager (PMM)
//! 
//! Part 3 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Manages physical memory frames with bitmap allocation.
//! Page size: 4096 bytes (2^12)

/// Page size in bytes
pub const PAGE_SIZE: u64 = 4096;

/// Number of frames that can be tracked in a 64-bit word
const FRAMES_PER_WORD: usize = 64;

/// Physical frame number
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct PhysFrame {
    pub number: u64,
}

impl PhysFrame {
    /// Create a new PhysFrame from frame number
    pub const fn new(number: u64) -> Self {
        Self { number }
    }

    /// Get the start address of this frame
    pub const fn start_address(self) -> u64 {
        self.number * PAGE_SIZE
    }

    /// Get the end address of this frame (exclusive)
    pub const fn end_address(self) -> u64 {
        (self.number + 1) * PAGE_SIZE
    }

    /// Create PhysFrame from physical address
    pub const fn from_address(address: u64) -> Option<Self> {
        // Address must be page-aligned
        if address % PAGE_SIZE != 0 {
            return None;
        }
        Some(Self { number: address / PAGE_SIZE })
    }

    /// Check if this frame is before another
    pub const fn is_before(self, other: Self) -> bool {
        self.number < other.number
    }
}

/// Allocation error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocError {
    OutOfMemory,
    FrameAlreadyAllocated,
    FrameNotAllocated,
    InvalidFrame,
}

/// Frame allocator trait
pub trait FrameAllocator {
    /// Allocate a frame
    fn alloc(&mut self) -> Option<PhysFrame>;

    /// Deallocate a frame
    fn dealloc(&mut self, frame: PhysFrame) -> Result<(), AllocError>;

    /// Mark a frame as reserved (cannot be allocated or freed)
    fn mark_reserved(&mut self, frame: PhysFrame);
}

/// Bitmap-based frame allocator
pub struct BitmapFrameAllocator<const WORDS: usize> {
    /// Bitmap: 0 = free, 1 = allocated
    bitmap: [u64; WORDS],
    /// Reserved frames bitmap
    reserved: [u64; WORDS],
    /// Total number of frames
    total_frames: usize,
    /// Number of allocated frames
    allocated_count: usize,
}

impl<const WORDS: usize> BitmapFrameAllocator<WORDS> {
    /// Create a new bitmap allocator
    pub const fn new(total_frames: usize) -> Self {
        assert!(total_frames <= WORDS * FRAMES_PER_WORD, "Too many frames for bitmap");
        Self {
            bitmap: [0; WORDS],
            reserved: [0; WORDS],
            total_frames,
            allocated_count: 0,
        }
    }

    /// Initialize from memory map
    pub fn from_memory_map(
        memory_regions: &[crate::boot::MemoryRegion],
        reserved_regions: &[crate::boot::MemoryRegion],
    ) -> Result<Self, AllocError> {
        // Count total usable frames
        let mut total_frames = 0usize;
        for region in memory_regions {
            if region.usable {
                let frames = (region.len / PAGE_SIZE) as usize;
                total_frames = total_frames.saturating_add(frames);
            }
        }

        // Calculate words needed
        let words = (total_frames + FRAMES_PER_WORD - 1) / FRAMES_PER_WORD;
        
        // Create allocator
        let mut allocator = Self::new(total_frames.min(words * FRAMES_PER_WORD));

        // Mark reserved regions
        for region in reserved_regions {
            let start_frame = PhysFrame::from_address(region.start)
                .ok_or(AllocError::InvalidFrame)?;
            let end_frame = PhysFrame::from_address(region.start + region.len)
                .unwrap_or(PhysFrame::new((region.start + region.len) / PAGE_SIZE));

            let mut frame = start_frame;
            while frame.number < end_frame.number && (frame.number as usize) < allocator.total_frames {
                allocator.mark_reserved(frame);
                frame = PhysFrame::new(frame.number + 1);
            }
        }

        Ok(allocator)
    }

    /// Find the first free frame
    fn find_free_frame(&self) -> Option<PhysFrame> {
        for (word_idx, &word) in self.bitmap.iter().enumerate() {
            // Check if any bit is free (0) and not reserved
            let reserved_word = self.reserved[word_idx];
            let available = !word & !reserved_word;
            
            if available != 0 {
                // Find first zero bit
                let bit_pos = available.trailing_zeros() as usize;
                let frame_number = (word_idx * FRAMES_PER_WORD + bit_pos) as u64;
                
                if (frame_number as usize) < self.total_frames {
                    return Some(PhysFrame::new(frame_number));
                }
            }
        }
        None
    }

    /// Set a bit in the bitmap
    fn set_bit(&mut self, frame: PhysFrame, bitmap: &mut [u64; WORDS]) {
        let word_idx = (frame.number as usize) / FRAMES_PER_WORD;
        let bit_pos = (frame.number as usize) % FRAMES_PER_WORD;
        bitmap[word_idx] |= 1 << bit_pos;
    }

    /// Clear a bit in the bitmap
    fn clear_bit(&mut self, frame: PhysFrame, bitmap: &mut [u64; WORDS]) {
        let word_idx = (frame.number as usize) / FRAMES_PER_WORD;
        let bit_pos = (frame.number as usize) % FRAMES_PER_WORD;
        bitmap[word_idx] &= !(1 << bit_pos);
    }

    /// Check if a bit is set
    fn is_bit_set(&self, frame: PhysFrame, bitmap: &[u64; WORDS]) -> bool {
        let word_idx = (frame.number as usize) / FRAMES_PER_WORD;
        let bit_pos = (frame.number as usize) % FRAMES_PER_WORD;
        (bitmap[word_idx] & (1 << bit_pos)) != 0
    }

    /// Get allocation statistics
    pub fn stats(&self) -> AllocStats {
        AllocStats {
            total_frames: self.total_frames,
            allocated_frames: self.allocated_count,
            free_frames: self.total_frames - self.allocated_count,
            fragmentation: self.calculate_fragmentation(),
        }
    }

    /// Calculate fragmentation metric
    fn calculate_fragmentation(&self) -> f64 {
        // Find largest free block
        let mut largest_free = 0usize;
        let mut current_free = 0usize;

        for frame_num in 0..self.total_frames {
            let frame = PhysFrame::new(frame_num as u64);
            if self.is_bit_set(frame, &self.bitmap) || self.is_bit_set(frame, &self.reserved) {
                largest_free = largest_free.max(current_free);
                current_free = 0;
            } else {
                current_free += 1;
            }
        }
        largest_free = largest_free.max(current_free);

        let total_free = self.total_frames - self.allocated_count;
        if total_free == 0 {
            0.0
        } else {
            1.0 - (largest_free as f64 / total_free as f64)
        }
    }
}

/// Allocation statistics
#[derive(Debug, Clone)]
pub struct AllocStats {
    pub total_frames: usize,
    pub allocated_frames: usize,
    pub free_frames: usize,
    pub fragmentation: f64,
}

impl<const WORDS: usize> FrameAllocator for BitmapFrameAllocator<WORDS> {
    fn alloc(&mut self) -> Option<PhysFrame> {
        let frame = self.find_free_frame()?;
        
        // Mark as allocated
        self.set_bit(frame, &mut self.bitmap);
        self.allocated_count += 1;
        
        Some(frame)
    }

    fn dealloc(&mut self, frame: PhysFrame) -> Result<(), AllocError> {
        // Check bounds
        if (frame.number as usize) >= self.total_frames {
            return Err(AllocError::InvalidFrame);
        }

        // Check if allocated
        if !self.is_bit_set(frame, &self.bitmap) {
            return Err(AllocError::FrameNotAllocated);
        }

        // Check if reserved
        if self.is_bit_set(frame, &self.reserved) {
            return Err(AllocError::FrameAlreadyAllocated);
        }

        // Mark as free
        self.clear_bit(frame, &mut self.bitmap);
        self.allocated_count -= 1;
        
        Ok(())
    }

    fn mark_reserved(&mut self, frame: PhysFrame) {
        if (frame.number as usize) < self.total_frames {
            self.set_bit(frame, &mut self.reserved);
        }
    }
}

// Core invariants (enforced by implementation):
// 1. Allocated ∩ Free = ∅ (a frame cannot be both allocated and free)
// 2. Reserved ∩ Free = ∅ (a reserved frame cannot be allocated)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_address_conversion() {
        let frame = PhysFrame::new(100);
        assert_eq!(frame.start_address(), 100 * PAGE_SIZE);
        assert_eq!(frame.end_address(), 101 * PAGE_SIZE);
        
        let addr = 200 * PAGE_SIZE;
        let frame_from_addr = PhysFrame::from_address(addr).unwrap();
        assert_eq!(frame_from_addr.number, 200);
    }

    #[test]
    fn test_unaligned_address_rejected() {
        assert!(PhysFrame::from_address(0x1234).is_none());
    }

    #[test]
    fn test_allocations_are_unique() {
        const WORDS: usize = 16;
        let mut allocator = BitmapFrameAllocator::<WORDS>::new(1000);
        
        let mut frames = Vec::new();
        for _ in 0..100 {
            if let Some(frame) = allocator.alloc() {
                frames.push(frame);
            }
        }
        
        // All frames should be unique
        frames.sort_by_key(|f| f.number);
        for i in 1..frames.len() {
            assert_ne!(frames[i].number, frames[i-1].number);
        }
    }

    #[test]
    fn test_alloc_dealloc_cycle() {
        const WORDS: usize = 16;
        let mut allocator = BitmapFrameAllocator::<WORDS>::new(100);
        
        let frame = allocator.alloc().unwrap();
        assert!(allocator.dealloc(frame).is_ok());
        
        // Should be able to allocate again
        let frame2 = allocator.alloc().unwrap();
        assert_eq!(frame.number, frame2.number);
    }

    #[test]
    fn test_reserved_frame_cannot_be_allocated() {
        const WORDS: usize = 16;
        let mut allocator = BitmapFrameAllocator::<WORDS>::new(100);
        
        let frame = PhysFrame::new(5);
        allocator.mark_reserved(frame);
        
        // Allocate until we get frame 5 or run out
        let mut found = false;
        while let Some(allocated) = allocator.alloc() {
            if allocated.number == 5 {
                found = true;
                break;
            }
        }
        
        // Frame 5 should never be allocated
        assert!(!found);
    }

    #[test]
    fn test_double_dealloc_error() {
        const WORDS: usize = 16;
        let mut allocator = BitmapFrameAllocator::<WORDS>::new(100);
        
        let frame = allocator.alloc().unwrap();
        assert!(allocator.dealloc(frame).is_ok());
        assert_eq!(allocator.dealloc(frame), Err(AllocError::FrameNotAllocated));
    }

    #[test]
    fn test_fragmentation_calculation() {
        const WORDS: usize = 16;
        let mut allocator = BitmapFrameAllocator::<WORDS>::new(100);
        
        // Initially no fragmentation
        let stats = allocator.stats();
        assert_eq!(stats.allocated_frames, 0);
        
        // Allocate some frames
        for _ in 0..10 {
            allocator.alloc();
        }
        
        let stats = allocator.stats();
        assert_eq!(stats.allocated_frames, 10);
        assert!(stats.fragmentation >= 0.0 && stats.fragmentation <= 1.0);
    }

    #[test]
    fn test_invariant_allocated_disjoint_free() {
        const WORDS: usize = 16;
        let mut allocator = BitmapFrameAllocator::<WORDS>::new(100);
        
        // Allocate 50 frames
        let mut allocated = Vec::new();
        for _ in 0..50 {
            if let Some(frame) = allocator.alloc() {
                allocated.push(frame);
            }
        }
        
        // Verify no allocated frame appears in free list
        for &frame in &allocated {
            // Try to allocate - should not get the same frame
            while let Some(new_frame) = allocator.alloc() {
                assert_ne!(new_frame.number, frame.number);
            }
        }
    }

    #[test]
    fn test_stress_random_alloc_dealloc() {
        const WORDS: usize = 256;
        let mut allocator = BitmapFrameAllocator::<WORDS>::new(10000);
        
        let mut allocated = Vec::new();
        
        // 10000 random operations
        for i in 0..10000 {
            if i % 3 == 0 && !allocated.is_empty() {
                // Dealloc
                let idx = (i * 7) % allocated.len();
                let frame = allocated.swap_remove(idx);
                assert!(allocator.dealloc(frame).is_ok());
            } else {
                // Alloc
                if let Some(frame) = allocator.alloc() {
                    allocated.push(frame);
                }
            }
        }
        
        // Verify all remaining allocated frames are valid
        for &frame in &allocated {
            assert!((frame.number as usize) < allocator.stats().total_frames);
        }
    }
}
