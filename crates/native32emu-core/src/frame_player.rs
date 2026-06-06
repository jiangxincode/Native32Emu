// Frame player: manages the main timeline frame playback at 30fps.

pub struct FramePlayer {
    pub current_frame: u32,
    pub playing: bool,
    pub next_frame: Option<u32>,
}

impl Default for FramePlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl FramePlayer {
    pub fn new() -> Self {
        Self {
            current_frame: 0,
            playing: true,
            next_frame: Some(1),
        }
    }

    /// Advance the timeline by one tick (called at 30fps).
    pub fn tick(&mut self) {
        if self.next_frame.is_none() && self.playing {
            self.next_frame = Some(self.current_frame + 1);
        }
        // Note: actual frame switching is done by the emulator when loading the frame
    }

    /// Check if there's a pending frame switch.
    pub fn has_pending_frame(&self) -> bool {
        self.next_frame.is_some()
    }

    /// Consume the pending frame and return it.
    pub fn take_next_frame(&mut self) -> Option<u32> {
        self.next_frame.take()
    }

    /// Set the next frame to jump to (used by GotoFrame/GotoFrame2).
    pub fn goto(&mut self, frame: u32, playing: bool) {
        self.next_frame = Some(frame);
        self.playing = playing;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let fp = FramePlayer::new();
        assert_eq!(fp.current_frame, 0);
        assert!(fp.playing);
        assert_eq!(fp.next_frame, Some(1));
    }

    #[test]
    fn test_default_trait() {
        let fp = FramePlayer::default();
        assert_eq!(fp.current_frame, 0);
        assert!(fp.playing);
    }

    #[test]
    fn test_take_next_frame_consumes() {
        let mut fp = FramePlayer::new();
        assert_eq!(fp.take_next_frame(), Some(1));
        assert!(fp.next_frame.is_none());
    }

    #[test]
    fn test_take_next_frame_returns_none_when_empty() {
        let mut fp = FramePlayer::new();
        fp.take_next_frame();
        assert_eq!(fp.take_next_frame(), None);
    }

    #[test]
    fn test_has_pending_frame() {
        let mut fp = FramePlayer::new();
        assert!(fp.has_pending_frame());
        fp.take_next_frame();
        assert!(!fp.has_pending_frame());
    }

    #[test]
    fn test_tick_advances_when_playing_and_no_pending() {
        let mut fp = FramePlayer::new();
        fp.current_frame = 5;
        fp.take_next_frame(); // consume initial Some(1)
        fp.tick();
        assert_eq!(fp.next_frame, Some(6)); // current_frame + 1
    }

    #[test]
    fn test_tick_does_not_advance_when_paused() {
        let mut fp = FramePlayer::new();
        fp.playing = false;
        fp.take_next_frame(); // consume initial Some(1)
        fp.tick();
        assert!(fp.next_frame.is_none());
    }

    #[test]
    fn test_tick_does_not_advance_when_pending_exists() {
        let mut fp = FramePlayer::new();
        // next_frame is Some(1) initially
        fp.tick();
        // Should still be Some(1), not overwritten
        assert_eq!(fp.next_frame, Some(1));
    }

    #[test]
    fn test_goto_sets_frame_and_playing() {
        let mut fp = FramePlayer::new();
        fp.goto(42, false);
        assert_eq!(fp.next_frame, Some(42));
        assert!(!fp.playing);

        fp.goto(100, true);
        assert_eq!(fp.next_frame, Some(100));
        assert!(fp.playing);
    }

    #[test]
    fn test_goto_frame_zero() {
        let mut fp = FramePlayer::new();
        fp.goto(0, true);
        assert_eq!(fp.next_frame, Some(0));
    }

    #[test]
    fn test_full_tick_cycle() {
        let mut fp = FramePlayer::new();
        // Initial: frame 0, next=Some(1)
        assert_eq!(fp.current_frame, 0);

        // Take the pending frame
        let next = fp.take_next_frame();
        assert_eq!(next, Some(1));

        // Simulate emulator setting current_frame
        fp.current_frame = 1;

        // Tick should advance to 2
        fp.tick();
        assert_eq!(fp.next_frame, Some(2));
    }
}
