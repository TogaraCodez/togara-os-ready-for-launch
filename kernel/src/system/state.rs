use core::sync::atomic::{AtomicU64, Ordering};

static KEYBOARD_EVENTS: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn record_keyboard_event() {
    KEYBOARD_EVENTS.fetch_add(1, Ordering::Relaxed);
}

#[inline]
pub fn keyboard_events() -> u64 {
    KEYBOARD_EVENTS.load(Ordering::Relaxed)
}

#[inline]
pub fn uptime_ticks() -> u64 {
    crate::interrupts::timer::ticks()
}

#[inline]
pub fn uptime_seconds() -> u64 {
    uptime_ticks() / crate::interrupts::timer::TIMER_FREQUENCY as u64
}

#[inline]
pub fn uptime_millis() -> u64 {
    let ticks = uptime_ticks();
    let frequency = crate::interrupts::timer::TIMER_FREQUENCY as u64;

    (ticks % frequency) * 1000 / frequency
}
