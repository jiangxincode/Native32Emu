// Audio engine: handles MP3 and raw PCM audio playback.

use crate::file_loader::{AudioFormat, Colorspace, Native32Reader};

pub struct AudioEngine {
    _stream: Option<rodio::OutputStream>,
    handle: Option<rodio::OutputStreamHandle>,
    sink: Option<rodio::Sink>,
    pub volume: f32,
    pub colorspace: Colorspace,
    // Track which movie owns the music channel
    pub music_owner: Option<String>,
}

impl AudioEngine {
    pub fn new(colorspace: Colorspace, volume: u32) -> Self {
        let (stream, handle) = match rodio::OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(e) => {
                log::warn!("Failed to initialize audio: {}", e);
                (None, None)
            }
        };

        Self {
            _stream: stream,
            handle,
            sink: None,
            volume: volume as f32 / 100.0,
            colorspace,
            music_owner: None,
        }
    }

    /// Play a sound. Returns true if playback started.
    pub fn play_sound(
        &mut self,
        reader: &mut Native32Reader,
        sound_value: u16,
        movie_name: &str,
    ) -> bool {
        let repeat = ((sound_value >> 8) & 0xFF) as i32;
        let index = (sound_value & 0xFF) as u32;

        if index == 0 {
            return false;
        }

        let sound = match reader.get_sound(index) {
            Some(s) => s,
            None => {
                log::warn!("Failed to load sound {}", index);
                return false;
            }
        };

        match sound.format {
            AudioFormat::MP3 => self.play_mp3(&sound.data, repeat, movie_name),
            AudioFormat::Raw => self.play_raw(&sound.data, repeat, movie_name),
        }
    }

    fn play_mp3(&mut self, data: &[u8], repeat: i32, movie_name: &str) -> bool {
        if let Some(ref handle) = self.handle {
            // Stop current music
            if let Some(ref sink) = self.sink {
                sink.stop();
            }

            match rodio::Sink::try_new(handle) {
                Ok(sink) => {
                    sink.set_volume(self.volume);
                    let loops = if repeat == 0xFF { i32::MAX } else { repeat };

                    // Create a cursor from the data
                    let cursor = std::io::Cursor::new(data.to_vec());
                    match rodio::Decoder::new_mp3(cursor) {
                        Ok(decoder) => {
                            if loops > 0 {
                                // For repeated playback, use the sink's built-in looping
                                // rodio doesn't have native loop count, so we just play once
                                // and let the emulator re-trigger if needed
                                sink.append(decoder);
                            }
                            self.sink = Some(sink);
                            self.music_owner = Some(movie_name.to_string());
                            true
                        }
                        Err(e) => {
                            log::warn!("Failed to decode MP3: {}", e);
                            false
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to create audio sink: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    fn play_raw(&mut self, data: &[u8], _repeat: i32, movie_name: &str) -> bool {
        if let Some(ref handle) = self.handle {
            // Determine sample rate based on colorspace
            let sample_rate = match self.colorspace {
                Colorspace::YUV => 11025u32,
                Colorspace::ARGB => 22050u32,
            };

            // Convert raw 16-bit mono PCM to f32 samples
            let samples: Vec<f32> = data
                .chunks_exact(2)
                .map(|chunk| {
                    let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                    sample as f32 / 32768.0
                })
                .collect();

            let source = rodio::buffer::SamplesBuffer::new(1, sample_rate, samples);

            match rodio::Sink::try_new(handle) {
                Ok(sink) => {
                    sink.set_volume(self.volume);
                    sink.append(source);
                    self.sink = Some(sink);
                    self.music_owner = Some(movie_name.to_string());
                    true
                }
                Err(e) => {
                    log::warn!("Failed to create audio sink: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Stop all currently playing sounds.
    pub fn stop_all(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.stop();
        }
        self.sink = None;
        self.music_owner = None;
    }

    /// Check if music is still playing.
    pub fn is_playing(&self) -> bool {
        self.sink.as_ref().is_some_and(|s| !s.empty())
    }

    /// Set the volume (0-100).
    pub fn set_volume(&mut self, volume: u32) {
        self.volume = volume as f32 / 100.0;
        if let Some(ref sink) = self.sink {
            sink.set_volume(self.volume);
        }
    }
}
