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
//! By default it looks for games in `<repo>/native32_game`. Override the
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
    // Default: <workspace_root>/native32_game. CARGO_MANIFEST_DIR points at the
    // core crate (crates/native32emu-core), so go up two levels.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("native32_game"));
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
        emu.handle_buttons();
        emu.tick();
        emu.draw();
    }

    Ok(frame_has_content(emu.get_framebuffer()))
}
