//! Desktop Compositor
//! 
//! Priority 7: Desktop Environment
//! Classification: PRODUCT
//! 
//! Composites windows onto the framebuffer.

use super::window_server::{WindowServer, WINDOW_SERVER, Rect, Surface};
use crate::vga::VgaDriver;

/// Compositor
pub struct Compositor {
    framebuffer: *mut u32,
    width: u32,
    height: u32,
    cursor_x: i32,
    cursor_y: i32,
}

impl Compositor {
    /// Create new compositor
    pub fn new(framebuffer: *mut u32, width: u32, height: u32) -> Self {
        Self {
            framebuffer,
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    /// Composite all windows to framebuffer
    pub fn composite(&self) {
        let window_server = WINDOW_SERVER.get().expect("Window server not initialized");
        let windows = window_server.get_visible_windows();
        
        // Clear framebuffer (black)
        self.clear(0xFF000000);
        
        // Composite each window from bottom to top
        for window in windows {
            self.composite_window(window);
        }
        
        // Draw cursor
        self.draw_cursor();
    }

    /// Composite a single window
    fn composite_window(&self, window: &super::window_server::Window) {
        let win_x = window.bounds.x;
        let win_y = window.bounds.y;
        let win_w = window.bounds.width;
        let win_h = window.bounds.height;
        
        // Clip to screen bounds
        let clip_x = win_x.max(0) as u32;
        let clip_y = win_y.max(0) as u32;
        let clip_w = win_w.min(self.width.saturating_sub(win_x.max(0) as u32));
        let clip_h = win_h.min(self.height.saturating_sub(win_y.max(0) as u32));
        
        // Copy window surface to framebuffer
        for y in 0..clip_h {
            for x in 0..clip_w {
                let src_x = x.saturating_sub(win_x.max(0) as u32);
                let src_y = y.saturating_sub(win_y.max(0) as u32);
                
                let pixel = window.surface.get_pixel(src_x, src_y);
                self.set_pixel((win_x + x as i32) as u32, (win_y + y as i32) as u32, pixel);
            }
        }
    }

    /// Clear framebuffer
    fn clear(&self, color: u32) {
        unsafe {
            for i in 0..(self.width * self.height) as usize {
                *self.framebuffer.add(i) = color;
            }
        }
    }

    /// Set pixel in framebuffer
    fn set_pixel(&self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            unsafe {
                let index = (y * self.width + x) as usize;
                *self.framebuffer.add(index) = color;
            }
        }
    }

    /// Draw mouse cursor
    fn draw_cursor(&self) {
        // Simple arrow cursor (16x16)
        let cursor_color = 0xFFFFFFFF; // White
        let cursor_size = 16;
        
        for dy in 0..cursor_size {
            for dx in 0..cursor_size {
                let x = self.cursor_x + dx as i32;
                let y = self.cursor_y + dy as i32;
                
                if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
                    // Simple square cursor for now
                    self.set_pixel(x as u32, y as u32, cursor_color);
                }
            }
        }
    }

    /// Update cursor position
    pub fn set_cursor_position(&mut self, x: i32, y: i32) {
        self.cursor_x = x;
        self.cursor_y = y;
    }

    /// Get cursor position
    pub const fn cursor_position(&self) -> (i32, i32) {
        (self.cursor_x, self.cursor_y)
    }

    /// Wait for VSync
    pub fn wait_vsync(&self) {
        // In real implementation, would wait for vertical blank
        // For now, just a simple delay
        for _ in 0..100000 {
            core::hint::spin_loop();
        }
    }
}

/// Global compositor instance
pub static COMPOSITOR: spin::Once<spin::Mutex<Option<Compositor>>> = spin::Once::new();

/// Initialize compositor
pub fn init_compositor(framebuffer: *mut u32, width: u32, height: u32) {
    COMPOSITOR.call_once(|| spin::Mutex::new(Some(Compositor::new(framebuffer, width, height))));
}

/// Composite and present frame
pub fn present() {
    if let Some(compositor_opt) = COMPOSITOR.get() {
        if let Some(compositor) = compositor_opt.lock().as_ref() {
            compositor.composite();
            compositor.wait_vsync();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compositor_creation() {
        // Just verify it compiles
        let _compositor = Compositor::new(core::ptr::null_mut(), 800, 600);
    }
}
