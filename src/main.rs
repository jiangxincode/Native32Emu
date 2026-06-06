// Native32 Emulator - main entry point and emulation loop.

#![allow(dead_code)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::manual_memcpy)]
#![allow(clippy::needless_range_loop)]

mod action_vm;
mod actions;
mod audio_engine;
mod cli;
mod content_loader;
mod des_constants;
mod error;
mod file_loader;
mod frame_player;
mod gamepad_overlay;
mod header_decryptor;
mod image_decoder;
mod input_handler;
mod renderer;
mod save_manager;
mod sprite_system;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use file_loader::{FrameObject, Native32Reader, ObjectType};
use sprite_system::{MovieState, SpriteSystem};

use crate::action_vm::{ActionProp, ActionVM, VmHost};
use crate::audio_engine::AudioEngine;
use crate::cli::Cli;
use crate::content_loader::ContentLoader;
use crate::frame_player::FramePlayer;
use crate::gamepad_overlay::GamepadOverlay;
use crate::input_handler::InputHandler;
use crate::renderer::Renderer;
use crate::save_manager::SaveManager;

/// The main emulator state.
struct Emulator {
    filename: PathBuf,
    reader: Native32Reader,
    sprites: SpriteSystem,
    frame_player: FramePlayer,
    vm: ActionVM,
    audio: AudioEngine,
    renderer: Renderer,
    input: InputHandler,
    save_manager: SaveManager,
    content_loader: ContentLoader,
    cur_frame_objects: Vec<FrameObject>,
    tick_count: u64,
    time_ms: u32,
    scale: u32,
    show_gamepad: bool,
    _debug: bool,
}

impl Emulator {
    fn new(
        filename: PathBuf,
        data: Vec<u8>,
        scale: u32,
        volume: u32,
        debug: bool,
        show_gamepad: bool,
        key_remappings: &[(u16, minifb::Key)],
    ) -> Result<Self> {
        let mut reader = Native32Reader::new(data);
        reader.init().context("Failed to initialize game file")?;

        let resolution = reader.resolution;
        let colorspace = reader.colorspace;

        let display_width = resolution.0 * scale;
        let display_height = resolution.1 * scale;

        let mut input = InputHandler::new();
        input.remap(key_remappings);

        let save_manager = SaveManager::new(&filename);

        Ok(Self {
            filename,
            reader,
            sprites: SpriteSystem::new(),
            frame_player: FramePlayer::new(),
            vm: ActionVM::new(),
            audio: AudioEngine::new(colorspace, volume),
            renderer: Renderer::new(display_width, display_height),
            input,
            save_manager,
            content_loader: ContentLoader::new(),
            cur_frame_objects: Vec::new(),
            tick_count: 0,
            time_ms: 0,
            scale,
            show_gamepad,
            _debug: debug,
        })
    }

    /// Load a new frame and set up sprites.
    fn load_frame(&mut self, frame: u32) {
        if let Some(objects) = self.reader.get_frame(frame) {
            self.cur_frame_objects = objects.clone();
            self.sprites.update_for_frame(&objects);
        } else {
            log::warn!("Failed to load frame {}", frame);
            self.cur_frame_objects.clear();
        }
    }

    /// Run one tick of the emulation (called at 30fps).
    fn tick(&mut self) {
        self.tick_count += 1;

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

            // Then execute movie actions
            let movie_action_list: Vec<(String, u32)> = {
                let mut list = Vec::new();
                let names: Vec<String> = self.sprites.sprites.keys().cloned().collect();
                for name in &names {
                    if let Some(movie) = self.sprites.get(name) {
                        let movie_frames = self.reader.get_movie(movie.movie);
                        if movie.frame < movie_frames.len() {
                            let mf = &movie_frames[movie.frame];
                            if mf.action != 0 {
                                list.push((name.clone(), mf.action as u32));
                            }
                        }
                    }
                }
                list
            };

            let self_ptr = self as *mut Emulator;
            unsafe {
                for (name, action_idx) in movie_action_list {
                    (*self_ptr)
                        .vm
                        .run(&mut (*self_ptr).reader, &mut *self_ptr, action_idx, &name);
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
        // Collect names first to avoid borrow conflicts
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

                // Collect sound/action data to avoid borrow conflicts
                // (self.audio.play_sound needs &mut self.reader and &mut self.audio,
                //  while we also need &mut self.sprites for sound_channel)
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
    }

    /// Handle button presses from the window.
    fn handle_buttons(&mut self, window: &minifb::Window) {
        let pressed = self.input.get_pressed_keycodes(window);
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

        for (_button_idx, events) in button_events {
            for (keycode, action_idx) in &events {
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
    fn draw(&mut self) {
        self.renderer
            .draw_frame(&mut self.reader, &self.sprites, &self.cur_frame_objects);
    }

    /// Draw the virtual gamepad overlay if enabled.
    fn draw_gamepad_overlay(&mut self, window: &minifb::Window) {
        if !self.show_gamepad {
            return;
        }
        let pressed: std::collections::HashSet<u16> = self
            .input
            .get_pressed_keycodes(window)
            .into_iter()
            .collect();
        let resolution = self.reader.resolution;
        let w = resolution.0 * self.scale;
        let h = resolution.1 * self.scale;
        GamepadOverlay::draw(&mut self.renderer.buffer, w, h, self.scale, &pressed);
    }

    /// Switch to new content (SSL multi-file).
    fn switch_content(&mut self, filename: &str) -> Result<()> {
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
            .with_context(|| format!("Failed to read {}", fullpath.display()))?;
        self.filename = fullpath;
        self.reader = Native32Reader::new(data);
        self.reader.init()?;
        self.audio = AudioEngine::new(self.reader.colorspace, (self.audio.volume * 100.0) as u32);
        self.save_manager = SaveManager::new(&self.filename);

        Ok(())
    }
}

/// Implement the VmHost trait for the Emulator so the Action VM can control it.
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
            // Stop all sounds and clear all sound_channel flags
            self.audio.stop_all();
            for (_, movie) in self.sprites.iter_mut() {
                movie.sound_channel = None;
            }
        } else {
            // Stop only the sound owned by this movie
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
                    // When a frame change is pending (e.g. set by GotoFrame2 within
                    // the same tick), report the pending target frame. Otherwise the
                    // stale current frame would be returned, which breaks games that
                    // read _currentframe right after redirecting a movie (e.g. the
                    // attack-animation logic in EBBLADE / EMETAL).
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
                for skipped in &url_parts[..url_parts.len().saturating_sub(1)] {
                    log::info!("Ignoring SSL_PlayNext pre-content: {}", skipped);
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
                // These are game-specific queries that we can safely ignore
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

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let cli = Cli::parse_args();

    // Validate game path
    let game_path = match &cli.game_path {
        Some(p) => p.clone(),
        None => {
            eprintln!("Error: No game file specified.");
            eprintln!("Usage: native32-emu [OPTIONS] <GAME_PATH>");
            eprintln!("Run with --help for more information.");
            std::process::exit(1);
        }
    };

    if !game_path.exists() {
        eprintln!("Error: Game file not found: {}", game_path.display());
        std::process::exit(1);
    }

    // Read game file
    let data = std::fs::read(&game_path)
        .with_context(|| format!("Failed to read game file: {}", game_path.display()))?;

    log::info!(
        "Loading game: {} ({} bytes)",
        game_path.display(),
        data.len()
    );

    // Parse key remappings
    let key_remappings = cli.parse_key_remappings();

    // Create emulator
    let mut emu = Emulator::new(
        game_path,
        data,
        cli.scale,
        cli.volume,
        cli.debug,
        cli.show_gamepad,
        &key_remappings,
    )?;

    let resolution = emu.reader.resolution;
    let display_width = resolution.0 * cli.scale;
    let display_height = resolution.1 * cli.scale;

    // Create window
    let window_opts = minifb::WindowOptions {
        resize: true,
        scale_mode: minifb::ScaleMode::AspectRatioStretch,
        ..Default::default()
    };

    let mut window = minifb::Window::new(
        "Native32 Emulator",
        display_width as usize,
        display_height as usize,
        window_opts,
    )
    .context("Failed to create window")?;

    // Limit to 30fps
    window.set_target_fps(30);

    let frame_duration = Duration::from_millis(1000 / 30);

    // Main emulation loop
    let mut frame_count: u32 = 0;
    let screenshot_path = cli.screenshot.clone();

    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        let frame_start = Instant::now();

        // Handle button input
        emu.handle_buttons(&window);

        // Tick emulation
        emu.tick();

        // Draw frame
        emu.draw();

        // Draw gamepad overlay if enabled
        emu.draw_gamepad_overlay(&window);

        // Update window
        window
            .update_with_buffer(
                &emu.renderer.buffer,
                display_width as usize,
                display_height as usize,
            )
            .context("Failed to update display")?;

        // Handle content switching
        if emu.content_loader.has_pending() {
            if let Some(filename) = emu.content_loader.take_pending() {
                if let Err(e) = emu.switch_content(&filename) {
                    log::error!("Failed to switch content: {}", e);
                }
            }
        }

        // Update time
        emu.time_ms += 1000 / 30;
        frame_count += 1;

        // Take screenshot if requested
        if let Some(ref path) = screenshot_path {
            if frame_count >= cli.screenshot_frames {
                emu.renderer
                    .save_screenshot(path)
                    .context("Failed to save screenshot")?;
                log::info!("Screenshot saved to: {}", path.display());
                break;
            }
        }

        // Frame timing
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    log::info!("Emulator exited normally");
    Ok(())
}
