//! PS/2 Mouse Driver
//! 
//! Additional Hardware I/O
//! Classification: RUNTIME
//! 
//! Handles PS/2 mouse input from IRQ12.

use crate::pic::{self, Pic};
use spin::Mutex;
use x86_64::instructions::port::Port;

/// Mouse data port (shared with keyboard)
const PS2_DATA_PORT: u16 = 0x60;

/// Mouse command port
const PS2_CMD_PORT: u16 = 0x64;

/// Mouse buffer size
const MOUSE_BUFFER_SIZE: usize = 64;

/// Mouse driver
pub struct MouseDriver {
    state: MouseState,
    packet: [u8; 5],
    packet_index: usize,
    packet_size: usize,
    buffer: MouseBuffer,
    cursor_x: i32,
    cursor_y: i32,
    screen_width: i32,
    screen_height: i32,
}

impl MouseDriver {
    /// Create a new mouse driver
    pub const fn new() -> Self {
        Self {
            state: MouseState::Reset,
            packet: [0; 5],
            packet_index: 0,
            packet_size: 3,
            buffer: MouseBuffer::new(),
            cursor_x: 0,
            cursor_y: 0,
            screen_width: 800,
            screen_height: 600,
        }
    }

    /// Initialize the mouse
    /// 
    /// # Safety
    /// - Programs PS/2 controller
    /// - Must be called before enabling interrupts
    pub unsafe fn init(&mut self, pic: &mut Pic) {
        // Enable mouse (IRQ12)
        pic.enable_irq(12);
        
        // Enable mouse in PS/2 controller
        self.enable_mouse();
        
        // Set sample rate
        self.set_sample_rate(100);
        
        // Set resolution
        self.set_resolution(3);
        
        // Enable mouse events
        self.enable_packet_streaming();
    }

    /// Enable mouse
    fn enable_mouse(&mut self) {
        self.write_ps2(0xA8); // Enable mouse
        self.write_ps2(0xD4); // Write to mouse
        self.write_ps2(0xF4); // Enable mouse data reporting
    }

    /// Write to PS/2 controller
    fn write_ps2(&mut self, data: u8) {
        // Wait for input buffer to be empty
        while (Port::new(PS2_CMD_PORT).read() & 0x02) != 0 {
            core::hint::spin_loop();
        }
        Port::new(PS2_DATA_PORT).write(data);
    }

    /// Set sample rate
    fn set_sample_rate(&mut self, rate: u8) {
        self.write_ps2(0xD4);
        self.write_ps2(0xF3);
        self.write_ps2(rate);
    }

    /// Set resolution
    fn set_resolution(&mut self, resolution: u8) {
        self.write_ps2(0xD4);
        self.write_ps2(0xE8);
        self.write_ps2(resolution);
    }

    /// Enable packet streaming
    fn enable_packet_streaming(&mut self) {
        self.write_ps2(0xD4);
        self.write_ps2(0xF4);
    }

    /// Handle mouse interrupt
    pub fn handle_interrupt(&mut self) {
        // Read mouse data
        let byte = Port::new(PS2_DATA_PORT).read();
        
        // Process byte based on state machine
        match self.state {
            MouseState::Reset => {
                // Look for packet start (bit 3 must be 1)
                if (byte & 0x08) != 0 {
                    self.packet[0] = byte;
                    self.packet_index = 1;
                    
                    // Determine packet size (bit 4 = scroll wheel present)
                    self.packet_size = if (byte & 0x10) != 0 { 4 } else { 3 };
                    
                    if self.packet_size == 3 {
                        self.state = MouseState::Reading;
                    } else {
                        self.state = MouseState::ReadingWheel;
                    }
                }
            }
            MouseState::Reading | MouseState::ReadingWheel => {
                self.packet[self.packet_index] = byte;
                self.packet_index += 1;
                
                if self.packet_index >= self.packet_size {
                    // Packet complete, decode it
                    self.decode_packet();
                    self.packet_index = 0;
                    self.state = MouseState::Reset;
                }
            }
        }
    }

    /// Decode mouse packet
    fn decode_packet(&mut self) {
        let byte0 = self.packet[0];
        let x_move = self.packet[1] as i8;
        let y_move = -(self.packet[2] as i8); // Invert Y
        
        // Handle overflow bits
        let x_overflow = (byte0 & 0x10) != 0;
        let y_overflow = (byte0 & 0x20) != 0;
        
        if x_overflow || y_overflow {
            // Overflow, ignore this packet
            return;
        }
        
        // Update cursor position
        self.cursor_x += x_move as i32;
        self.cursor_y += y_move as i32;
        
        // Clamp to screen bounds
        self.cursor_x = self.cursor_x.max(0).min(self.screen_width - 1);
        self.cursor_y = self.cursor_y.max(0).min(self.screen_height - 1);
        
        // Check buttons
        let left_button = (byte0 & 0x01) != 0;
        let right_button = (byte0 & 0x02) != 0;
        let middle_button = (byte0 & 0x04) != 0;
        
        // Create mouse event
        let event = MouseEvent {
            x: self.cursor_x,
            y: self.cursor_y,
            dx: x_move as i32,
            dy: y_move as i32,
            left_button,
            right_button,
            middle_button,
            wheel: if self.packet_size >= 4 { self.packet[3] as i8 } else { 0 },
        };
        
        // Add to buffer
        self.buffer.push(event);
    }

    /// Get cursor position
    pub const fn cursor_position(&self) -> (i32, i32) {
        (self.cursor_x, self.cursor_y)
    }

    /// Read mouse event
    pub fn read_event(&mut self) -> Option<MouseEvent> {
        self.buffer.pop()
    }

    /// Check if events available
    pub const fn has_events(&self) -> bool {
        self.buffer.len() > 0
    }
}

impl Default for MouseDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Mouse state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MouseState {
    Reset,
    Reading,
    ReadingWheel,
}

/// Mouse event
#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    pub x: i32,
    pub y: i32,
    pub dx: i32,
    pub dy: i32,
    pub left_button: bool,
    pub right_button: bool,
    pub middle_button: bool,
    pub wheel: i8,
}

/// Mouse event buffer
struct MouseBuffer {
    buffer: [MouseEvent; MOUSE_BUFFER_SIZE],
    read_pos: usize,
    write_pos: usize,
}

impl MouseBuffer {
    const fn new() -> Self {
        Self {
            buffer: [MouseEvent {
                x: 0, y: 0, dx: 0, dy: 0,
                left_button: false, right_button: false,
                middle_button: false, wheel: 0,
            }; MOUSE_BUFFER_SIZE],
            read_pos: 0,
            write_pos: 0,
        }
    }

    fn push(&mut self, event: MouseEvent) {
        let next_write = (self.write_pos + 1) % MOUSE_BUFFER_SIZE;
        if next_write != self.read_pos {
            self.buffer[self.write_pos] = event;
            self.write_pos = next_write;
        }
    }

    fn pop(&mut self) -> Option<MouseEvent> {
        if self.read_pos == self.write_pos {
            return None;
        }
        
        let event = self.buffer[self.read_pos];
        self.read_pos = (self.read_pos + 1) % MOUSE_BUFFER_SIZE;
        Some(event)
    }

    const fn len(&self) -> usize {
        if self.write_pos >= self.read_pos {
            self.write_pos - self.read_pos
        } else {
            MOUSE_BUFFER_SIZE - self.read_pos + self.write_pos
        }
    }
}

/// Global mouse driver instance
pub static MOUSE: Mutex<MouseDriver> = Mutex::new(MouseDriver::new());

/// Initialize mouse driver
/// 
/// # Safety
/// - Must be called during kernel initialization
pub unsafe fn init_mouse(pic: &mut Pic) {
    MOUSE.lock().init(pic);
}

/// Handle mouse interrupt
pub fn handle_mouse_interrupt() {
    MOUSE.lock().handle_interrupt();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_buffer() {
        let mut buffer = MouseBuffer::new();
        assert_eq!(buffer.len(), 0);
        
        let event = MouseEvent {
            x: 100, y: 200, dx: 1, dy: -1,
            left_button: true, right_button: false,
            middle_button: false, wheel: 0,
        };
        buffer.push(event);
        assert_eq!(buffer.len(), 1);
        
        let popped = buffer.pop();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().x, 100);
        assert_eq!(buffer.len(), 0);
    }
}
