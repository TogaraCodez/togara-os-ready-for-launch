//! PIT Timer
//! 
//! Part 8 of TOGARA OS Platform Roadmap
//! Classification: RUNTIME
//! 
//! Configures the Programmable Interval Timer (PIT) for system ticks.

/// PIT I/O ports
const PIT_CHANNEL0: u16 = 0x40;
const PIT_COMMAND: u16 = 0x43;

/// Default frequency: 100 Hz
pub const DEFAULT_FREQUENCY: u32 = 100;

/// PIT input frequency: 1.193182 MHz
const PIT_INPUT_FREQUENCY: u32 = 1193182;

/// Initialize the PIT timer
/// 
/// # Safety
/// - This modifies hardware state
pub unsafe fn init_timer(frequency: u32) {
    let divisor = PIT_INPUT_FREQUENCY / frequency;
    
    // Send command byte: channel 0, lobyte/hibyte, square wave
    core::arch::asm!(
        "outb %al, %dx",
        in("al") 0x36u8,
        in("dx") PIT_COMMAND,
    );
    
    // Send divisor
    let low = (divisor & 0xFF) as u8;
    let high = ((divisor >> 8) & 0xFF) as u8;
    
    core::arch::asm!(
        "outb %al, %dx",
        in("al") low,
        in("dx") PIT_CHANNEL0,
    );
    
    core::arch::asm!(
        "outb %al, %dx",
        in("al") high,
        in("dx") PIT_CHANNEL0,
    );
}

/// Get tick period in milliseconds
pub const fn tick_period_ms(frequency: u32) -> u32 {
    1000 / frequency
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_period() {
        assert_eq!(tick_period_ms(100), 10);
        assert_eq!(tick_period_ms(1000), 1);
    }
}
