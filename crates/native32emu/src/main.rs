// Native32 Emulator - standalone front-end (minifb window + CLI).
// This binary reuses the shared emulator core from the `native32emu` library
// crate and only adds the platform layer: window management, command-line
// argument parsing, keyboard input and the optional on-screen gamepad overlay.
// It is only compiled when the "standalone" feature is enabled.

mod standalone;

use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use native32emu_core::emulator::Emulator;

use crate::standalone::cli::Cli;
use crate::standalone::gamepad_overlay::GamepadOverlay;

// Platform-specific screen resolution APIs for fullscreen mode

#[cfg(target_os = "windows")]
mod screen {
    extern "system" {
        fn GetSystemMetrics(nIndex: i32) -> i32;
    }
    const SM_CXSCREEN: i32 = 0;
    const SM_CYSCREEN: i32 = 1;

    pub fn get_screen_size() -> (usize, usize) {
        unsafe {
            (
                GetSystemMetrics(SM_CXSCREEN) as usize,
                GetSystemMetrics(SM_CYSCREEN) as usize,
            )
        }
    }
}

#[cfg(target_os = "linux")]
mod screen {
    // X11 FFI for querying display resolution
    type Display = *mut core::ffi::c_void;
    type Window = u64;

    #[link(name = "X11")]
    extern "system" {
        fn XOpenDisplay(display_name: *const u8) -> Display;
        fn XCloseDisplay(display: Display) -> i32;
        fn XDefaultRootWindow(display: Display) -> Window;
        fn XDisplayWidth(display: Display, screen_number: i32) -> i32;
        fn XDisplayHeight(display: Display, screen_number: i32) -> i32;
    }

    pub fn get_screen_size() -> (usize, usize) {
        unsafe {
            let display = XOpenDisplay(std::ptr::null());
            if display.is_null() {
                return (800, 600);
            }
            let w = XDisplayWidth(display, 0) as usize;
            let h = XDisplayHeight(display, 0) as usize;
            let _ = XDefaultRootWindow(display);
            let _ = XCloseDisplay(display);
            (w, h)
        }
    }
}

#[cfg(target_os = "macos")]
mod screen {
    // macOS Core Graphics FFI for querying main display resolution
    type CGDirectDisplayID = u32;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGMainDisplayID() -> CGDirectDisplayID;
        fn CGDisplayPixelsWide(display: CGDirectDisplayID) -> usize;
        fn CGDisplayPixelsHigh(display: CGDirectDisplayID) -> usize;
    }

    pub fn get_screen_size() -> (usize, usize) {
        unsafe {
            let display = CGMainDisplayID();
            (CGDisplayPixelsWide(display), CGDisplayPixelsHigh(display))
        }
    }
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

    log::info!("Loading game: {}", game_path.display());

    // Create the shared emulator core
    let mut emu = Emulator::from_path(game_path, cli.volume)?;

    // Apply key remappings (standalone-only feature)
    let key_remappings = cli.parse_key_remappings();
    emu.input.remap(&key_remappings);

    // Apply typematic key-repeat timing (matches the hardware keypad driver).
    emu.input
        .set_repeat_timing(cli.repeat_delay, cli.repeat_period);

    let resolution = emu.reader.resolution;
    let display_width = resolution.0 * cli.scale;
    let display_height = resolution.1 * cli.scale;
    let (buf_width, buf_height) = (resolution.0, resolution.1);

    // For fullscreen, get screen resolution before creating the window
    // so minifb creates the window at the correct size from the start.
    let (window_width, window_height) = if cli.fullscreen {
        screen::get_screen_size()
    } else {
        (display_width as usize, display_height as usize)
    };

    // Create window
    let window_opts = minifb::WindowOptions {
        resize: !cli.fullscreen,
        borderless: cli.fullscreen,
        scale_mode: minifb::ScaleMode::AspectRatioStretch,
        ..Default::default()
    };

    let mut window = minifb::Window::new(
        "Native32 Emulator",
        window_width,
        window_height,
        window_opts,
    )
    .context("Failed to create window")?;

    // Apply fullscreen settings
    if cli.fullscreen {
        window.topmost(true);
        window.set_position(0, 0);
    }

    // Limit to 30fps
    window.set_target_fps(30);

    let frame_duration = Duration::from_millis(1000 / 30);

    // Main emulation loop
    let mut frame_count: u32 = 0;
    let screenshot_path = cli.screenshot.clone();

    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        let frame_start = Instant::now();

        // Feed keyboard state into the shared core, then run button actions
        let pressed = emu.input.get_pressed_keycodes(&window);
        emu.set_buttons(&pressed);
        if emu.is_cutscene_active() {
            // Allow skipping logo/cutscene videos with the A or B button.
            if pressed.contains(&0x4000) || pressed.contains(&0x8800) {
                emu.skip_cutscene();
            }
        } else {
            emu.handle_buttons();
        }

        // Tick emulation (handles content switching internally)
        emu.tick();

        // Draw frame
        emu.draw();

        // Draw gamepad overlay if enabled
        if cli.show_gamepad {
            let pressed_set: std::collections::HashSet<u16> = pressed.iter().copied().collect();
            GamepadOverlay::draw(
                &mut emu.renderer.buffer,
                resolution.0,
                resolution.1,
                cli.scale,
                &pressed_set,
            );
        }

        // Update window
        window
            .update_with_buffer(
                &emu.renderer.buffer,
                buf_width as usize,
                buf_height as usize,
            )
            .context("Failed to update display")?;

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
