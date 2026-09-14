/// GNOSIS runtime context.
///
/// This module provides a read-only view over existing kernel state.
/// It intentionally does not maintain duplicate copies of system
/// counters; the system state remains the source of truth.

pub const ACTIVE: bool = true;

#[inline]
pub fn is_active() -> bool {
    ACTIVE
}

/// Current GNOSIS context generation.
///
/// Keyboard events are currently the primary event source available to
/// the kernel, so the event count acts as the context generation.
#[inline]
pub fn generation() -> u64 {
    crate::system::state::keyboard_events()
}

/// Number of keyboard events observed by the system.
#[inline]
pub fn keyboard_events() -> u64 {
    crate::system::state::keyboard_events()
}

/// Current kernel timer ticks.
#[inline]
pub fn uptime_ticks() -> u64 {
    crate::system::state::uptime_ticks()
}

/// Current kernel uptime in seconds.
#[inline]
pub fn uptime_seconds() -> u64 {
    crate::system::state::uptime_seconds()
}

/// Current kernel uptime milliseconds within the current second.
#[inline]
pub fn uptime_millis() -> u64 {
    crate::system::state::uptime_millis()
}
