//! VGA Text Mode Driver
//! 
//! Priority 1: Boot & Output
//! Classification: RUNTIME
//! 
//! Provides text output to VGA buffer at 0xB8000.

use core::fmt::{self, Write};
use spin::Mutex;

/// VGA buffer address
const VGA_BUFFER: *mut u8 = 0xB8000 as *mut u8;

/// Number of columns
const COLS: usize = 80;

/// Number of rows
const ROWS: usize = 25;

/// VGA color codes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// VGA color code (foreground + background)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    /// Create new color code
    pub const fn new(foreground: Color, background: Color) -> Self {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// VGA character (ASCII + color)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VgaCharacter {
    character: u8,
    color_code: ColorCode,
}

impl VgaCharacter {
    pub const fn new(character: u8, color_code: ColorCode) -> Self {
        VgaCharacter { character, color_code }
    }
}

/// VGA text mode driver
pub struct VgaDriver {
    buffer: &'static mut [VgaCharacter; COLS * ROWS],
    cursor_x: usize,
    cursor_y: usize,
    color_code: ColorCode,
}

impl VgaDriver {
    /// Create a new VGA driver
    pub const fn new(color_code: ColorCode) -> Self {
        VgaDriver {
            buffer: unsafe { &mut *(VGA_BUFFER as *mut [VgaCharacter; COLS * ROWS]) },
            cursor_x: 0,
            cursor_y: 0,
            color_code,
        }
    }

    /// Clear the screen
    pub fn clear(&mut self) {
        let blank = VgaCharacter::new(b' ', self.color_code);
        for i in 0..COLS * ROWS {
            self.buffer[i] = blank;
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    /// Write a byte to VGA buffer
    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.cursor_x = 0;
                self.cursor_y += 1;
            }
            byte => {
                if self.cursor_x >= COLS {
                    self.cursor_x = 0;
                    self.cursor_y += 1;
                }

                if self.cursor_y >= ROWS {
                    self.scroll();
                    self.cursor_y = ROWS - 1;
                }

                let index = self.cursor_y * COLS + self.cursor_x;
                self.buffer[index] = VgaCharacter::new(byte, self.color_code);
                self.cursor_x += 1;
            }
        }
    }

    /// Scroll the screen up by one line
    fn scroll(&mut self) {
        for row in 0..(ROWS - 1) {
            for col in 0..COLS {
                let src_index = (row + 1) * COLS + col;
                let dst_index = row * COLS + col;
                self.buffer[dst_index] = self.buffer[src_index];
            }
        }

        // Clear last line
        for col in 0..COLS {
            let index = (ROWS - 1) * COLS + col;
            self.buffer[index] = VgaCharacter::new(b' ', self.color_code);
        }
    }

    /// Set cursor position
    pub fn set_cursor(&mut self, x: usize, y: usize) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    /// Enable cursor
    pub fn enable_cursor(&self) {
        // In real implementation, would program VGA controller
        // to show hardware cursor
    }
}

impl Write for VgaDriver {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

/// Global VGA driver instance
pub static VGA_WRITER: Mutex<VgaDriver> = Mutex::new(VgaDriver::new(ColorCode::new(Color::LightGray, Color::Black)));

/// Print to VGA (used by panic handler)
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    VGA_WRITER.lock().write_fmt(args).unwrap();
}

/// Print macro
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::vga::_print(format_args!($($arg)*));
    };
}

/// Println macro
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_code() {
        let cc = ColorCode::new(Color::White, Color::Black);
        assert_eq!(cc.0, 0x07);
    }

    #[test]
    fn test_vga_character() {
        let cc = ColorCode::new(Color::LightGray, Color::Black);
        let ch = VgaCharacter::new(b'A', cc);
        assert_eq!(ch.character, b'A');
        assert_eq!(ch.color_code.0, 0x07);
    }
}
