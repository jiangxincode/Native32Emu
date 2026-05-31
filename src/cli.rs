// CLI: command-line interface for the Native32 emulator.

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "native32-emu")]
#[command(about = "A Native32 game emulator written in Rust")]
#[command(version)]
pub struct Cli {
    /// Path to the game file (.smf, .sgm, or .ssl)
    pub game_path: Option<PathBuf>,

    /// Integer scaling factor (1-16)
    #[arg(short, long, default_value = "1", value_parser = clap::value_parser!(u32).range(1..=16))]
    pub scale: u32,

    /// Run in fullscreen mode
    #[arg(short, long)]
    pub fullscreen: bool,

    /// Volume level (0-100, 0=mute, 100=original)
    #[arg(short, long, default_value = "100", value_parser = clap::value_parser!(u32).range(0..=100))]
    pub volume: u32,

    /// Enable debug/development mode
    #[arg(long)]
    pub debug: bool,

    /// Key remapping in format "keycode:key" (e.g., "0x0200:a")
    #[arg(long = "remap", value_name = "KEYCODE:KEY")]
    pub key_remappings: Vec<String>,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Parse key remappings from the command-line format.
    pub fn parse_key_remappings(&self) -> Vec<(u16, minifb::Key)> {
        let mut remappings = Vec::new();
        for remap in &self.key_remappings {
            let parts: Vec<&str> = remap.splitn(2, ':').collect();
            if parts.len() != 2 {
                log::warn!("Invalid key remapping format: {}", remap);
                continue;
            }
            let keycode = match u16::from_str_radix(parts[0].trim_start_matches("0x"), 16) {
                Ok(k) => k,
                Err(_) => {
                    log::warn!("Invalid keycode: {}", parts[0]);
                    continue;
                }
            };
            let key = match parse_key(parts[1]) {
                Some(k) => k,
                None => {
                    log::warn!("Unknown key: {}", parts[1]);
                    continue;
                }
            };
            remappings.push((keycode, key));
        }
        remappings
    }
}

fn parse_key(s: &str) -> Option<minifb::Key> {
    match s.to_lowercase().as_str() {
        "a" => Some(minifb::Key::A),
        "b" => Some(minifb::Key::B),
        "c" => Some(minifb::Key::C),
        "d" => Some(minifb::Key::D),
        "e" => Some(minifb::Key::E),
        "f" => Some(minifb::Key::F),
        "g" => Some(minifb::Key::G),
        "h" => Some(minifb::Key::H),
        "i" => Some(minifb::Key::I),
        "j" => Some(minifb::Key::J),
        "k" => Some(minifb::Key::K),
        "l" => Some(minifb::Key::L),
        "m" => Some(minifb::Key::M),
        "n" => Some(minifb::Key::N),
        "o" => Some(minifb::Key::O),
        "p" => Some(minifb::Key::P),
        "q" => Some(minifb::Key::Q),
        "r" => Some(minifb::Key::R),
        "s" => Some(minifb::Key::S),
        "t" => Some(minifb::Key::T),
        "u" => Some(minifb::Key::U),
        "v" => Some(minifb::Key::V),
        "w" => Some(minifb::Key::W),
        "x" => Some(minifb::Key::X),
        "y" => Some(minifb::Key::Y),
        "z" => Some(minifb::Key::Z),
        "0" => Some(minifb::Key::Key0),
        "1" => Some(minifb::Key::Key1),
        "2" => Some(minifb::Key::Key2),
        "3" => Some(minifb::Key::Key3),
        "4" => Some(minifb::Key::Key4),
        "5" => Some(minifb::Key::Key5),
        "6" => Some(minifb::Key::Key6),
        "7" => Some(minifb::Key::Key7),
        "8" => Some(minifb::Key::Key8),
        "9" => Some(minifb::Key::Key9),
        "space" => Some(minifb::Key::Space),
        "enter" | "return" => Some(minifb::Key::Enter),
        "left" => Some(minifb::Key::Left),
        "right" => Some(minifb::Key::Right),
        "up" => Some(minifb::Key::Up),
        "down" => Some(minifb::Key::Down),
        "escape" | "esc" => Some(minifb::Key::Escape),
        _ => None,
    }
}
