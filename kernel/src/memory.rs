use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use bootloader_api::info::{MemoryRegion, MemoryRegionKind};

const FRAME_SIZE: u64 = 4096;
const MAX_USABLE_REGIONS: usize = 32;
const MAX_RECLAIMED_FRAMES: usize = 256;

static TOTAL_BYTES: AtomicU64 = AtomicU64::new(0);
static USABLE_BYTES: AtomicU64 = AtomicU64::new(0);
static REGION_COUNT: AtomicUsize = AtomicUsize::new(0);
static USABLE_REGION_COUNT: AtomicUsize = AtomicUsize::new(0);

static ALLOCATED_FRAMES: AtomicU64 = AtomicU64::new(0);
static ALLOCATOR_INITIALIZED: AtomicUsize = AtomicUsize::new(0);

static REGION_STARTS: [AtomicU64; MAX_USABLE_REGIONS] =
    [const { AtomicU64::new(0) }; MAX_USABLE_REGIONS];

static REGION_ENDS: [AtomicU64; MAX_USABLE_REGIONS] =
    [const { AtomicU64::new(0) }; MAX_USABLE_REGIONS];

static REGION_CURSORS: [AtomicU64; MAX_USABLE_REGIONS] =
    [const { AtomicU64::new(0) }; MAX_USABLE_REGIONS];

static RECLAIMED_FRAMES: [AtomicU64; MAX_RECLAIMED_FRAMES] =
    [const { AtomicU64::new(0) }; MAX_RECLAIMED_FRAMES];

static RECLAIMED_COUNT: AtomicUsize = AtomicUsize::new(0);

static ALLOCATOR_LOCK: AtomicUsize = AtomicUsize::new(0);

/// Align an address upward to the next 4 KiB frame boundary.
#[inline]
fn align_up(address: u64) -> u64 {
    address
        .saturating_add(FRAME_SIZE - 1)
        .saturating_div(FRAME_SIZE)
        .saturating_mul(FRAME_SIZE)
}

/// Initialize the physical-memory inventory and frame allocator.
///
/// Only regions explicitly marked `Usable` by the bootloader are entered
/// into the allocator.
pub fn init(regions: &[MemoryRegion]) -> usize {
    let mut total_bytes = 0u64;
    let mut usable_bytes = 0u64;
    let mut usable_regions = 0usize;

    for region in regions {
        let bytes = region.end.saturating_sub(region.start);

        total_bytes = total_bytes.saturating_add(bytes);

        if matches!(region.kind, MemoryRegionKind::Usable) {
            usable_bytes = usable_bytes.saturating_add(bytes);

            if usable_regions < MAX_USABLE_REGIONS {
                let start = align_up(region.start);
                let end = region.end / FRAME_SIZE * FRAME_SIZE;

                if start < end {
                    REGION_STARTS[usable_regions].store(start, Ordering::Relaxed);
                    REGION_ENDS[usable_regions].store(end, Ordering::Relaxed);
                    REGION_CURSORS[usable_regions].store(start, Ordering::Relaxed);

                    usable_regions += 1;
                }
            }
        }
    }

    TOTAL_BYTES.store(total_bytes, Ordering::Relaxed);
    USABLE_BYTES.store(usable_bytes, Ordering::Relaxed);
    REGION_COUNT.store(regions.len(), Ordering::Relaxed);
    USABLE_REGION_COUNT.store(usable_regions, Ordering::Relaxed);

    ALLOCATED_FRAMES.store(0, Ordering::Relaxed);
    RECLAIMED_COUNT.store(0, Ordering::Relaxed);

    let mut i = 0usize;

    while i < MAX_RECLAIMED_FRAMES {
        RECLAIMED_FRAMES[i].store(0, Ordering::Relaxed);
        i += 1;
    }

    ALLOCATOR_INITIALIZED.store(1, Ordering::Release);

    usable_regions
}

/// Allocate one physical 4 KiB frame.
///
/// Reclaimed frames are reused before the allocator advances into new
/// physical memory.
pub fn allocate_frame() -> Option<u64> {
    if ALLOCATOR_INITIALIZED.load(Ordering::Acquire) == 0 {
        return None;
    }

    lock();

    let reclaimed = pop_reclaimed_frame_locked();

    if let Some(address) = reclaimed {
        ALLOCATED_FRAMES.fetch_add(1, Ordering::Relaxed);
        unlock();
        return Some(address);
    }

    let result = allocate_frame_locked();

    if result.is_some() {
        ALLOCATED_FRAMES.fetch_add(1, Ordering::Relaxed);
    }

    unlock();

    result
}

fn allocate_frame_locked() -> Option<u64> {
    let region_count = usable_region_count() as usize;

    let mut index = 0usize;

    while index < region_count {
        let cursor = REGION_CURSORS[index].load(Ordering::Relaxed);
        let end = REGION_ENDS[index].load(Ordering::Relaxed);

        if cursor < end {
            let next = cursor.saturating_add(FRAME_SIZE);

            if next <= end {
                REGION_CURSORS[index].store(next, Ordering::Relaxed);
                return Some(cursor);
            }
        }

        index += 1;
    }

    None
}

/// Release a previously allocated physical frame.
///
/// The frame is placed into a bounded reclaim stack and can subsequently
/// be returned by `allocate_frame()`.
///
/// Returns `true` when the frame was accepted for reclamation.
pub fn free_frame(address: u64) -> bool {
    if ALLOCATOR_INITIALIZED.load(Ordering::Acquire) == 0 {
        return false;
    }

    if address % FRAME_SIZE != 0 {
        return false;
    }

    lock();

    if !is_usable_address_locked(address) {
        unlock();
        return false;
    }

    if is_reclaimed_locked(address) {
        unlock();
        return false;
    }

    let reclaimed_count = RECLAIMED_COUNT.load(Ordering::Relaxed);

    if reclaimed_count >= MAX_RECLAIMED_FRAMES {
        unlock();
        return false;
    }

    let free_index = reclaimed_count;
    RECLAIMED_FRAMES[free_index].store(address, Ordering::Relaxed);
    RECLAIMED_COUNT.store(reclaimed_count + 1, Ordering::Release);

    let allocated = ALLOCATED_FRAMES.load(Ordering::Relaxed);

    if allocated > 0 {
        ALLOCATED_FRAMES.store(allocated - 1, Ordering::Relaxed);
    }

    unlock();

    true
}

/// Release the most recently allocated/reclaimed frame.
///
/// This is primarily useful for shell-level testing of the allocator.
/// It does not attempt to reverse the bump cursor.
pub fn free_last_frame() -> Option<u64> {
    if ALLOCATOR_INITIALIZED.load(Ordering::Acquire) == 0 {
        return None;
    }

    lock();

    let address = pop_reclaimed_frame_locked();

    if let Some(address) = address {
        let allocated = ALLOCATED_FRAMES.load(Ordering::Relaxed);

        if allocated > 0 {
            ALLOCATED_FRAMES.store(allocated - 1, Ordering::Relaxed);
        }

        unlock();

        return Some(address);
    }

    unlock();

    None
}

/// Exercise the physical frame allocator.
///
/// The test:
/// 1. Allocates several frames.
/// 2. Verifies that each allocation is aligned and usable.
/// 3. Releases every frame.
/// 4. Allocates again and verifies that the most recently released frame
///    is actually reused.
/// 5. Releases the verification frame.
///
/// Returns `true` only when every step succeeds.
pub fn self_test() -> bool {
    if ALLOCATOR_INITIALIZED.load(Ordering::Acquire) == 0 {
        return false;
    }

    const TEST_FRAMES: usize = 8;

    let mut frames = [0u64; TEST_FRAMES];
    let mut allocated = 0usize;

    // Phase 1: allocate test frames.
    while allocated < TEST_FRAMES {
        match allocate_frame() {
            Some(address) => {
                if address % FRAME_SIZE != 0 {
                    let mut cleanup = 0usize;

                    while cleanup < allocated {
                        free_frame(frames[cleanup]);
                        cleanup += 1;
                    }

                    return false;
                }

                lock();
                let usable = is_usable_address_locked(address);
                unlock();

                if !usable {
                    let mut cleanup = 0usize;

                    while cleanup < allocated {
                        free_frame(frames[cleanup]);
                        cleanup += 1;
                    }

                    free_frame(address);
                    return false;
                }

                frames[allocated] = address;
                allocated += 1;
            }

            None => {
                let mut cleanup = 0usize;

                while cleanup < allocated {
                    free_frame(frames[cleanup]);
                    cleanup += 1;
                }

                return false;
            }
        }
    }

    // Phase 2: release every test frame.
    let mut index = 0usize;

    while index < TEST_FRAMES {
        if !free_frame(frames[index]) {
            let mut cleanup = index + 1;

            while cleanup < TEST_FRAMES {
                free_frame(frames[cleanup]);
                cleanup += 1;
            }

            return false;
        }

        index += 1;
    }

    // The reclaim stack is LIFO, so the last released frame must be
    // returned first. This gives us a concrete reuse verification.
    let reused = match allocate_frame() {
        Some(address) => address,
        None => return false,
    };

    if reused != frames[TEST_FRAMES - 1] {
        free_frame(reused);
        return false;
    }

    // Release the verification allocation.
    if !free_frame(reused) {
        return false;
    }

    true
}

fn pop_reclaimed_frame_locked() -> Option<u64> {
    let count = RECLAIMED_COUNT.load(Ordering::Acquire);

    if count == 0 {
        return None;
    }

    let new_count = count - 1;
    let address = RECLAIMED_FRAMES[new_count].load(Ordering::Relaxed);

    RECLAIMED_FRAMES[new_count].store(0, Ordering::Relaxed);
    RECLAIMED_COUNT.store(new_count, Ordering::Release);

    Some(address)
}

fn is_reclaimed_locked(address: u64) -> bool {
    let count = RECLAIMED_COUNT.load(Ordering::Acquire);

    let mut index = 0usize;

    while index < count {
        if RECLAIMED_FRAMES[index].load(Ordering::Relaxed) == address {
            return true;
        }

        index += 1;
    }

    false
}

fn is_usable_address_locked(address: u64) -> bool {
    let region_count = usable_region_count() as usize;

    let mut index = 0usize;

    while index < region_count {
        let start = REGION_STARTS[index].load(Ordering::Relaxed);
        let end = REGION_ENDS[index].load(Ordering::Relaxed);

        if address >= start && address < end {
            return true;
        }

        index += 1;
    }

    false
}

/// Number of currently allocated physical frames.
#[inline]
pub fn allocated_frames() -> u64 {
    ALLOCATED_FRAMES.load(Ordering::Relaxed)
}

/// Number of currently allocated bytes.
#[inline]
pub fn allocated_bytes() -> u64 {
    allocated_frames().saturating_mul(FRAME_SIZE)
}

/// Number of frames currently waiting to be reused.
#[inline]
pub fn reclaimed_frames() -> u64 {
    RECLAIMED_COUNT.load(Ordering::Acquire) as u64
}

/// Number of bytes currently waiting to be reused.
#[inline]
pub fn reclaimed_bytes() -> u64 {
    reclaimed_frames().saturating_mul(FRAME_SIZE)
}

/// Whether the physical frame allocator has been initialized.
#[inline]
pub fn allocator_initialized() -> bool {
    ALLOCATOR_INITIALIZED.load(Ordering::Acquire) != 0
}

/// Total physical memory represented by the bootloader map.
#[inline]
pub fn total_bytes() -> u64 {
    TOTAL_BYTES.load(Ordering::Relaxed)
}

/// Physical memory marked usable by the bootloader.
#[inline]
pub fn usable_bytes() -> u64 {
    USABLE_BYTES.load(Ordering::Relaxed)
}

/// Physical memory represented by non-usable regions.
#[inline]
pub fn reserved_bytes() -> u64 {
    total_bytes().saturating_sub(usable_bytes())
}

/// Total number of bootloader memory regions.
#[inline]
pub fn region_count() -> u64 {
    REGION_COUNT.load(Ordering::Relaxed) as u64
}

/// Number of usable regions accepted by the allocator.
#[inline]
pub fn usable_region_count() -> u64 {
    USABLE_REGION_COUNT.load(Ordering::Relaxed) as u64
}

#[inline]
pub fn total_mib() -> u64 {
    total_bytes() / (1024 * 1024)
}

#[inline]
pub fn usable_mib() -> u64 {
    usable_bytes() / (1024 * 1024)
}

#[inline]
pub fn reserved_mib() -> u64 {
    reserved_bytes() / (1024 * 1024)
}

#[inline]
pub fn allocated_mib() -> u64 {
    allocated_bytes() / (1024 * 1024)
}

#[inline]
pub fn reclaimed_mib() -> u64 {
    reclaimed_bytes() / (1024 * 1024)
}

#[inline]
fn lock() {
    while ALLOCATOR_LOCK
        .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
}

#[inline]
fn unlock() {
    ALLOCATOR_LOCK.store(0, Ordering::Release);
}
