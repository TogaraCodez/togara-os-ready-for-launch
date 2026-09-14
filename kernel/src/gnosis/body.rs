//! GNOSIS semantic body snapshot.
//!
//! REL-06 is intentionally observation-only. This module summarizes the
//! current embodiment of the running kernel by reading the authoritative
//! runtime state that already exists in the system. It never mutates canonical
//! state, never executes hardware actions, and never claims capabilities not
//! backed by existing kernel APIs.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodySnapshot {
    pub uptime_ticks: u64,
    pub uptime_seconds: u64,
    pub uptime_millis: u64,
    pub keyboard_events: u64,
    pub timer_frequency: u32,
    pub total_memory_bytes: u64,
    pub usable_memory_bytes: u64,
    pub reserved_memory_bytes: u64,
    pub allocated_memory_bytes: u64,
    pub reclaimed_memory_bytes: u64,
    pub messaging_count: usize,
}

#[inline]
pub fn snapshot() -> BodySnapshot {
    BodySnapshot {
        uptime_ticks: crate::system::state::uptime_ticks(),
        uptime_seconds: crate::system::state::uptime_seconds(),
        uptime_millis: crate::system::state::uptime_millis(),
        keyboard_events: crate::system::state::keyboard_events(),
        timer_frequency: crate::interrupts::timer::TIMER_FREQUENCY,
        total_memory_bytes: crate::memory::total_bytes(),
        usable_memory_bytes: crate::memory::usable_bytes(),
        reserved_memory_bytes: crate::memory::reserved_bytes(),
        allocated_memory_bytes: crate::memory::allocated_bytes(),
        reclaimed_memory_bytes: crate::memory::reclaimed_bytes(),
        messaging_count: crate::gnosis::messaging::count(),
    }
}

#[inline]
pub fn self_test() -> bool {
    let before_total = crate::memory::total_bytes();
    let before_usable = crate::memory::usable_bytes();
    let before_reserved = crate::memory::reserved_bytes();
    let before_allocated = crate::memory::allocated_bytes();
    let before_reclaimed = crate::memory::reclaimed_bytes();
    let before_messages = crate::gnosis::messaging::count();

    let snapshot_value = snapshot();

    if snapshot_value.timer_frequency == 0 {
        return false;
    }

    if snapshot_value.total_memory_bytes != before_total
        || snapshot_value.usable_memory_bytes != before_usable
        || snapshot_value.reserved_memory_bytes != before_reserved
        || snapshot_value.allocated_memory_bytes != before_allocated
        || snapshot_value.reclaimed_memory_bytes != before_reclaimed
        || snapshot_value.messaging_count != before_messages
    {
        return false;
    }

    if snapshot_value.usable_memory_bytes > snapshot_value.total_memory_bytes {
        return false;
    }

    if snapshot_value.reserved_memory_bytes
        != snapshot_value
            .total_memory_bytes
            .saturating_sub(snapshot_value.usable_memory_bytes)
    {
        return false;
    }

    let after_total = crate::memory::total_bytes();
    let after_usable = crate::memory::usable_bytes();
    let after_reserved = crate::memory::reserved_bytes();
    let after_allocated = crate::memory::allocated_bytes();
    let after_reclaimed = crate::memory::reclaimed_bytes();
    let after_messages = crate::gnosis::messaging::count();

    if after_total != before_total
        || after_usable != before_usable
        || after_reserved != before_reserved
        || after_allocated != before_allocated
        || after_reclaimed != before_reclaimed
        || after_messages != before_messages
    {
        return false;
    }

    let size = core::mem::size_of::<BodySnapshot>();
    if size == 0 || size > 256 {
        return false;
    }

    let repeated = snapshot();
    if repeated != snapshot_value {
        let current_total = crate::memory::total_bytes();
        let current_usable = crate::memory::usable_bytes();
        let current_reserved = crate::memory::reserved_bytes();
        let current_allocated = crate::memory::allocated_bytes();
        let current_reclaimed = crate::memory::reclaimed_bytes();
        let current_messages = crate::gnosis::messaging::count();

        if repeated.total_memory_bytes != current_total
            || repeated.usable_memory_bytes != current_usable
            || repeated.reserved_memory_bytes != current_reserved
            || repeated.allocated_memory_bytes != current_allocated
            || repeated.reclaimed_memory_bytes != current_reclaimed
            || repeated.messaging_count != current_messages
        {
            return false;
        }
    }

    true
}
