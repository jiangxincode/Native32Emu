// Audio engine: handles MP3 and raw PCM audio playback.

use crate::file_loader::{AudioFormat, Colorspace, Native32Reader};

#[cfg(feature = "standalone")]
use rodio::Source;
#[cfg(feature = "standalone")]
use std::num::NonZero;

pub struct AudioEngine {
    // Standalone mode: rodio audio
    #[cfg(feature = "standalone")]
    _mixer_device: Option<rodio::MixerDeviceSink>,
    #[cfg(feature = "standalone")]
    mixer: Option<rodio::mixer::Mixer>,
    #[cfg(feature = "standalone")]
    player: Option<rodio::Player>,

    pub volume: f32,
    pub colorspace: Colorspace,
    // Track which movie owns the currently playing sound
    pub music_owner: Option<String>,

    // Libretro mode: pending audio samples buffer
    pending_samples: Vec<i16>,
    // Simple tone generator for testing
    tone_phase: f64,
    tone_active: bool,
}

impl AudioEngine {
    pub fn new(colorspace: Colorspace, volume: u32) -> Self {
        #[cfg(feature = "standalone")]
        {
            let (mixer_device, mixer) = match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mds) => {
                    let mixer = mds.mixer().clone();
                    (Some(mds), Some(mixer))
                }
                Err(e) => {
                    log::warn!("Failed to initialize audio: {}", e);
                    (None, None)
                }
            };

            Self {
                _mixer_device: mixer_device,
                mixer,
                player: None,
                volume: volume as f32 / 100.0,
                colorspace,
                music_owner: None,
                pending_samples: Vec::new(),
                tone_phase: 0.0,
                tone_active: false,
            }
        }

        #[cfg(not(feature = "standalone"))]
        {
            Self {
                volume: volume as f32 / 100.0,
                colorspace,
                music_owner: None,
                pending_samples: Vec::new(),
                tone_phase: 0.0,
                tone_active: false,
            }
        }
    }

    /// Get pending audio samples for libretro mode.
    /// Returns interleaved stereo i16 samples.
    pub fn get_pending_samples(&mut self) -> Vec<i16> {
        // For now, generate a simple tone if active
        if self.tone_active {
            let sample_rate = match self.colorspace {
                Colorspace::YUV => 11025.0,
                Colorspace::ARGB => 22050.0,
            };
            let num_samples = (sample_rate / 30.0) as usize; // Samples per frame at 30fps
            let mut samples = Vec::with_capacity(num_samples * 2);

            for _ in 0..num_samples {
                let sample =
                    (self.tone_phase * 2.0 * std::f64::consts::PI * 440.0 / sample_rate).sin();
                let sample_i16 = (sample * 16000.0 * self.volume as f64) as i16;
                samples.push(sample_i16); // Left
                samples.push(sample_i16); // Right
                self.tone_phase += 1.0;
                if self.tone_phase >= sample_rate {
                    self.tone_phase -= sample_rate;
                }
            }
            samples
        } else {
            // Return silence
            let sample_rate = match self.colorspace {
                Colorspace::YUV => 11025.0,
                Colorspace::ARGB => 22050.0,
            };
            let num_samples = (sample_rate / 30.0) as usize;
            vec![0i16; num_samples * 2]
        }
    }

    /// Start a test tone (for debugging).
    pub fn start_tone(&mut self) {
        self.tone_active = true;
        self.tone_phase = 0.0;
    }

    /// Stop the test tone.
    pub fn stop_tone(&mut self) {
        self.tone_active = false;
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

    #[cfg(not(feature = "standalone"))]
    fn play_mp3(&mut self, _data: &[u8], _repeat: i32, _movie_name: &str) -> bool {
        log::debug!("MP3 playback requested (libretro mode - not implemented)");
        false
    }

    #[cfg(not(feature = "standalone"))]
    fn play_raw(&mut self, _data: &[u8], _repeat: i32, _movie_name: &str) -> bool {
        log::debug!("Raw audio playback requested (libretro mode - not implemented)");
        false
    }

    #[cfg(feature = "standalone")]
    fn play_mp3(&mut self, data: &[u8], repeat: i32, movie_name: &str) -> bool {
        if let Some(ref mixer) = self.mixer {
            // Stop current music
            if let Some(ref player) = self.player {
                player.stop();
            }

            let player = rodio::Player::connect_new(mixer);
            player.set_volume(self.volume);

            // Create a cursor from the data
            let cursor = std::io::Cursor::new(data.to_vec());
            match rodio::Decoder::new_mp3(cursor) {
                Ok(decoder) => {
                    let buffered = decoder.buffered();
                    if repeat == 0xFF {
                        // Infinite loop
                        player.append(buffered.repeat_infinite());
                    } else if repeat > 1 {
                        // Finite repeat: append the buffered source N times
                        for _ in 0..repeat {
                            player.append(buffered.clone());
                        }
                    } else {
                        // Play once (repeat == 0 or 1)
                        player.append(buffered);
                    }
                    self.player = Some(player);
                    self.music_owner = Some(movie_name.to_string());
                    true
                }
                Err(e) => {
                    log::warn!("Failed to decode MP3: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    #[cfg(feature = "standalone")]
    fn play_raw(&mut self, data: &[u8], repeat: i32, movie_name: &str) -> bool {
        if let Some(ref mixer) = self.mixer {
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

            let source = rodio::buffer::SamplesBuffer::new(
                NonZero::<u16>::new(1).unwrap(),
                NonZero::<u32>::new(sample_rate).unwrap(),
                samples,
            );

            let player = rodio::Player::connect_new(mixer);
            player.set_volume(self.volume);

            if repeat == 0xFF {
                player.append(source.repeat_infinite());
            } else if repeat > 1 {
                let buffered = source.buffered();
                for _ in 0..repeat {
                    player.append(buffered.clone());
                }
            } else {
                player.append(source);
            }

            self.player = Some(player);
            self.music_owner = Some(movie_name.to_string());
            true
        } else {
            false
        }
    }

    /// Stop all currently playing sounds.
    pub fn stop_all(&mut self) {
        #[cfg(feature = "standalone")]
        if let Some(ref player) = self.player {
            player.stop();
        }
        #[cfg(feature = "standalone")]
        {
            self.player = None;
        }
        self.music_owner = None;
        self.tone_active = false;
    }

    /// Stop sound only if the given movie is the current owner.
    pub fn stop_for_movie(&mut self, movie_name: &str) {
        if self.music_owner.as_deref() == Some(movie_name) {
            #[cfg(feature = "standalone")]
            if let Some(ref player) = self.player {
                player.stop();
            }
            #[cfg(feature = "standalone")]
            {
                self.player = None;
            }
            self.music_owner = None;
        }
    }

    /// Check if music is still playing.
    pub fn is_playing(&self) -> bool {
        #[cfg(feature = "standalone")]
        {
            self.player.as_ref().is_some_and(|p| !p.empty())
        }
        #[cfg(not(feature = "standalone"))]
        {
            self.tone_active
        }
    }

    /// Set the volume (0-100).
    pub fn set_volume(&mut self, volume: u32) {
        self.volume = volume as f32 / 100.0;
        #[cfg(feature = "standalone")]
        if let Some(ref player) = self.player {
            player.set_volume(self.volume);
        }
    }
}
