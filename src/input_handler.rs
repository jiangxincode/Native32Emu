// Input handler: maps keyboard input to Native32 button keycodes.

#[cfg(feature = "standalone")]
use std::collections::HashMap;
use std::collections::HashSet;

/// Default keycode mappings: Native32 keycode -> minifb Key
#[cfg(feature = "standalone")]
pub const DEFAULT_KEY_MAP: &[(u16, minifb::Key)] = &[
    (0x0200, minifb::Key::Left),
    (0x0400, minifb::Key::Right),
    (0x1c00, minifb::Key::Up),
    (0x1e00, minifb::Key::Down),
    (0x4000, minifb::Key::Z),
    (0x8800, minifb::Key::X),
];

pub struct InputHandler {
    /// Map from Native32 keycode to physical key (standalone mode)
    #[cfg(feature = "standalone")]
    pub key_map: HashMap<u16, minifb::Key>,
    /// Currently pressed buttons (libretro mode)
    pressed_buttons: HashSet<u16>,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "standalone")]
            key_map: {
                let mut map = HashMap::new();
                for &(keycode, key) in DEFAULT_KEY_MAP {
                    map.insert(keycode, key);
                }
                map
            },
            pressed_buttons: HashSet::new(),
        }
    }

    /// Apply user-specified key remappings (standalone mode).
    #[cfg(feature = "standalone")]
    pub fn remap(&mut self, remappings: &[(u16, minifb::Key)]) {
        for &(keycode, key) in remappings {
            self.key_map.insert(keycode, key);
        }
    }

    /// Check which Native32 keycodes are currently pressed (standalone mode).
    #[cfg(feature = "standalone")]
    pub fn get_pressed_keycodes(&self, window: &minifb::Window) -> Vec<u16> {
        let mut pressed = Vec::new();
        for (&keycode, &key) in &self.key_map {
            if window.is_key_down(key) {
                pressed.push(keycode);
            }
        }
        pressed
    }

    /// Set the currently pressed buttons (libretro mode).
    pub fn set_buttons(&mut self, buttons: u16) {
        self.pressed_buttons.clear();
        // Check each button bit
        if buttons & 0x0200 != 0 {
            self.pressed_buttons.insert(0x0200);
        }
        if buttons & 0x0400 != 0 {
            self.pressed_buttons.insert(0x0400);
        }
        if buttons & 0x1c00 != 0 {
            self.pressed_buttons.insert(0x1c00);
        }
        if buttons & 0x1e00 != 0 {
            self.pressed_buttons.insert(0x1e00);
        }
        if buttons & 0x4000 != 0 {
            self.pressed_buttons.insert(0x4000);
        }
        if buttons & 0x8800 != 0 {
            self.pressed_buttons.insert(0x8800);
        }
    }

    /// Get the currently pressed buttons (libretro mode).
    pub fn get_pressed_buttons(&self) -> Vec<u16> {
        self.pressed_buttons.iter().copied().collect()
    }
}
