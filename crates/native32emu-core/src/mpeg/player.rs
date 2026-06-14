// Drives an MPEG-1 video stream for real-time playback: decodes frames on a
// time budget and renders the current frame (scaled) into an XRGB8888 buffer.
// Audio is decoded separately by the caller and handed to the audio engine.

use super::video::Video;

pub struct VideoPlayer {
    video: Video,
    framerate: f64,
    /// Current playback time, in seconds.
    time: f64,
    /// Time at which the next frame should be decoded.
    next_decode_time: f64,
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
            next_decode_time: 0.0,
            cur_frame: None,
            finished: false,
        })
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Advance playback by `dt` seconds, decoding frames as needed, and render
    /// the current frame into `dst` (dst_w x dst_h XRGB8888).
    pub fn advance_and_render(&mut self, dt: f64, dst: &mut [u32], dst_w: usize, dst_h: usize) {
        if !self.finished {
            self.time += dt;
            while self.time >= self.next_decode_time {
                match self.video.decode() {
                    Some(idx) => {
                        self.cur_frame = Some(idx);
                        self.next_decode_time += 1.0 / self.framerate;
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
