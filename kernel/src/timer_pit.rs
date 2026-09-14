//! PIT (Programmable Interval Timer) Driver
//! 
//! Priority 8: Interrupt Handling
//! Classification: RUNTIME
//! 
//! Configures PIT for periodic timer interrupts at IRQ0.

use crate::pic::{self, Pic};
use x86_64::instructions::port::Port;

/// PIT channel 0 data port
const PIT_CHANNEL0: u16 = 0x40;

/// PIT command port
const PIT_COMMAND: u16 = 0x43;

/// PIT input frequency: 1.193182 MHz
const PIT_INPUT_FREQUENCY: u32 = 1193182;

/// Default frequency: 100 Hz (10ms tick)
pub const DEFAULT_FREQUENCY: u32 = 100;

/// PIT timer driver
pub struct PitTimer {
    frequency: u32,
    ticks: u64,
}

impl PitTimer {
    /// Create a new PIT timer
    pub const fn new(frequency: u32) -> Self {
        Self {
            frequency,
            ticks: 0,
        }
    }

    /// Initialize the PIT timer
    /// 
    /// # Safety
    /// - Programs hardware timer
    /// - Must be called before enabling interrupts
    pub unsafe fn init(&mut self, pic: &mut Pic) {
        let divisor = PIT_INPUT_FREQUENCY / self.frequency;
        
        // Send command byte: channel 0, lobyte/hibyte, square wave
        Port::new(PIT_COMMAND).write(0x36);
        
        // Send divisor
        Port::new(PIT_CHANNEL0).write((divisor & 0xFF) as u8);
        Port::new(PIT_CHANNEL0).write(((divisor >> 8) & 0xFF) as u8);
        
        // Enable timer interrupt (IRQ0)
        pic.enable_irq(pic::IRQ_TIMER);
    }

    /// Handle timer interrupt
    pub fn handle_interrupt(&mut self) {
        self.ticks += 1;
        
        // In real implementation, would call scheduler here
    }

    /// Get current tick count
    pub const fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Get frequency
    pub const fn frequency(&self) -> u32 {
        self.frequency
    }

    /// Get milliseconds since boot
    pub const fn milliseconds(&self) -> u64 {
        self.ticks * (1000 / self.frequency as u64)
    }

    /// Sleep for specified milliseconds
    pub fn sleep_ms(&mut self, ms: u64) {
        let target = self.ticks + (ms / (1000 / self.frequency as u64));
        while self.ticks < target {
            core::hint::spin_loop();
        }
    }
}

/// Global PIT timer instance
pub static mut PIT: Option<PitTimer> = None;

/// Initialize PIT timer
/// 
/// # Safety
/// - Must be called during kernel initialization
pub unsafe fn init_pit(frequency: u32, pic: &mut Pic) {
    PIT = Some(PitTimer::new(frequency));
    if let Some(timer) = &mut PIT {
        timer.init(pic);
    }
}

/// Handle timer interrupt
pub fn handle_timer_interrupt() {
    unsafe {
        if let Some(timer) = &mut PIT {
            timer.handle_interrupt();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_creation() {
        let timer = PitTimer::new(100);
        assert_eq!(timer.frequency(), 100);
        assert_eq!(timer.ticks(), 0);
    }

    #[test]
    fn test_milliseconds_calculation() {
        let timer = PitTimer::new(100);
        // At 100Hz, each tick is 10ms
        // This would need actual ticks to test properly
    }
}
