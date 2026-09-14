use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::instructions::port::Port;

const PIT_FREQUENCY: u32 = 1_193_182;
pub const TIMER_FREQUENCY: u32 = 100;

static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    let divisor = PIT_FREQUENCY / TIMER_FREQUENCY;

    let mut command = Port::new(0x43);
    let mut channel0 = Port::new(0x40);

    unsafe {
        command.write(0x36u8);
        channel0.write((divisor & 0xFF) as u8);
        channel0.write((divisor >> 8) as u8);
    }
}

#[inline]
pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

#[inline]
pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}
