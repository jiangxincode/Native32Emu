// Input handler: maps keyboard input to Native32 button keycodes.

#[cfg(feature = "standalone")]
use std::collections::HashMap;
use std::collections::HashSet;

/// Native32 keycode for the A button (south/confirm).
pub const KEYCODE_A: u16 = 0x4000;
/// Native32 keycode for the B button (east/cancel).
pub const KEYCODE_B: u16 = 0x8800;

/// Default keycode mappings: Native32 keycode -> minifb Key
#[cfg(feature = "standalone")]
pub const DEFAULT_KEY_MAP: &[(u16, minifb::Key)] = &[
    (0x0200, minifb::Key::Left),
    (0x0400, minifb::Key::Right),
    (0x1c00, minifb::Key::Up),
    (0x1e00, minifb::Key::Down),
    (KEYCODE_A, minifb::Key::Z),
    (KEYCODE_B, minifb::Key::X),
];

pub struct InputHandler {
    /// Map from Native32 keycode to physical key (standalone mode)
    #[cfg(feature = "standalone")]
    pub key_map: HashMap<u16, minifb::Key>,
    /// Number of consecutive frames each currently-held keycode has been down.
    /// 0 means it was first pressed on the most recent frame.
    held_frames: std::collections::HashMap<u16, u32>,
    /// Keycodes that count as "pressed" this frame after typematic filtering.
    active_buttons: HashSet<u16>,
    /// Frames a key must be held before auto-repeat starts.
    repeat_delay: u32,
    /// Frames between auto-repeat pulses once repeating.
    repeat_period: u32,
    /// When true, the A and B buttons are swapped before processing.
    swap_ab: bool,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    /// Default typematic key-repeat timing, in 30fps frames.
    ///
    /// The original hardware does not report a held key as pressed on every
    /// frame; its keypad driver emits an initial press, waits, then auto-repeats.
    /// Several games rely on the gap this produces. The clearest example is the
    /// walk/run mechanic in some action titles: holding a direction walks, and
    /// the pause-then-repeat from auto-repeat is what the game detects to break
    /// into a run. Reporting the key as pressed every frame keeps the player
    /// permanently walking. These defaults reproduce the hardware behaviour:
    /// ~0.4s before repeat, then a pulse every ~0.1s.
    pub const DEFAULT_REPEAT_DELAY: u32 = 12;
    pub const DEFAULT_REPEAT_PERIOD: u32 = 3;

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
            held_frames: std::collections::HashMap::new(),
            active_buttons: HashSet::new(),
            repeat_delay: Self::DEFAULT_REPEAT_DELAY,
            repeat_period: Self::DEFAULT_REPEAT_PERIOD,
            swap_ab: false,
        }
    }

    /// Override the typematic key-repeat timing (in 30fps frames).
    pub fn set_repeat_timing(&mut self, delay: u32, period: u32) {
        self.repeat_delay = delay;
        self.repeat_period = period.max(1);
    }

    /// Enable or disable swapping the A and B buttons.
    pub fn set_swap_ab(&mut self, swap: bool) {
        self.swap_ab = swap;
    }

    /// Swap the A and B keycodes; all other keycodes pass through unchanged.
    fn apply_ab_swap(keycode: u16) -> u16 {
        match keycode {
            KEYCODE_A => KEYCODE_B,
            KEYCODE_B => KEYCODE_A,
            other => other,
        }
    }

    /// Decide whether a key that has been held for `count` frames should be
    /// reported as pressed this frame, given the typematic timing.
    fn is_active(count: u32, delay: u32, period: u32) -> bool {
        if count == 0 {
            // Initial press always registers.
            true
        } else if count < delay {
            // Still inside the pre-repeat delay: treat as released.
            false
        } else {
            // Auto-repeating: pulse every `period` frames.
            (count - delay).is_multiple_of(period.max(1))
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
    ///
    /// This returns the raw physical key state (no typematic filtering); it is
    /// fed into [`set_buttons`](Self::set_buttons), which applies the repeat
    /// logic. It is also used directly by the on-screen gamepad overlay, which
    /// wants the raw held state for display.
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

    /// Set the currently pressed buttons and apply typematic key-repeat.
    ///
    /// Takes the exact Native32 keycodes that are physically down this frame.
    /// These keycodes are NOT independent bit flags (e.g. DOWN 0x1e00 == UP
    /// 0x1c00 | LEFT 0x0200), so they are stored verbatim rather than decoded
    /// from a packed bitmask.
    ///
    /// Must be called once per frame. It updates per-key hold counters and
    /// computes which keys count as "pressed" this frame: a key registers on
    /// its initial press, goes quiet for `repeat_delay` frames, then pulses
    /// every `repeat_period` frames, matching the hardware keypad driver.
    pub fn set_buttons(&mut self, keycodes: &[u16]) {
        // Optionally swap A/B before any other processing so downstream logic
        // (typematic filtering, button-event dispatch) sees the swapped layout.
        let swapped: Vec<u16>;
        let keycodes: &[u16] = if self.swap_ab {
            swapped = keycodes.iter().map(|&k| Self::apply_ab_swap(k)).collect();
            &swapped
        } else {
            keycodes
        };

        let mut new_counts = std::collections::HashMap::with_capacity(keycodes.len());
        let mut active = HashSet::new();
        for &keycode in keycodes {
            // Skip duplicates so a repeated keycode is only counted once.
            if new_counts.contains_key(&keycode) {
                continue;
            }
            let count = match self.held_frames.get(&keycode) {
                Some(&prev) => prev + 1, // still held
                None => 0,               // newly pressed this frame
            };
            new_counts.insert(keycode, count);
            if Self::is_active(count, self.repeat_delay, self.repeat_period) {
                active.insert(keycode);
            }
        }
        self.held_frames = new_counts;
        self.active_buttons = active;
    }

    /// Get the keycodes that count as pressed this frame after typematic
    /// filtering. Consumed by the button-event dispatcher.
    pub fn get_pressed_buttons(&self) -> Vec<u16> {
        self.active_buttons.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_handler_has_no_pressed_buttons() {
        let handler = InputHandler::new();
        assert!(handler.get_pressed_buttons().is_empty());
    }

    #[test]
    fn test_default_trait() {
        let handler = InputHandler::default();
        assert!(handler.get_pressed_buttons().is_empty());
    }

    #[test]
    fn test_set_buttons_stores_keycodes() {
        let mut handler = InputHandler::new();
        handler.set_buttons(&[0x0200, 0x1c00]); // Left, Up
        let pressed = handler.get_pressed_buttons();
        assert_eq!(pressed.len(), 2);
        assert!(pressed.contains(&0x0200));
        assert!(pressed.contains(&0x1c00));
    }

    #[test]
    fn test_set_buttons_clears_previous() {
        let mut handler = InputHandler::new();
        handler.set_buttons(&[0x0200]);
        handler.set_buttons(&[0x0400]); // replace with Right only
        let pressed = handler.get_pressed_buttons();
        assert_eq!(pressed.len(), 1);
        assert!(pressed.contains(&0x0400));
        assert!(!pressed.contains(&0x0200));
    }

    #[test]
    fn test_set_buttons_empty_clears_all() {
        let mut handler = InputHandler::new();
        handler.set_buttons(&[0x0200, 0x0400]);
        handler.set_buttons(&[]);
        assert!(handler.get_pressed_buttons().is_empty());
    }

    #[test]
    fn test_set_buttons_deduplicates() {
        let mut handler = InputHandler::new();
        handler.set_buttons(&[0x0200, 0x0200, 0x0200]);
        let pressed = handler.get_pressed_buttons();
        assert_eq!(pressed.len(), 1);
    }

    #[test]
    fn test_set_buttons_with_all_directions() {
        let mut handler = InputHandler::new();
        handler.set_buttons(&[0x0200, 0x0400, 0x1c00, 0x1e00, 0x4000, 0x8800]);
        assert_eq!(handler.get_pressed_buttons().len(), 6);
    }

    #[test]
    fn test_initial_press_is_active() {
        let mut handler = InputHandler::new();
        handler.set_buttons(&[0x0200]);
        assert!(handler.get_pressed_buttons().contains(&0x0200));
    }

    #[test]
    fn test_held_key_goes_quiet_during_repeat_delay() {
        let mut handler = InputHandler::new();
        handler.set_repeat_timing(12, 3);
        handler.set_buttons(&[0x0200]); // frame 0: active (initial press)
        assert!(handler.get_pressed_buttons().contains(&0x0200));
        // Frames 1..12 are inside the delay window and must be silent, so the
        // game observes a release gap (this is what enables the run mechanic).
        for _ in 1..12 {
            handler.set_buttons(&[0x0200]);
            assert!(
                handler.get_pressed_buttons().is_empty(),
                "held key must be quiet during the repeat delay"
            );
        }
    }

    #[test]
    fn test_held_key_repeats_after_delay() {
        let mut handler = InputHandler::new();
        handler.set_repeat_timing(12, 3);
        // Frame 0 is the initial press; count reaches `delay` on frame 12.
        for frame in 0..=12 {
            handler.set_buttons(&[0x0200]);
            let active = handler.get_pressed_buttons().contains(&0x0200);
            let expected = frame == 0 || frame == 12;
            assert_eq!(active, expected, "unexpected activity on frame {frame}");
        }
        // Next pulse is `period` frames later.
        for frame in 13..=15 {
            handler.set_buttons(&[0x0200]);
            let active = handler.get_pressed_buttons().contains(&0x0200);
            assert_eq!(active, frame == 15, "unexpected activity on frame {frame}");
        }
    }

    #[test]
    fn test_release_resets_repeat_state() {
        let mut handler = InputHandler::new();
        handler.set_repeat_timing(12, 3);
        for _ in 0..5 {
            handler.set_buttons(&[0x0200]);
        }
        handler.set_buttons(&[]); // release
        assert!(handler.get_pressed_buttons().is_empty());
        // Re-press registers immediately as a fresh initial press.
        handler.set_buttons(&[0x0200]);
        assert!(handler.get_pressed_buttons().contains(&0x0200));
    }

    #[test]
    fn test_swap_ab_swaps_a_and_b() {
        let mut handler = InputHandler::new();
        handler.set_swap_ab(true);
        handler.set_buttons(&[KEYCODE_A]);
        // Pressing A registers as B when swapped.
        assert!(handler.get_pressed_buttons().contains(&KEYCODE_B));
        assert!(!handler.get_pressed_buttons().contains(&KEYCODE_A));
    }

    #[test]
    fn test_swap_ab_leaves_directions_untouched() {
        let mut handler = InputHandler::new();
        handler.set_swap_ab(true);
        handler.set_buttons(&[0x0200, KEYCODE_B]); // Left + B
        let pressed = handler.get_pressed_buttons();
        assert!(pressed.contains(&0x0200)); // direction unchanged
        assert!(pressed.contains(&KEYCODE_A)); // B -> A when swapped
    }

    #[test]
    fn test_swap_ab_disabled_by_default() {
        let mut handler = InputHandler::new();
        handler.set_buttons(&[KEYCODE_A]);
        assert!(handler.get_pressed_buttons().contains(&KEYCODE_A));
    }
}
