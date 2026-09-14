//! Input Event Routing
//! 
//! Priority 7: Desktop Environment
//! Classification: PRODUCT
//! 
//! Routes keyboard and mouse events to windows.

use crate::keyboard::Key;
use crate::mouse::MouseEvent;
use super::window_server::{WINDOW_SERVER, WindowId};

/// Input event types
#[derive(Debug, Clone)]
pub enum InputEvent {
    Key(Key),
    MouseMove(i32, i32),
    MouseButton { x: i32, y: i32, button: MouseButton, pressed: bool },
    MouseWheel { x: i32, y: i32, delta: i8 },
}

/// Mouse buttons
#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Input event router
pub struct InputRouter {
    last_mouse_x: i32,
    last_mouse_y: i32,
}

impl InputRouter {
    /// Create new input router
    pub const fn new() -> Self {
        Self {
            last_mouse_x: 0,
            last_mouse_y: 0,
        }
    }

    /// Route keyboard event
    pub fn route_keyboard(&self, key: Key) {
        let window_server = WINDOW_SERVER.get().expect("Window server not initialized");
        
        // Send to focused window
        if let Some(focused_id) = window_server.focused_window() {
            // In real implementation, would send event to window
            // For now, just log
        }
    }

    /// Route mouse event
    pub fn route_mouse(&mut self, event: MouseEvent) {
        let window_server = WINDOW_SERVER.get().expect("Window server not initialized");
        
        // Update cursor position
        let new_x = self.last_mouse_x + event.dx;
        let new_y = self.last_mouse_y + event.dy;
        
        self.last_mouse_x = new_x;
        self.last_mouse_y = new_y;
        
        // Route based on event type
        if event.left_button || event.right_button || event.middle_button {
            // Mouse button event
            let button = if event.left_button {
                MouseButton::Left
            } else if event.right_button {
                MouseButton::Right
            } else {
                MouseButton::Middle
            };
            
            let input_event = InputEvent::MouseButton {
                x: new_x,
                y: new_y,
                button,
                pressed: true, // Simplified
            };
            
            // Find window at position
            if let Some(window_id) = window_server.window_at(new_x, new_y) {
                // Focus the window
                window_server.focus_window(window_id);
                
                // Send event to window
                self.send_to_window(window_id, input_event);
            }
        } else if event.dx != 0 || event.dy != 0 {
            // Mouse move event
            let input_event = InputEvent::MouseMove(new_x, new_y);
            
            // Send to window under cursor
            if let Some(window_id) = window_server.window_at(new_x, new_y) {
                self.send_to_window(window_id, input_event);
            }
        }
    }

    /// Send event to window
    fn send_to_window(&self, window_id: WindowId, event: InputEvent) {
        // In real implementation, would add to window's event queue
        // For now, just drop the event
        drop((window_id, event));
    }
}

impl Default for InputRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Global input router
pub static INPUT_ROUTER: spin::Once<spin::Mutex<InputRouter>> = spin::Once::new();

/// Initialize input router
pub fn init_input_router() {
    INPUT_ROUTER.call_once(|| spin::Mutex::new(InputRouter::new()));
}

/// Handle keyboard event
pub fn handle_keyboard_event(key: Key) {
    if let Some(router) = INPUT_ROUTER.get() {
        router.lock().route_keyboard(key);
    }
}

/// Handle mouse event
pub fn handle_mouse_event(event: MouseEvent) {
    if let Some(router) = INPUT_ROUTER.get() {
        router.lock().route_mouse(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_router() {
        let router = InputRouter::new();
        assert_eq!(router.last_mouse_x, 0);
        assert_eq!(router.last_mouse_y, 0);
    }
}
