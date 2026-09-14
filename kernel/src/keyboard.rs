//! PS/2 Keyboard Driver
//! 
//! Priority 1: Boot & Output  
//! Classification: RUNTIME
//! 
//! Handles keyboard input from IRQ1.

use crate::pic::{self, Pic};
use spin::Mutex;
use x86_64::instructions::port::Port;

/// Keyboard data port
const KBD_DATA_PORT: u16 = 0x60;

/// Keyboard command port
const KBD_CMD_PORT: u16 = 0x64;

/// Keyboard buffer size
const BUFFER_SIZE: usize = 256;

/// Keyboard driver
pub struct KeyboardDriver {
    buffer: KeyboardBuffer,
}

impl KeyboardDriver {
    /// Create a new keyboard driver
    pub const fn new() -> Self {
        Self {
            buffer: KeyboardBuffer::new(),
        }
    }

    /// Initialize the keyboard
    /// 
    /// # Safety
    /// - Programs keyboard controller
    /// - Must be called before enabling interrupts
    pub unsafe fn init(&mut self, pic: &mut Pic) {
        // Enable keyboard interrupt (IRQ1)
        pic.enable_irq(pic::IRQ_KEYBOARD);
        
        // Set keyboard scancode set 2 (default)
        self.set_scancode_set(2);
    }

    /// Set scancode set
    fn set_scancode_set(&mut self, set: u8) {
        // Send command to set scancode set
        self.send_command(0xF0);
        self.send_command(set);
    }

    /// Send command to keyboard
    fn send_command(&mut self, cmd: u8) {
        // Wait for input buffer to be empty
        while (Port::new(KBD_CMD_PORT).read() & 0x02) != 0 {
            core::hint::spin_loop();
        }
        
        Port::new(KBD_DATA_PORT).write(cmd);
    }

    /// Handle keyboard interrupt
    pub fn handle_interrupt(&mut self) {
        // Read scancode
        let scancode = Port::new(KBD_DATA_PORT).read();
        
        // Process scancode
        if let Some(key) = self.decode_scancode(scancode) {
            self.buffer.push(key);
        }
    }

    /// Decode scancode to key
    fn decode_scancode(&self, scancode: u8) -> Option<Key> {
        // Simple US QWERTY scancode set 2 decoding
        // This is a minimal implementation
        match scancode {
            0x1C => Some(Key::A),
            0x32 => Some(Key::B),
            0x21 => Some(Key::C),
            0x23 => Some(Key::D),
            0x24 => Some(Key::E),
            0x2B => Some(Key::F),
            0x34 => Some(Key::G),
            0x33 => Some(Key::H),
            0x43 => Some(Key::I),
            0x3B => Some(Key::J),
            0x42 => Some(Key::K),
            0x4B => Some(Key::L),
            0x3A => Some(Key::M),
            0x31 => Some(Key::N),
            0x44 => Some(Key::O),
            0x4D => Some(Key::P),
            0x15 => Some(Key::Q),
            0x2D => Some(Key::R),
            0x1B => Some(Key::S),
            0x2C => Some(Key::T),
            0x3C => Some(Key::U),
            0x2A => Some(Key::V),
            0x1D => Some(Key::W),
            0x2B => Some(Key::X),
            0x35 => Some(Key::Y),
            0x1A => Some(Key::Z),
            0x29 => Some(Key::Space),
            0x5A => Some(Key::Enter),
            0x66 => Some(Key::Backspace),
            0x0D => Some(Key::Tab),
            0x76 => Some(Key::Escape),
            _ => None,
        }
    }

    /// Read a key from the buffer
    pub fn read_key(&mut self) -> Option<Key> {
        self.buffer.pop()
    }

    /// Check if buffer has keys
    pub fn has_keys(&self) -> bool {
        self.buffer.len() > 0
    }
}

impl Default for KeyboardDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Keyboard buffer (lock-free SPSC)
struct KeyboardBuffer {
    buffer: [Key; BUFFER_SIZE],
    read_pos: usize,
    write_pos: usize,
}

impl KeyboardBuffer {
    const fn new() -> Self {
        Self {
            buffer: [Key::None; BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
        }
    }

    fn push(&mut self, key: Key) {
        let next_write = (self.write_pos + 1) % BUFFER_SIZE;
        if next_write != self.read_pos {
            self.buffer[self.write_pos] = key;
            self.write_pos = next_write;
        }
    }

    fn pop(&mut self) -> Option<Key> {
        if self.read_pos == self.write_pos {
            return None;
        }
        
        let key = self.buffer[self.read_pos];
        self.read_pos = (self.read_pos + 1) % BUFFER_SIZE;
        Some(key)
    }

    const fn len(&self) -> usize {
        if self.write_pos >= self.read_pos {
            self.write_pos - self.read_pos
        } else {
            BUFFER_SIZE - self.read_pos + self.write_pos
        }
    }
}

/// Key representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    None,
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    Space,
    Enter,
    Backspace,
    Tab,
    Escape,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
}

impl Key {
    /// Convert key to character
    pub fn to_char(self) -> Option<char> {
        match self {
            Key::A => Some('a'),
            Key::B => Some('b'),
            Key::C => Some('c'),
            Key::D => Some('d'),
            Key::E => Some('e'),
            Key::F => Some('f'),
            Key::G => Some('g'),
            Key::H => Some('h'),
            Key::I => Some('i'),
            Key::J => Some('j'),
            Key::K => Some('k'),
            Key::L => Some('l'),
            Key::M => Some('m'),
            Key::N => Some('n'),
            Key::O => Some('o'),
            Key::P => Some('p'),
            Key::Q => Some('q'),
            Key::R => Some('r'),
            Key::S => Some('s'),
            Key::T => Some('t'),
            Key::U => Some('u'),
            Key::V => Some('v'),
            Key::W => Some('w'),
            Key::X => Some('x'),
            Key::Y => Some('y'),
            Key::Z => Some('z'),
            Key::Num0 => Some('0'),
            Key::Num1 => Some('1'),
            Key::Num2 => Some('2'),
            Key::Num3 => Some('3'),
            Key::Num4 => Some('4'),
            Key::Num5 => Some('5'),
            Key::Num6 => Some('6'),
            Key::Num7 => Some('7'),
            Key::Num8 => Some('8'),
            Key::Num9 => Some('9'),
            Key::Space => Some(' '),
            Key::Enter => Some('\n'),
            Key::Backspace => Some('\x08'),
            Key::Tab => Some('\t'),
            _ => None,
        }
    }
}

/// Global keyboard driver instance
pub static KEYBOARD: Mutex<KeyboardDriver> = Mutex::new(KeyboardDriver::new());

/// Initialize keyboard driver
/// 
/// # Safety
/// - Must be called during kernel initialization
pub unsafe fn init_keyboard(pic: &mut Pic) {
    KEYBOARD.lock().init(pic);
}

/// Handle keyboard interrupt
pub fn handle_keyboard_interrupt() {
    KEYBOARD.lock().handle_interrupt();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyboard_buffer() {
        let mut buffer = KeyboardBuffer::new();
        assert_eq!(buffer.len(), 0);
        
        buffer.push(Key::A);
        assert_eq!(buffer.len(), 1);
        
        let key = buffer.pop();
        assert_eq!(key, Some(Key::A));
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_key_to_char() {
        assert_eq!(Key::A.to_char(), Some('a'));
        assert_eq!(Key::Space.to_char(), Some(' '));
        assert_eq!(Key::Enter.to_char(), Some('\n'));
        assert_eq!(Key::ArrowUp.to_char(), None);
    }
}
