//! Integration test for the MPEG-PS demuxer against a real cutscene file.
//!
//! Needs the (non-distributed) game assets, so it is `#[ignore]`d and run on
//! demand:
//!
//! ```text
//! cargo test -p native32emu-core --test mpeg_demux -- --ignored --nocapture
//! ```
//!
//! Looks for the video under `<repo>/tmp/native32_game` by default, or override
//! the game root with the `NATIVE32_GAME_DIR` environment variable.

use std::path::PathBuf;

use native32emu_core::mpeg;

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

/// Find a `.mpg` cutscene, trying the known METAL path first then any `.mpg`.
fn find_mpg() -> Option<PathBuf> {
    let root = game_dir()?;
    let known = root.join("NA32SSL/ENGLISH/METAL/MSCG1.mpg");
    if known.is_file() {
        return Some(known);
    }
    // Fall back to a recursive scan for any .mpg.
    fn scan(dir: &std::path::Path) -> Option<PathBuf> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if let Some(found) = scan(&p) {
                    return Some(found);
                }
            } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mpg")) {
                return Some(p);
            }
        }
        None
    }
    scan(&root)
}

#[test]
#[ignore = "requires non-distributed .mpg assets"]
fn demux_real_cutscene_splits_streams() {
    let Some(path) = find_mpg() else {
        eprintln!("No .mpg assets found; skipping.");
        return;
    };
    eprintln!("Demuxing {}", path.display());

    let data = std::fs::read(&path).expect("read mpg");
    let streams = mpeg::demux_all(data);

    eprintln!(
        "video stream: {} bytes, audio stream: {} bytes (declared video={}, audio={})",
        streams.video.len(),
        streams.audio.len(),
        streams.num_video_streams,
        streams.num_audio_streams,
    );

    // The video elementary stream must be present and begin with an MPEG-1
    // sequence header start code (00 00 01 B3).
    assert!(
        !streams.video.is_empty(),
        "video stream should not be empty"
    );
    let seq = streams
        .video
        .windows(4)
        .position(|w| w == [0x00, 0x00, 0x01, 0xB3]);
    assert!(
        seq.is_some(),
        "video stream should contain a sequence header (00 00 01 B3)"
    );
    assert_eq!(
        seq,
        Some(0),
        "video stream should start with the sequence header"
    );

    // These cutscenes carry an MP2 audio track.
    assert!(
        !streams.audio.is_empty(),
        "audio stream should not be empty"
    );
}

#[test]
#[ignore = "requires non-distributed .mpg assets"]
fn decode_real_cutscene_video_frames() {
    let Some(path) = find_mpg() else {
        eprintln!("No .mpg assets found; skipping.");
        return;
    };
    eprintln!("Decoding video from {}", path.display());

    let data = std::fs::read(&path).expect("read mpg");
    let streams = mpeg::demux_all(data);

    let mut video = mpeg::Video::new(streams.video);
    assert!(video.has_header(), "should parse sequence header");
    let (w, h) = (video.width(), video.height());
    eprintln!("video {}x{} @ {:.3} fps", w, h, video.framerate());
    assert!(w > 0 && h > 0, "sane dimensions");

    // Decode several frames and confirm at least one is non-blank.
    let mut frames = 0u32;
    let mut non_blank = false;
    while frames < 30 {
        let Some(idx) = video.decode() else { break };
        let frame = video.frame(idx);
        assert_eq!(frame.width, w);
        assert_eq!(frame.height, h);
        if frame.y.data.iter().any(|&p| p != 0) {
            non_blank = true;
        }
        frames += 1;
    }
    eprintln!("decoded {frames} frames, non_blank={non_blank}");
    assert!(frames > 0, "should decode at least one frame");
    assert!(non_blank, "decoded frames should not be entirely blank");
}

#[test]
#[ignore = "requires non-distributed .mpg assets"]
fn decode_real_cutscene_audio_frames() {
    let Some(path) = find_mpg() else {
        eprintln!("No .mpg assets found; skipping.");
        return;
    };
    eprintln!("Decoding audio from {}", path.display());

    let data = std::fs::read(&path).expect("read mpg");
    let streams = mpeg::demux_all(data);

    let mut audio = mpeg::Audio::new(streams.audio);
    assert!(audio.has_header(), "should parse an MP2 frame header");
    let sr = audio.samplerate();
    eprintln!("audio samplerate = {sr} Hz");
    assert!(
        [44100, 48000, 32000, 22050, 24000, 16000].contains(&sr),
        "samplerate should be a valid MPEG audio rate"
    );

    let mut frames = 0u32;
    let mut peak = 0.0f32;
    while frames < 50 {
        let Some(s) = audio.decode() else { break };
        assert_eq!(s.interleaved.len(), 1152 * 2);
        for &v in &s.interleaved {
            assert!(v.is_finite(), "samples must be finite");
            peak = peak.max(v.abs());
        }
        frames += 1;
    }
    eprintln!("decoded {frames} audio frames, peak amplitude = {peak:.4}");
    assert!(frames > 0, "should decode at least one audio frame");
    assert!(peak > 0.0, "decoded audio should not be pure silence");
}
