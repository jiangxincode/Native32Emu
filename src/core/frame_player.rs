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
