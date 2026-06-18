//! End-to-end cutscene playback test. Loads EMETAL.smf, which uses
//! SSL_PlayNext to play a logo video before the game, and verifies the
//! emulator enters cutscene playback and renders non-black video frames.
//!
//! Needs the (non-distributed) game assets, so it is `#[ignore]`d:
//!
//! ```text
//! cargo test -p native32emu-core --test cutscene -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use native32emu_core::emulator::Emulator;

fn game_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("NATIVE32_GAME_DIR") {
        let p = PathBuf::from(dir);
        return p.is_dir().then_some(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("tmp").join("native32_game"))
        .filter(|p| p.is_dir())
}

#[test]
#[ignore = "requires non-distributed game assets"]
fn emetal_plays_logo_cutscene() {
    let Some(root) = game_dir() else {
        eprintln!("No game assets found; skipping.");
        return;
    };
    let path = root.join("EPOP/EMETAL.smf");
    if !path.is_file() {
        eprintln!("EMETAL.smf not found; skipping.");
        return;
    }

    let mut emu = Emulator::from_path(path, 0).expect("load EMETAL.smf");

    let mut became_active = false;
    let mut non_black = false;
    // ~6 seconds at 30fps.
    for _ in 0..180 {
        emu.set_buttons(&[]);
        if !emu.is_cutscene_active() {
            emu.handle_buttons();
        }
        emu.tick();
        emu.draw();
        if emu.is_cutscene_active() {
            became_active = true;
            if emu.renderer.buffer.iter().any(|&p| (p & 0x00FF_FFFF) != 0) {
                non_black = true;
            }
        }
    }

    eprintln!("became_active={became_active}, non_black={non_black}");
    assert!(became_active, "EMETAL should enter cutscene playback");
    assert!(non_black, "cutscene should render non-black video frames");
}
