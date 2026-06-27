// Drives an MPEG-1 video stream for real-time playback: decodes frames on a
// time budget and renders the current frame (scaled) into an XRGB8888 buffer.
// Audio is decoded separately by the caller and handed to the audio engine.

use super::video::Video;

pub struct VideoPlayer {
    video: Video,
    framerate: f64,
    /// Current playback time, in seconds (accumulator, not a loop counter).
    time: f64,
    /// Number of frames decoded/shown so far (drives the decode loop).
    frames_shown: u64,
    /// Index of the most recently decoded displayable frame.
    cur_frame: Option<usize>,
    finished: bool,
}

impl VideoPlayer {
    /// Create a player over a video elementary stream. Returns `None` if no
    /// sequence header could be parsed.
    pub fn new(video_es: Vec<u8>) -> Option<Self> {
        let mut video = Video::new(video_es);
        if !video.has_header() {
            return None;
        }
        let framerate = if video.framerate() > 0.0 {
            video.framerate()
        } else {
            25.0
        };
        Some(Self {
            video,
            framerate,
            time: 0.0,
            frames_shown: 0,
            cur_frame: None,
            finished: false,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    pub fn elapsed(&self) -> f64 {
        self.time
    }

    /// Advance playback by `dt` seconds, decoding frames as needed, and render
    /// the current frame into `dst` (dst_w x dst_h XRGB8888).
    pub fn advance_and_render(&mut self, dt: f64, dst: &mut [u32], dst_w: usize, dst_h: usize) {
        if !self.finished {
            self.time += dt;
            // Number of frames that should have been shown by `time`. Decoding
            // is driven by this integer target, not a floating-point counter.
            let target = (self.time * self.framerate) as u64;
            while self.frames_shown <= target {
                match self.video.decode() {
                    Some(idx) => {
                        self.cur_frame = Some(idx);
                        self.frames_shown += 1;
                    }
                    None => {
                        self.finished = true;
                        break;
                    }
                }
            }
        }

        if let Some(idx) = self.cur_frame {
            self.video.frame(idx).write_rgb_scaled(dst, dst_w, dst_h);
        }
    }
}
