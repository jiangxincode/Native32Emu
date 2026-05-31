// Input handler: maps keyboard input to Native32 button keycodes.

use std::collections::HashMap;

/// Default keycode mappings: Native32 keycode -> minifb Key
pub const DEFAULT_KEY_MAP: &[(u16, minifb::Key)] = &[
    (0x0200, minifb::Key::Left),
    (0x0400, minifb::Key::Right),
    (0x1c00, minifb::Key::Up),
    (0x1e00, minifb::Key::Down),
    (0x4000, minifb::Key::Z),
    (0x8800, minifb::Key::X),
];

pub struct InputHandler {
    /// Map from Native32 keycode to physical key
    pub key_map: HashMap<u16, minifb::Key>,
}

impl InputHandler {
    pub fn new() -> Self {
        let mut key_map = HashMap::new();
        for &(keycode, key) in DEFAULT_KEY_MAP {
            key_map.insert(keycode, key);
        }
        Self { key_map }
    }

    /// Apply user-specified key remappings.
    pub fn remap(&mut self, remappings: &[(u16, minifb::Key)]) {
        for &(keycode, key) in remappings {
            self.key_map.insert(keycode, key);
        }
    }

    /// Check which Native32 keycodes are currently pressed.
    pub fn get_pressed_keycodes(&self, window: &minifb::Window) -> Vec<u16> {
        let mut pressed = Vec::new();
        for (&keycode, &key) in &self.key_map {
            if window.is_key_down(key) {
                pressed.push(keycode);
            }
        }
        pressed
    }
}
