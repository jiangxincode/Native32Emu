// Core emulator implementation for libretro.

use crate::action_vm::{ActionProp, ActionVM, VmHost};
use crate::audio_engine::AudioEngine;
use crate::content_loader::ContentLoader;
use crate::file_loader::{FrameObject, Native32Reader, ObjectType};
use crate::frame_player::FramePlayer;
use crate::input_handler::InputHandler;
use crate::renderer::Renderer;
use crate::save_manager::SaveManager;
use crate::sprite_system::{MovieState, SpriteSystem};
use anyhow::{Context, Result};
use std::path::PathBuf;

/// The platform-independent emulator core.
///
/// This holds all the game simulation state and is shared by both the
/// standalone (minifb window) front-end and the libretro core. Platform
/// specifics (window management, audio output device, input source) live in the
/// respective front-ends; the core only consumes already-decoded input button
/// codes and produces a framebuffer plus audio samples.
pub struct Emulator {
    pub filename: PathBuf,
    pub reader: Native32Reader,
    pub sprites: SpriteSystem,
    pub frame_player: FramePlayer,
    pub vm: ActionVM,
    pub audio: AudioEngine,
    pub renderer: Renderer,
    pub input: InputHandler,
    pub save_manager: SaveManager,
    pub content_loader: ContentLoader,
    pub cur_frame_objects: Vec<FrameObject>,
    pub tick_count: u64,
    pub time_ms: u32,
    /// MPEG-1 cutscene videos queued by SSL_PlayNext, played before the next
    /// SSL content loads.
    pub pending_videos: Vec<String>,
    /// Active cutscene player, if a video is currently playing.
    pub video_player: Option<crate::mpeg::VideoPlayer>,
}

impl Emulator {
    /// Create a new emulator from a game file path.
    pub fn from_path(path: PathBuf, volume: u32) -> Result<Self> {
        let data = std::fs::read(&path)
            .with_context(|| format!("Failed to read game file: {}", path.display()))?;

        let mut reader = Native32Reader::new(data);
        reader.init().context("Failed to initialize game file")?;

        let resolution = reader.resolution;
        let colorspace = reader.colorspace;

        let save_manager = SaveManager::new(&path);

        Ok(Self {
            filename: path,
            reader,
            sprites: SpriteSystem::new(),
            frame_player: FramePlayer::new(),
            vm: ActionVM::new(),
            audio: AudioEngine::new(colorspace, volume),
            renderer: Renderer::new(resolution.0, resolution.1),
            input: InputHandler::new(),
            save_manager,
            content_loader: ContentLoader::new(),
            cur_frame_objects: Vec::new(),
            tick_count: 0,
            time_ms: 0,
            pending_videos: Vec::new(),
            video_player: None,
        })
    }

    /// Get the game resolution.
    pub fn get_resolution(&self) -> (u32, u32) {
        self.reader.resolution
    }

    /// Get the audio sample rate based on colorspace.
    pub fn get_audio_sample_rate(&self) -> f64 {
        match self.reader.colorspace {
            crate::file_loader::Colorspace::YUV => 11025.0,
            crate::file_loader::Colorspace::ARGB => 22050.0,
        }
    }

    /// Set button state from libretro input.
    pub fn set_buttons(&mut self, keycodes: &[u16]) {
        self.input.set_buttons(keycodes);
    }

    /// Load a new frame and set up sprites.
    pub fn load_frame(&mut self, frame: u32) {
        if let Some(objects) = self.reader.get_frame(frame) {
            self.cur_frame_objects = objects.clone();
            self.sprites.update_for_frame(&objects);
        } else {
            log::warn!("Failed to load frame {}", frame);
            self.cur_frame_objects.clear();
        }
    }

    /// Run one tick of the emulation (called at 30fps).
    pub fn tick(&mut self) {
        self.tick_count += 1;

        // While a cutscene is playing (or queued), drive video playback instead
        // of the normal timeline.
        if self.is_cutscene_active() {
            self.cutscene_tick();
            return;
        }

        // Advance main timeline
        self.frame_player.tick();

        // Handle pending frame switch
        if let Some(next) = self.frame_player.take_next_frame() {
            self.frame_player.current_frame = next;
            self.load_frame(next);

            // Execute frame actions first
            let frame_action_indices: Vec<u32> = self
                .cur_frame_objects
                .iter()
                .filter(|o| o.obj_type == ObjectType::Action)
                .map(|o| o.index as u32)
                .collect();
            let self_ptr = self as *mut Emulator;
            unsafe {
                for action_idx in frame_action_indices {
                    (*self_ptr)
                        .vm
                        .run(&mut (*self_ptr).reader, &mut *self_ptr, action_idx, "");
                }
            }
        }

        // Advance movie frames
        let tick = self.tick_count;

        // Determine which movies need frame advancement
        let movie_advancements: Vec<(String, isize)> = {
            let mut advancements = Vec::new();
            for (name, movie) in self.sprites.iter_mut() {
                if !movie.playing || movie.next_frame.is_some() || movie.sound_channel.is_some() {
                    continue;
                }
                let movie_frames = self.reader.get_movie(movie.movie);
                if movie_frames.is_empty() {
                    continue;
                }
                if tick.is_multiple_of(2) {
                    if movie.frame < movie_frames.len() - 1 {
                        advancements.push((name.clone(), movie.frame as isize + 1));
                    } else {
                        advancements.push((name.clone(), 0)); // loop
                    }
                }
            }
            advancements
        };

        // Apply advancements
        for (name, next_frame) in movie_advancements {
            if let Some(movie) = self.sprites.get_mut(&name) {
                movie.next_frame = Some(next_frame);
            }
        }

        // Process pending movie frame switches
        let names: Vec<String> = self.sprites.sprites.keys().cloned().collect();
        for name in &names {
            let next = self.sprites.get(name).and_then(|m| m.next_frame);
            if let Some(next_frame) = next {
                if let Some(movie) = self.sprites.get_mut(name) {
                    if next_frame == -1 {
                        movie.frame = 0;
                    } else if (next_frame as usize) < self.reader.get_movie(movie.movie).len() {
                        movie.frame = next_frame as usize;
                    }
                    movie.next_frame = None;
                }

                let frame_data = {
                    let movie = match self.sprites.get(name) {
                        Some(m) => m,
                        None => continue,
                    };
                    let movie_frames = self.reader.get_movie(movie.movie);
                    if movie.frame < movie_frames.len() {
                        let mf = movie_frames[movie.frame];
                        Some((mf.sound, mf.action))
                    } else {
                        None
                    }
                };

                if let Some((sound, action)) = frame_data {
                    // Play sound if present and track it on the movie
                    if sound != 0 && self.audio.play_sound(&mut self.reader, sound, name) {
                        if let Some(movie) = self.sprites.get_mut(name) {
                            movie.sound_channel = Some(0);
                        }
                    }

                    // Execute movie frame action
                    if action != 0 {
                        let action_idx = action as u32;
                        let self_ptr = self as *mut Emulator;
                        unsafe {
                            (*self_ptr).vm.run(
                                &mut (*self_ptr).reader,
                                &mut *self_ptr,
                                action_idx,
                                name,
                            );
                        }
                    }
                }
            }
        }

        // Handle ended sounds
        if !self.audio.is_playing() {
            if let Some(owner) = self.audio.music_owner.clone() {
                if let Some(movie) = self.sprites.get_mut(&owner) {
                    movie.sound_channel = None;
                }
                self.audio.music_owner = None;
            }
        }

        // Handle content switching (SSL_PlayNext)
        if self.content_loader.has_pending() && !self.is_cutscene_active() {
            if let Some(filename) = self.content_loader.take_pending() {
                if let Err(e) = self.switch_content(&filename) {
                    log::error!("Failed to switch content: {}", e);
                }
            }
        }
    }

    /// Whether a cutscene video is currently playing or queued.
    pub fn is_cutscene_active(&self) -> bool {
        self.video_player.is_some() || !self.pending_videos.is_empty()
    }

    /// Queue MPEG-1 cutscene videos (SSL_PlayNext pre-content) for playback.
    pub fn queue_videos(&mut self, names: &[&str]) {
        for name in names {
            let normalized = normalize_content_path(name);
            if !normalized.is_empty() {
                self.pending_videos.push(normalized);
            }
        }
    }

    /// Skip the current (and any queued) cutscene videos.
    pub fn skip_cutscene(&mut self) {
        self.pending_videos.clear();
        self.video_player = None;
        self.audio.stop_all();
    }

    /// Advance cutscene playback by one 30fps tick.
    fn cutscene_tick(&mut self) {
        // Load the next queued video if none is playing.
        if self.video_player.is_none() {
            if self.pending_videos.is_empty() {
                return;
            }
            let name = self.pending_videos.remove(0);
            if !self.start_video(&name) {
                // Could not start this one; the next tick tries the next entry
                // (or finishes the cutscene and loads the SSL content).
                return;
            }
        }

        let (w, h) = (self.renderer.width as usize, self.renderer.height as usize);
        if let Some(player) = self.video_player.as_mut() {
            player.advance_and_render(1.0 / 30.0, &mut self.renderer.buffer, w, h);
            if player.is_finished() {
                self.video_player = None;
                self.audio.stop_all();
            }
        }
    }

    /// Resolve, load and start a cutscene video. Returns false on any failure
    /// (missing file, bad data) so the caller can fall through.
    fn start_video(&mut self, name: &str) -> bool {
        let path = match ContentLoader::find_content_file(&self.filename, name) {
            Some(p) => p,
            None => {
                log::warn!("Cutscene video not found: {}", name);
                return false;
            }
        };
        log::info!("Playing cutscene: {}", path.display());

        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                log::warn!("Failed to read cutscene {}: {}", path.display(), e);
                return false;
            }
        };

        let streams = crate::mpeg::demux_all(data);
        if streams.video.is_empty() {
            log::warn!("Cutscene has no video stream: {}", path.display());
            return false;
        }

        // Decode and start the audio track up front (if present).
        if !streams.audio.is_empty() {
            let mut audio = crate::mpeg::Audio::new(streams.audio);
            if audio.has_header() {
                let rate = audio.samplerate();
                let mut pcm: Vec<f32> = Vec::new();
                while let Some(s) = audio.decode() {
                    pcm.extend_from_slice(&s.interleaved);
                }
                self.audio.stop_all();
                self.audio.play_pcm_stream(pcm, 2, rate);
            }
        }

        match crate::mpeg::VideoPlayer::new(streams.video) {
            Some(player) => {
                self.video_player = Some(player);
                true
            }
            None => {
                log::warn!("Cutscene video has no sequence header: {}", path.display());
                false
            }
        }
    }

    /// Handle button presses from libretro input.
    pub fn handle_buttons(&mut self) {
        let pressed = self.input.get_pressed_buttons();
        let button_events: Vec<(u32, Vec<(u16, u16)>)> = self
            .cur_frame_objects
            .iter()
            .filter(|o| o.obj_type == ObjectType::Button)
            .map(|o| {
                (
                    o.index as u32,
                    self.reader.get_button_events(o.index as u32),
                )
            })
            .collect();

        for (_button_idx, events) in &button_events {
            for (keycode, action_idx) in events {
                if pressed.contains(keycode) {
                    let self_ptr = self as *mut Emulator;
                    unsafe {
                        (*self_ptr).vm.run(
                            &mut (*self_ptr).reader,
                            &mut *self_ptr,
                            *action_idx as u32,
                            "",
                        );
                    }
                }
            }
        }
    }

    /// Draw the current frame to the renderer's buffer.
    pub fn draw(&mut self) {
        // During a cutscene the framebuffer is produced by the video player.
        if self.is_cutscene_active() {
            return;
        }
        self.renderer
            .draw_frame(&mut self.reader, &self.sprites, &self.cur_frame_objects);
    }

    /// Switch to a new content file (for SSL_PlayNext).
    fn switch_content(&mut self, filename: &str) -> anyhow::Result<()> {
        let fullpath = ContentLoader::find_content_file(&self.filename, filename)
            .ok_or_else(|| anyhow::anyhow!("Content file not found: {}", filename))?;

        log::info!("Loading content: {}", fullpath.display());

        // Stop all sounds
        self.audio.stop_all();

        // Reset state
        self.tick_count = 0;
        self.time_ms = 0;
        self.sprites = SpriteSystem::new();
        self.frame_player = FramePlayer::new();
        self.vm = ActionVM::new();
        self.content_loader = ContentLoader::new();

        // Load new file
        let data = std::fs::read(&fullpath)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", fullpath.display(), e))?;
        self.save_manager = SaveManager::new(&fullpath);
        self.filename = fullpath;
        self.reader = Native32Reader::new(data);
        self.reader.init()?;
        self.audio = AudioEngine::new(self.reader.colorspace, (self.audio.volume * 100.0) as u32);
        self.cur_frame_objects.clear();

        Ok(())
    }

    /// Get the framebuffer as a slice of u32 (XRGB8888).
    pub fn get_framebuffer(&self) -> &[u32] {
        &self.renderer.buffer
    }

    /// Get pending audio samples as i16 interleaved stereo.
    pub fn get_pending_audio_samples(&mut self) -> Vec<i16> {
        self.audio.get_pending_samples()
    }

    /// Reset the emulator state.
    pub fn reset(&mut self) {
        self.tick_count = 0;
        self.time_ms = 0;
        self.sprites = SpriteSystem::new();
        self.frame_player = FramePlayer::new();
        self.vm = ActionVM::new();
        self.audio.stop_all();
    }

    /// Get the size needed for serialization.
    ///
    /// Save states are not implemented yet. Returning 0 tells the libretro
    /// frontend that this core does not support save states, so it will not
    /// expose a (silently broken) save/load state action to the user.
    pub fn serialize_size(&self) -> usize {
        0
    }

    /// Serialize the emulator state to a buffer.
    ///
    /// Not implemented yet: returns an error so the frontend does not believe a
    /// save state was successfully captured.
    pub fn serialize(&self, _buffer: &mut [u8]) -> Result<()> {
        anyhow::bail!("save states are not supported")
    }

    /// Deserialize the emulator state from a buffer.
    ///
    /// Not implemented yet: returns an error so the frontend does not believe a
    /// save state was successfully restored.
    pub fn deserialize(&mut self, _buffer: &[u8]) -> Result<()> {
        anyhow::bail!("save states are not supported")
    }
}

/// VmHost trait implementation for the Emulator.
impl VmHost for Emulator {
    fn stop(&mut self, target: &str) {
        if target.is_empty() {
            self.frame_player.playing = false;
        } else if let Some(movie) = self.sprites.get_mut(target) {
            movie.playing = false;
        }
    }

    fn play(&mut self, target: &str) {
        if target.is_empty() {
            self.frame_player.playing = true;
        } else if let Some(movie) = self.sprites.get_mut(target) {
            movie.playing = true;
        }
    }

    fn get_frame(&mut self, target: &str) -> u32 {
        if target.is_empty() {
            self.frame_player.current_frame
        } else if let Some(movie) = self.sprites.get(target) {
            movie.frame as u32 + 1
        } else {
            0
        }
    }

    fn goto_frame(&mut self, target: &str, frame: u32, playing: bool) {
        if target.is_empty() {
            self.frame_player.goto(frame, playing);
        } else if let Some(movie) = self.sprites.get_mut(target) {
            movie.next_frame = Some(frame as isize - 1);
            movie.playing = playing;
        }
    }

    fn stop_sounds(&mut self, target: &str) {
        if target.is_empty() {
            self.audio.stop_all();
            for (_, movie) in self.sprites.iter_mut() {
                movie.sound_channel = None;
            }
        } else {
            self.audio.stop_for_movie(target);
            if let Some(movie) = self.sprites.get_mut(target) {
                movie.sound_channel = None;
            }
        }
    }

    fn set_property(&mut self, target: &str, prop: ActionProp, value: &str) {
        if let Some(movie) = self.sprites.get_mut(target) {
            match prop {
                ActionProp::X => movie.x = str_to_float(value) as i16,
                ActionProp::Y => movie.y = str_to_float(value) as i16,
                ActionProp::Visible => movie.visible = str_to_float(value) != 0.0,
                ActionProp::CurrentFrame => {
                    movie.next_frame = Some(str_to_float(value) as isize);
                }
                ActionProp::Name => {
                    let old_name = target.to_string();
                    if let Some(state) = self.sprites.remove(&old_name) {
                        self.sprites.insert(value.to_string(), state);
                    }
                }
                _ => {
                    log::warn!("Unsupported SetProperty({:?})", prop);
                }
            }
        }
    }

    fn get_property(&mut self, target: &str, prop: ActionProp) -> String {
        if let Some(movie) = self.sprites.get(target) {
            match prop {
                ActionProp::X => movie.x.to_string(),
                ActionProp::Y => movie.y.to_string(),
                ActionProp::Visible => (movie.visible as i32).to_string(),
                ActionProp::CurrentFrame => {
                    if let Some(nf) = movie.next_frame {
                        (nf.max(0) as u32 + 1).to_string()
                    } else if movie.playing {
                        (movie.frame as u32 + 2).to_string()
                    } else {
                        (movie.frame as u32 + 1).to_string()
                    }
                }
                ActionProp::TotalFrames => {
                    let frames = self.reader.get_movie(movie.movie);
                    frames.len().to_string()
                }
                ActionProp::Name => target.to_string(),
                _ => "0".to_string(),
            }
        } else {
            "0".to_string()
        }
    }

    fn clone_sprite(&mut self, src: &str, dest: &str, depth: i32) {
        if let Some(orig) = self.sprites.get(src).cloned() {
            let mut new_state = MovieState::new(orig.movie, orig.x, orig.y, depth as u16);
            new_state.frame = 0;
            new_state.visible = true;
            new_state.playing = orig.playing;
            new_state.cloned = true;
            new_state.next_frame = Some(orig.frame as isize);
            self.sprites.insert(dest.to_string(), new_state);
        }
    }

    fn remove_sprite(&mut self, name: &str) {
        if let Some(movie) = self.sprites.remove(name) {
            if movie.sound_channel.is_some() {
                self.audio.stop_for_movie(name);
            }
        }
    }

    fn call(&mut self, frame: u32) {
        if let Some(objects) = self.reader.get_frame(frame) {
            let action_indices: Vec<u32> = objects
                .iter()
                .filter(|o| o.obj_type == ObjectType::Action)
                .map(|o| o.index as u32)
                .collect();
            let self_ptr = self as *mut Emulator;
            unsafe {
                for action_idx in action_indices {
                    (*self_ptr)
                        .vm
                        .run(&mut (*self_ptr).reader, &mut *self_ptr, action_idx, "");
                }
            }
        }
    }

    fn get_time(&self) -> u32 {
        self.time_ms
    }

    fn get_url(&mut self, url: &str, target: &str) {
        let parts: Vec<&str> = target.split('+').collect();
        if parts.len() < 2 {
            log::warn!("Invalid GetUrl2 target: {}", target);
            return;
        }

        match parts[1] {
            "SSL_PlayNext" => {
                log::info!("SSL_PlayNext({}, {})", url, target);
                let url_parts: Vec<&str> = url.split('+').collect();
                // All parts except the last are MPEG-1 pre-content (logo /
                // cutscene videos) to play before loading the final SSL content.
                if url_parts.len() > 1 {
                    let pre = &url_parts[..url_parts.len() - 1];
                    self.queue_videos(pre);
                }
                if let Some(last) = url_parts.last() {
                    self.content_loader.queue_load(last);
                }
            }
            "SSL_PlayPlan" => {
                log::info!("Ignoring SSL_PlayPlan('{}')", url);
            }
            "SSL_PlayProg" => {
                log::info!("Ignoring SSL_PlayProg('{}')", url);
            }
            "SSL_GetSSLData" => {
                log::info!("SSL_GetSSLData");
                if parts.len() >= 3 {
                    let success_var = parts[2];
                    match self.save_manager.load() {
                        Some(data) => {
                            self.vm.vars.insert(url.to_lowercase(), data);
                            self.vm
                                .vars
                                .insert(success_var.to_lowercase(), "S".to_string());
                        }
                        None => {
                            self.vm
                                .vars
                                .insert(success_var.to_lowercase(), "N".to_string());
                        }
                    }
                }
            }
            "SSL_SaveSSLData" => {
                log::info!("SSL_SaveSSLData");
                if parts.len() >= 3 {
                    let success_var = parts[2];
                    if self.save_manager.save(url) {
                        self.vm
                            .vars
                            .insert(success_var.to_lowercase(), "S".to_string());
                    }
                }
            }
            "NAV_ScreenMove" => {
                let coords: Vec<&str> = url.split('+').collect();
                if coords.len() >= 2 {
                    let dx = coords[0].parse::<i32>().unwrap_or(0);
                    let dy = coords[1].parse::<i32>().unwrap_or(0);
                    self.renderer.screen_x = dx;
                    self.renderer.screen_y = dy;
                }
            }
            "GetFileNum" | "GetContext" => {
                log::debug!("Ignoring GetUrl2('{}', '{}')", url, target);
            }
            _ => {
                log::warn!("Unhandled GetUrl2('{}', '{}')", url, target);
            }
        }
    }

    fn run_frame_actions(&mut self, frame: u32) {
        if let Some(objects) = self.reader.get_frame(frame) {
            let action_indices: Vec<u32> = objects
                .iter()
                .filter(|o| o.obj_type == ObjectType::Action)
                .map(|o| o.index as u32)
                .collect();
            let self_ptr = self as *mut Emulator;
            unsafe {
                for action_idx in action_indices {
                    (*self_ptr)
                        .vm
                        .run(&mut (*self_ptr).reader, &mut *self_ptr, action_idx, "");
                }
            }
        }
    }
}

fn str_to_float(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    s.parse::<f64>().unwrap_or(0.0)
}

/// Normalize a Native32 content path: split on '/', trim each component
/// (games pad directory names with trailing spaces), drop empties, rejoin.
fn normalize_content_path(filename: &str) -> String {
    filename
        .split('/')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}
