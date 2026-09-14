//! Window Server
//! 
//! Priority 7: Desktop Environment
//! Classification: PRODUCT
//! 
//! Manages windows and surfaces for the desktop environment.

use spin::Mutex;

/// Surface ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceId(pub u64);

/// Window ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

/// Rectangle
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && px < self.x + self.width as i32
            && py >= self.y
            && py < self.y + self.height as i32
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width as i32
            && self.x + self.width as i32 > other.x
            && self.y < other.y + other.height as i32
            && self.y + self.height as i32 > other.y
    }
}

/// Surface (backing store for window content)
pub struct Surface {
    pub id: SurfaceId,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>, // RGBA
}

impl Surface {
    pub fn new(id: SurfaceId, width: u32, height: u32) -> Self {
        Self {
            id,
            width,
            height,
            pixels: vec![0; (width * height) as usize],
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            self.pixels[index] = color;
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> u32 {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            self.pixels[index]
        } else {
            0
        }
    }

    pub fn clear(&mut self, color: u32) {
        for pixel in &mut self.pixels {
            *pixel = color;
        }
    }
}

/// Window
pub struct Window {
    pub id: WindowId,
    pub surface: Surface,
    pub bounds: Rect,
    pub title: String,
    pub focused: bool,
    pub visible: bool,
    pub z_order: u32,
}

impl Window {
    pub fn new(id: WindowId, x: i32, y: i32, width: u32, height: u32, title: &str) -> Self {
        let surface = Surface::new(SurfaceId(id.0), width, height);
        
        Self {
            id,
            surface,
            bounds: Rect::new(x, y, width, height),
            title: title.to_string(),
            focused: false,
            visible: true,
            z_order: 0,
        }
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        self.bounds.x = x;
        self.bounds.y = y;
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.bounds.width = width;
        self.bounds.height = height;
        self.surface = Surface::new(self.surface.id, width, height);
    }
}

/// Window server
pub struct WindowServer {
    windows: Mutex<Vec<Window>>,
    next_window_id: u64,
    next_surface_id: u64,
    focused_window: Option<WindowId>,
    screen_width: u32,
    screen_height: u32,
}

impl WindowServer {
    /// Create new window server
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        Self {
            windows: Mutex::new(Vec::new()),
            next_window_id: 1,
            next_surface_id: 1,
            focused_window: None,
            screen_width,
            screen_height,
        }
    }

    /// Create a new window
    pub fn create_window(
        &self,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        title: &str,
    ) -> WindowId {
        let mut windows = self.windows.lock();
        
        let window_id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        
        let window = Window::new(window_id, x, y, width, height, title);
        windows.push(window);
        
        // Focus the new window
        self.focused_window = Some(window_id);
        
        window_id
    }

    /// Destroy a window
    pub fn destroy_window(&self, window_id: WindowId) {
        let mut windows = self.windows.lock();
        windows.retain(|w| w.id != window_id);
        
        if self.focused_window == Some(window_id) {
            self.focused_window = None;
        }
    }

    /// Get window by ID
    pub fn get_window(&self, window_id: WindowId) -> Option<&Window> {
        let windows = self.windows.lock();
        windows.iter().find(|w| w.id == window_id)
    }

    /// Get mutable window by ID
    pub fn get_window_mut(&self, window_id: WindowId) -> Option<&mut Window> {
        let mut windows = self.windows.lock();
        windows.iter_mut().find(|w| w.id == window_id)
    }

    /// Focus a window
    pub fn focus_window(&self, window_id: WindowId) {
        let mut windows = self.windows.lock();
        
        for window in windows.iter_mut() {
            window.focused = window.id == window_id;
        }
        
        self.focused_window = Some(window_id);
    }

    /// Get focused window
    pub fn focused_window(&self) -> Option<WindowId> {
        self.focused_window
    }

    /// Find window at position
    pub fn window_at(&self, x: i32, y: i32) -> Option<WindowId> {
        let windows = self.windows.lock();
        
        // Search from top to bottom (reverse z-order)
        for window in windows.iter().rev() {
            if window.visible && window.bounds.contains(x, y) {
                return Some(window.id);
            }
        }
        
        None
    }

    /// Get all visible windows sorted by z-order
    pub fn get_visible_windows(&self) -> Vec<&Window> {
        let windows = self.windows.lock();
        let mut visible: Vec<&Window> = windows
            .iter()
            .filter(|w| w.visible)
            .collect();
        
        visible.sort_by_key(|w| w.z_order);
        visible
    }

    /// Get screen dimensions
    pub const fn screen_size(&self) -> (u32, u32) {
        (self.screen_width, self.screen_height)
    }
}

/// Global window server instance
pub static WINDOW_SERVER: spin::Once<WindowServer> = spin::Once::new();

/// Initialize window server
pub fn init_window_server(screen_width: u32, screen_height: u32) {
    WINDOW_SERVER.call_once(|| WindowServer::new(screen_width, screen_height));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(0, 0, 100, 100);
        assert!(rect.contains(50, 50));
        assert!(!rect.contains(100, 100));
    }

    #[test]
    fn test_rect_intersects() {
        let a = Rect::new(0, 0, 100, 100);
        let b = Rect::new(50, 50, 100, 100);
        assert!(a.intersects(&b));
        
        let c = Rect::new(200, 200, 100, 100);
        assert!(!a.intersects(&c));
    }

    #[test]
    fn test_surface_pixel() {
        let mut surface = Surface::new(SurfaceId(1), 100, 100);
        surface.set_pixel(10, 20, 0xFF0000FF);
        assert_eq!(surface.get_pixel(10, 20), 0xFF0000FF);
    }
}
