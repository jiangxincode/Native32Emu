// Audio engine: handles MP3 and raw PCM audio playback.

use std::num::NonZero;

use crate::file_loader::{AudioFormat, Colorspace, Native32Reader};
use rodio::Source;

pub struct AudioEngine {
    // MixerDeviceSink must be kept alive; dropping it stops playback
    _mixer_device: Option<rodio::MixerDeviceSink>,
    mixer: Option<rodio::mixer::Mixer>,
    player: Option<rodio::Player>,
    pub volume: f32,
    pub colorspace: Colorspace,
    // Track which movie owns the currently playing sound
    pub music_owner: Option<String>,
}

impl AudioEngine {
    pub fn new(colorspace: Colorspace, volume: u32) -> Self {
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
        if let Some(ref player) = self.player {
            player.stop();
        }
        self.player = None;
        self.music_owner = None;
    }

    /// Stop sound only if the given movie is the current owner.
    pub fn stop_for_movie(&mut self, movie_name: &str) {
        if self.music_owner.as_deref() == Some(movie_name) {
            if let Some(ref player) = self.player {
                player.stop();
            }
            self.player = None;
            self.music_owner = None;
        }
    }

    /// Check if music is still playing.
    pub fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(|p| !p.empty())
    }

    /// Set the volume (0-100).
    pub fn set_volume(&mut self, volume: u32) {
        self.volume = volume as f32 / 100.0;
        if let Some(ref player) = self.player {
            player.set_volume(self.volume);
        }
    }
}
