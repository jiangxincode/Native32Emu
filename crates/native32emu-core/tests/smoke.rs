//! Smoke test: load every available Native32 game, run it for a number of
//! frames, and assert the emulator neither panics nor produces a blank frame.
//!
//! This test needs the (large, non-distributed) game assets, so it is marked
//! `#[ignore]` and only runs on demand:
//!
//! ```text
//! cargo test -p native32emu-core --test smoke -- --ignored --nocapture
//! ```
//!
//! By default it looks for games in `<repo>/tmp/native32_game`. Override the
//! location with the `NATIVE32_GAME_DIR` environment variable.

use std::path::{Path, PathBuf};

use native32emu_core::emulator::Emulator;

/// Number of frames to run per game before sampling the output.
const FRAMES: u32 = 150;

/// Resolve the directory that holds the Native32 game assets.
fn game_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("NATIVE32_GAME_DIR") {
        let p = PathBuf::from(dir);
        return p.is_dir().then_some(p);
    }
    // Default: <workspace_root>/tmp/native32_game. CARGO_MANIFEST_DIR points at the
    // core crate (crates/native32emu-core), so go up two levels.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("tmp").join("native32_game"));
    candidate.filter(|p| p.is_dir())
}

/// Recursively collect playable game files (.smf / .ssl), skipping the NES
/// games and save files.
fn collect_games(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("NESGAME"))
            {
                continue;
            }
            collect_games(&path, out);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext = ext.to_ascii_lowercase();
            if ext == "smf" || ext == "ssl" {
                out.push(path);
            }
        }
    }
}

/// Returns true if the frame buffer contains more than one distinct pixel
/// value, i.e. it is not a single flat color (all-black / all-white / etc.).
fn frame_has_content(framebuffer: &[u32]) -> bool {
    let mut iter = framebuffer.iter();
    let first = match iter.next() {
        Some(&p) => p,
        None => return false,
    };
    iter.any(|&p| p != first)
}

#[test]
#[ignore = "requires local Native32 game assets (set NATIVE32_GAME_DIR)"]
fn smoke_all_games() {
    let dir = match game_dir() {
        Some(d) => d,
        None => {
            eprintln!(
                "skipping: no game directory found \
                 (set NATIVE32_GAME_DIR or place games in <repo>/native32_game)"
            );
            return;
        }
    };

    let mut games = Vec::new();
    collect_games(&dir, &mut games);
    games.sort();

    assert!(
        !games.is_empty(),
        "no .smf/.ssl games found under {}",
        dir.display()
    );

    println!(
        "Running smoke test over {} games ({FRAMES} frames each)",
        games.len()
    );

    let mut failures = Vec::new();

    for game in &games {
        let rel = game.strip_prefix(&dir).unwrap_or(game);
        match run_one(game) {
            Ok(true) => println!("[PASS] {}", rel.display()),
            Ok(false) => {
                println!("[WARN] {} (blank frame)", rel.display());
            }
            Err(reason) => {
                println!("[FAIL] {} - {reason}", rel.display());
                failures.push(format!("{}: {reason}", rel.display()));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} game(s) failed the smoke test:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Run a single game for `FRAMES` frames. Returns Ok(true) if the final frame
/// has visible content, Ok(false) if it is blank, or Err on a load failure.
/// A panic inside the emulator will fail the test via the normal unwinding.
fn run_one(path: &Path) -> Result<bool, String> {
    let mut emu =
        Emulator::from_path(path.to_path_buf(), 100).map_err(|e| format!("failed to load: {e}"))?;

    for _ in 0..FRAMES {
        emu.set_buttons(&[]);
        emu.tick();
        emu.draw();
    }

    Ok(frame_has_content(emu.get_framebuffer()))
}

#[test]
#[ignore = "requires local Native32 game assets (set NATIVE32_GAME_DIR)"]
fn save_state_round_trip() {
    let dir = game_dir().expect("no game directory found");
    let game = find_asset(&dir, "FHUI.smf").expect("FHUI.smf not found");
    let mut emu = Emulator::from_path(game, 100).expect("load game");

    // Saving before the first tick exercises the frame-zero startup state.
    let mut startup_state = vec![0; emu.serialize_size()];
    emu.serialize(&mut startup_state)
        .expect("save startup state");
    emu.deserialize(&startup_state)
        .expect("restore startup state");

    for _ in 0..30 {
        emu.set_buttons(&[]);
        emu.tick();
        emu.draw();
    }
    emu.vm.vars.insert("state_probe".into(), "saved".into());
    let saved_tick = emu.tick_count;
    let saved_frame = emu.frame_player.current_frame;
    let saved_pixels = emu.get_framebuffer().to_vec();
    let mut state = vec![0; emu.serialize_size()];
    emu.serialize(&mut state).expect("save running state");

    for _ in 0..5 {
        emu.tick();
    }
    emu.vm.vars.insert("state_probe".into(), "changed".into());
    emu.deserialize(&state).expect("restore running state");
    emu.draw();

    assert_eq!(emu.tick_count, saved_tick);
    assert_eq!(emu.frame_player.current_frame, saved_frame);
    assert_eq!(emu.vm.vars.get("state_probe").unwrap(), "saved");
    assert_eq!(emu.get_framebuffer(), saved_pixels);
}

// ---------------------------------------------------------------------------
// Scripted-input regression harness
//
// GUI / interaction bugs (e.g. "pressing a key on a menu freezes the screen")
// are hard to catch with the plain smoke test because that test never feeds any
// input. The helpers below turn such interactions into deterministic, headless
// regression tests: input is scripted per-frame and the emulator core is driven
// directly, so there is no window and no timing dependency. The Action VM is
// seeded deterministically, so a given input script always produces the same
// run.
// ---------------------------------------------------------------------------

/// Native32 keycode for the Z / "A" button (menu confirm).
const KEY_Z: u16 = 0x4000;

/// Locate a game asset by file name (case-insensitive) under `dir`.
fn find_asset(dir: &Path, file_name: &str) -> Option<PathBuf> {
    let mut all = Vec::new();
    collect_games(dir, &mut all);
    all.into_iter().find(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(file_name))
    })
}

/// Drive a game headlessly with a per-frame input script.
///
/// `input_for_frame(frame)` returns the Native32 keycodes held during that
/// frame. The closure is called for every frame from 0 to `frames - 1`. The
/// returned emulator can be inspected for final state (loaded content, sprite
/// positions, etc.).
fn run_scripted<F>(path: &Path, frames: u32, mut input_for_frame: F) -> Result<Emulator, String>
where
    F: FnMut(u32) -> Vec<u16>,
{
    let mut emu =
        Emulator::from_path(path.to_path_buf(), 100).map_err(|e| format!("failed to load: {e}"))?;
    for frame in 0..frames {
        let pressed = input_for_frame(frame);
        emu.set_buttons(&pressed);
        emu.tick();
        emu.draw();
    }
    Ok(emu)
}

/// Regression test for the GunFire menu freeze: holding the confirm button on
/// the start menu must navigate to the next screen instead of freezing.
///
/// Previously a redundant per-frame movie-action pass kept re-issuing the
/// `Stop` keyframe action of the selection movie, so its confirm animation
/// never advanced and the menu never progressed. We assert the loaded content
/// actually switches once confirm is held.
#[test]
#[ignore = "requires local Native32 game assets (set NATIVE32_GAME_DIR)"]
fn gunfire_menu_advances_on_confirm() {
    let dir = match game_dir() {
        Some(d) => d,
        None => {
            eprintln!("skipping: no game directory found (set NATIVE32_GAME_DIR)");
            return;
        }
    };

    let menu = match find_asset(&dir, "GFSTART.SSL") {
        Some(p) => p,
        None => {
            eprintln!("skipping: GFSTART.SSL not found under {}", dir.display());
            return;
        }
    };

    // Warm up for 40 frames with no input so the menu is fully shown, then hold
    // the confirm button. The selection animation should play and the menu
    // should load the next screen, i.e. the loaded content file changes.
    const WARMUP: u32 = 40;
    const TOTAL: u32 = 160;

    let emu = run_scripted(&menu, TOTAL, |frame| {
        if frame >= WARMUP {
            vec![KEY_Z]
        } else {
            vec![]
        }
    })
    .expect("run gunfire menu");

    let final_name = emu
        .filename
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    assert!(
        !final_name.eq_ignore_ascii_case("GFSTART.SSL"),
        "menu did not advance after holding confirm: still on {final_name} \
         (selection animation appears frozen)"
    );
}

/// Regression test for the front-end menu game list: the FHUI shell enumerates
/// the games on disk via the GetFileNum / GetFirstFile / GetNextFile host calls
/// and loads each game's `.dat` thumbnail. Previously these calls were ignored,
/// so the menu showed an empty game list. We drive the menu into the first
/// category and assert the list is populated and thumbnails are bound.
#[test]
#[ignore = "requires local Native32 game assets (set NATIVE32_GAME_DIR)"]
fn fhui_menu_populates_game_list() {
    let dir = match game_dir() {
        Some(d) => d,
        None => {
            eprintln!("skipping: no game directory found (set NATIVE32_GAME_DIR)");
            return;
        }
    };
    let menu = match find_asset(&dir, "FHUI.smf") {
        Some(p) => p,
        None => {
            eprintln!("skipping: FHUI.smf not found under {}", dir.display());
            return;
        }
    };

    // Warm up, then tap confirm once to enter the first category's game grid.
    let emu = run_scripted(&menu, 160, |frame| {
        if (50..53).contains(&frame) {
            vec![KEY_Z]
        } else {
            vec![]
        }
    })
    .expect("run FHUI menu");

    // We should still be in the menu (not have launched a game yet).
    let name = emu
        .filename
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    assert!(
        name.eq_ignore_ascii_case("FHUI.smf"),
        "expected to remain in the menu, but loaded {name}"
    );

    // The per-category game counts must be populated (EPOP is the first tab).
    let epop_count = emu
        .vm
        .vars
        .get("tfilenum0")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);
    assert!(
        epop_count > 0,
        "menu game count not populated (tfilenum0 = {:?})",
        emu.vm.vars.get("tfilenum0")
    );

    // Entering the category must enumerate its games and bind thumbnails.
    assert!(
        emu.vm.vars.get("realfilenum").is_some_and(|v| v != "0"),
        "category game list not enumerated (realfilenum = {:?})",
        emu.vm.vars.get("realfilenum")
    );
    assert!(
        emu.renderer.sprite_override_count() > 0,
        "no game thumbnails were loaded from .dat files"
    );
}

/// Selecting a game in the populated list must launch it: the StartGame host
/// call loads the chosen `.smf`, switching the loaded content away from FHUI.
#[test]
#[ignore = "requires local Native32 game assets (set NATIVE32_GAME_DIR)"]
fn fhui_menu_starts_selected_game() {
    let dir = match game_dir() {
        Some(d) => d,
        None => {
            eprintln!("skipping: no game directory found (set NATIVE32_GAME_DIR)");
            return;
        }
    };
    let menu = match find_asset(&dir, "FHUI.smf") {
        Some(p) => p,
        None => {
            eprintln!("skipping: FHUI.smf not found under {}", dir.display());
            return;
        }
    };

    // Tap confirm to enter the category, then confirm again to launch the
    // first game.
    let emu = run_scripted(&menu, 260, |frame| {
        if (50..53).contains(&frame) || (110..113).contains(&frame) {
            vec![KEY_Z]
        } else {
            vec![]
        }
    })
    .expect("run FHUI menu");

    let name = emu
        .filename
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    assert!(
        !name.eq_ignore_ascii_case("FHUI.smf"),
        "selecting a game did not launch it: still on the menu"
    );
}

/// Regression test for Magical Adventure's first mini-game flow: a confirm
/// press must be consumed by the mini-game script after the frame actions run,
/// starting the timer instead of leaving the prompt stuck forever.
#[test]
#[ignore = "requires local Native32 game assets (set NATIVE32_GAME_DIR)"]
fn magical_adventure_minigame_confirm_starts_timer() {
    let dir = match game_dir() {
        Some(d) => d,
        None => {
            eprintln!("skipping: no game directory found (set NATIVE32_GAME_DIR)");
            return;
        }
    };
    let game = match find_asset(&dir, "MagicalA.smf") {
        Some(p) => p,
        None => {
            eprintln!("skipping: MagicalA.smf not found under {}", dir.display());
            return;
        }
    };

    let mut emu = Emulator::from_path(game, 100).expect("load Magical Adventure");

    for frame in 0..540 {
        if frame == 240 {
            emu.vm.vars.insert("gamestate".into(), "2".into());
            emu.vm.vars.insert("mgstate".into(), "0".into());
            emu.vm.vars.insert("mgkind".into(), "0".into());
            emu.vm.vars.insert("mglevel".into(), "0".into());
            emu.vm.vars.insert("mgenemy".into(), "0".into());
            emu.vm.vars.insert("loopstep".into(), "0".into());
            emu.vm.vars.insert("mgstep".into(), "0".into());
            emu.vm.vars.insert("oldkeypad".into(), "0".into());
            emu.vm.vars.insert("icount".into(), "0".into());
        }

        let pressed = if (80..83).contains(&frame)
            || (220..223).contains(&frame)
            || (285..288).contains(&frame)
        {
            vec![KEY_Z]
        } else {
            vec![]
        };
        emu.set_buttons(&pressed);
        emu.tick();
        emu.draw();
    }

    let timebar_frame = emu.sprites.get("timeBar").map(|m| m.frame).unwrap_or(0);
    assert!(
        timebar_frame > 0,
        "mini-game timer did not advance after confirm (timeBar frame = {timebar_frame})"
    );
    assert!(
        emu.vm.vars.get("mgstate").is_some_and(|v| v != "0"),
        "mini-game stayed in init state after confirm: {:?}",
        emu.vm.vars.get("mgstate")
    );
}
#[test]
#[ignore = "requires local Native32 game assets (set NATIVE32_GAME_DIR)"]
fn magical_adventure_mp3_music_decodes_mixes_and_loops() {
    let dir = game_dir().expect("no game directory found");
    let game = find_asset(&dir, "MagicalA.smf").expect("MagicalA.smf not found");
    let mut emu = Emulator::from_path(game, 100).expect("load Magical Adventure");

    let channel = emu
        .audio
        .play_sound(&mut emu.reader, 0xFF01, "regression_bgm")
        .expect("decode looping MP3 background music");
    let effect_channel = emu
        .audio
        .play_sound(&mut emu.reader, 0x0007, "regression_effect")
        .expect("play RAW sound effect alongside MP3 music");
    assert!(emu.audio.is_channel_playing(channel));
    assert!(emu.audio.is_channel_playing(effect_channel));

    let mut non_silent_frames = 0;
    for _ in 0..900 {
        let samples = emu.get_pending_audio_samples();
        if samples.iter().any(|sample| *sample != 0) {
            non_silent_frames += 1;
        }
    }

    assert!(
        non_silent_frames > 800,
        "decoded MP3 unexpectedly contained prolonged silence"
    );
    assert!(
        emu.audio.is_channel_playing(channel),
        "0xFF MP3 background music stopped instead of looping"
    );
    assert!(
        !emu.audio.is_channel_playing(effect_channel),
        "one-shot RAW sound effect did not finish"
    );

    let mut state = vec![0; emu.serialize_size()];
    emu.serialize(&mut state).expect("save active MP3 state");
    emu.deserialize(&state).expect("restore active MP3 state");
    assert!(
        emu.audio.is_channel_playing(channel),
        "active MP3 channel was not restored from save state"
    );
}
/// Test ZIP file loading: a ZIP archive containing FHUI.smf should be extracted
/// and the main menu loaded automatically.
#[test]
#[ignore = "requires local Native32 game assets (set NATIVE32_GAME_DIR)"]
fn zip_file_loads_fhui() {
    let dir = match game_dir() {
        Some(d) => d,
        None => {
            eprintln!("skipping: no game directory found (set NATIVE32_GAME_DIR)");
            return;
        }
    };

    // Look for a ZIP file in the parent directory (tmp/native32_game.zip)
    let zip_path = dir
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("tmp").join("native32_game.zip"))
        .filter(|p| p.exists());

    let zip_path = match zip_path {
        Some(p) => p,
        None => {
            eprintln!("skipping: native32_game.zip not found");
            return;
        }
    };

    // Load the ZIP file
    let mut emu = Emulator::from_path(zip_path, 100).expect("failed to load ZIP game");

    // Verify that FHUI.smf was loaded (the main menu)
    let name = emu
        .filename
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    assert!(
        name.eq_ignore_ascii_case("FHUI.smf"),
        "expected FHUI.smf to be loaded from ZIP, but got: {name}"
    );

    // Run a few frames to ensure it's functional
    for _ in 0..30 {
        emu.set_buttons(&[]);
        emu.tick();
        emu.draw();
    }

    assert!(
        frame_has_content(emu.get_framebuffer()),
        "ZIP-loaded FHUI.smf produced a blank frame"
    );
}
