//! Serial Port Driver (COM1)
//! 
//! Priority 1: Boot & Output
//! Classification: RUNTIME
//! 
//! Provides debug output to serial port 0x3F8 (COM1).

use core::fmt::{self, Write};
use spin::Mutex;
use volatile::Volatile;

/// COM1 port address
const COM1_PORT: u16 = 0x3F8;

/// Serial port driver
pub struct SerialPort {
    data: Volatile<u8>,
    int_enable: Volatile<u8>,
    fifo_control: Volatile<u8>,
    line_control: Volatile<u8>,
    modem_control: Volatile<u8>,
    line_status: Volatile<u8>,
}

impl SerialPort {
    /// Create a new serial port instance
    pub const fn new(base_port: u16) -> Self {
        Self {
            data: Volatile::new(base_port as *mut u8),
            int_enable: Volatile::new((base_port + 1) as *mut u8),
            fifo_control: Volatile::new((base_port + 2) as *mut u8),
            line_control: Volatile::new((base_port + 3) as *mut u8),
            modem_control: Volatile::new((base_port + 4) as *mut u8),
            line_status: Volatile::new((base_port + 5) as *mut u8),
        }
    }

    /// Initialize the serial port
    pub fn init(&mut self) {
        // Disable interrupts
        self.int_enable.write(0x00);
        
        // Enable DLAB (set baud rate divisor)
        self.line_control.write(0x80);
        
        // Set divisor to 3 (38400 baud)
        self.data.write(0x03);
        self.int_enable.write(0x00);
        
        // 8 bits, no parity, one stop bit
        self.line_control.write(0x03);
        
        // Enable FIFO
        self.fifo_control.write(0xC7);
        
        // Enable IRQ, set RTS/DSR
        self.modem_control.write(0x0B);
        
        // Test serial chip (only if port is COM1)
        if self.line_status.read() == 0x60 {
            // Serial chip not present
        }
    }

    /// Check if transmit holding register is empty
    fn is_transmit_empty(&self) -> bool {
        (self.line_status.read() & 0x20) != 0
    }

    /// Write a byte to serial port
    fn write_byte(&mut self, data: u8) {
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }
        self.data.write(data);
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

/// Global serial port instance
pub static SERIAL: Mutex<SerialPort> = Mutex::new(SerialPort::new(COM1_PORT));

/// Initialize serial port
pub fn init_serial() {
    SERIAL.lock().init();
}

/// Print to serial port
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    SERIAL.lock().write_fmt(args).unwrap();
}

/// Serial print macro
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

/// Serial println macro
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serial_port_creation() {
        let port = SerialPort::new(COM1_PORT);
        // Just verify it compiles and can be created
    }
}
