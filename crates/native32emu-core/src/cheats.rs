// Shared cheat-code support for all front-ends.

use crate::action_vm::ActionVM;
use crate::frame_player::FramePlayer;
use crate::sprite_system::{MovieState, SpriteSystem};
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CheatParseError {
    #[error("cheat code is empty")]
    Empty,
    #[error("cheat code must use '<target>=<value>' syntax")]
    MissingValue,
    #[error("unknown cheat target '{0}'")]
    UnknownTarget(String),
    #[error("sprite cheat must use 'sprite:<name>.<field>=<value>'")]
    InvalidSpriteTarget,
    #[error("frame cheat must use 'frame:goto=<n>' or 'frame:playing=<bool>'")]
    InvalidFrameTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheatRule {
    Variable {
        name: String,
        value: String,
    },
    Sprite {
        name: String,
        field: SpriteField,
        value: String,
    },
    Frame {
        field: FrameField,
        value: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteField {
    X,
    Y,
    Depth,
    Frame,
    Visible,
    Playing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameField {
    Goto,
    Playing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheatSlot {
    pub enabled: bool,
    pub code: String,
    pub rule: CheatRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheatDebugConfig {
    pub enabled: bool,
    pub interval_frames: u64,
}

impl Default for CheatDebugConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_frames: CheatManager::DEFAULT_DEBUG_INTERVAL_FRAMES,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheatManager {
    slots: BTreeMap<u32, CheatSlot>,
    debug: CheatDebugConfig,
}

impl CheatManager {
    pub const DEFAULT_DEBUG_INTERVAL_FRAMES: u64 = 30;
    const DEBUG_LIST_LIMIT: usize = 64;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.slots.clear();
    }

    pub fn set_slot(
        &mut self,
        index: u32,
        enabled: bool,
        code: &str,
    ) -> Result<(), CheatParseError> {
        let code = code.trim();
        if code.is_empty() {
            self.slots.remove(&index);
            return Ok(());
        }

        let rule = CheatRule::from_str(code)?;
        self.slots.insert(
            index,
            CheatSlot {
                enabled,
                code: code.to_string(),
                rule,
            },
        );
        Ok(())
    }

    pub fn add_code(&mut self, code: &str) -> Result<u32, CheatParseError> {
        let index = self
            .slots
            .keys()
            .next_back()
            .map_or(0, |highest| highest.saturating_add(1));
        self.set_slot(index, true, code)?;
        Ok(index)
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    pub fn set_debug_logging(&mut self, enabled: bool, interval_frames: u64) {
        self.debug = CheatDebugConfig {
            enabled,
            interval_frames: interval_frames.max(1),
        };
    }

    pub fn debug_logging_enabled(&self) -> bool {
        self.debug.enabled
    }

    pub fn debug_interval_frames(&self) -> u64 {
        self.debug.interval_frames
    }

    pub fn maybe_log_targets(
        &self,
        tick_count: u64,
        vm: &ActionVM,
        sprites: &SpriteSystem,
        frame: &FramePlayer,
    ) {
        if !self.debug.enabled || !tick_count.is_multiple_of(self.debug.interval_frames) {
            return;
        }

        log::info!(
            "Cheat targets @ tick {}: frame current={} playing={} next={}",
            tick_count,
            frame.current_frame,
            frame.playing,
            format_option(frame.next_frame)
        );
        log::info!(
            "Cheat targets @ tick {}: vars {}",
            tick_count,
            format_vars(&vm.vars, Self::DEBUG_LIST_LIMIT)
        );
        log::info!(
            "Cheat targets @ tick {}: sprites {}",
            tick_count,
            format_sprites(sprites, Self::DEBUG_LIST_LIMIT)
        );
    }

    pub fn apply(&self, vm: &mut ActionVM, sprites: &mut SpriteSystem, frame: &mut FramePlayer) {
        for slot in self.slots.values().filter(|slot| slot.enabled) {
            slot.rule.apply(vm, sprites, frame);
        }
    }
}

impl FromStr for CheatRule {
    type Err = CheatParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if input.is_empty() {
            return Err(CheatParseError::Empty);
        }

        let (target, value) = input.split_once('=').ok_or(CheatParseError::MissingValue)?;
        let target = target.trim();
        let value = value.trim().to_string();

        if let Some(name) = target.strip_prefix("var:") {
            let name = name.trim();
            if name.is_empty() {
                return Err(CheatParseError::UnknownTarget(target.to_string()));
            }
            return Ok(Self::Variable {
                name: name.to_lowercase(),
                value,
            });
        }

        if let Some(spec) = target
            .strip_prefix("sprite:")
            .or_else(|| target.strip_prefix("movie:"))
        {
            let (name, field) = spec
                .rsplit_once('.')
                .ok_or(CheatParseError::InvalidSpriteTarget)?;
            let name = name.trim();
            if name.is_empty() {
                return Err(CheatParseError::InvalidSpriteTarget);
            }
            return Ok(Self::Sprite {
                name: name.to_string(),
                field: SpriteField::from_str(field.trim())?,
                value,
            });
        }

        if let Some(field) = target.strip_prefix("frame:") {
            let field = FrameField::from_str(field.trim())?;
            return Ok(Self::Frame { field, value });
        }

        Err(CheatParseError::UnknownTarget(target.to_string()))
    }
}

impl CheatRule {
    fn apply(&self, vm: &mut ActionVM, sprites: &mut SpriteSystem, frame: &mut FramePlayer) {
        match self {
            Self::Variable { name, value } => {
                vm.vars.insert(name.clone(), value.clone());
            }
            Self::Sprite { name, field, value } => {
                if let Some(movie) = sprites.get_mut(name) {
                    field.apply(movie, value);
                }
            }
            Self::Frame { field, value } => field.apply(frame, value),
        }
    }
}

impl FromStr for SpriteField {
    type Err = CheatParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_ascii_lowercase().as_str() {
            "x" => Ok(Self::X),
            "y" => Ok(Self::Y),
            "depth" => Ok(Self::Depth),
            "frame" | "currentframe" | "current_frame" => Ok(Self::Frame),
            "visible" => Ok(Self::Visible),
            "playing" => Ok(Self::Playing),
            _ => Err(CheatParseError::InvalidSpriteTarget),
        }
    }
}

impl SpriteField {
    fn apply(self, movie: &mut MovieState, value: &str) {
        match self {
            Self::X => movie.x = parse_i16(value),
            Self::Y => movie.y = parse_i16(value),
            Self::Depth => movie.depth = parse_u16(value),
            Self::Frame => movie.next_frame = Some(parse_usize(value) as isize),
            Self::Visible => movie.visible = parse_bool(value),
            Self::Playing => movie.playing = parse_bool(value),
        }
    }
}

impl FromStr for FrameField {
    type Err = CheatParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_ascii_lowercase().as_str() {
            "goto" | "current" | "currentframe" | "current_frame" => Ok(Self::Goto),
            "playing" => Ok(Self::Playing),
            _ => Err(CheatParseError::InvalidFrameTarget),
        }
    }
}

impl FrameField {
    fn apply(self, frame: &mut FramePlayer, value: &str) {
        match self {
            Self::Goto => frame.goto(parse_u32(value), frame.playing),
            Self::Playing => frame.playing = parse_bool(value),
        }
    }
}

fn format_vars(vars: &HashMap<String, String>, limit: usize) -> String {
    if vars.is_empty() {
        return "[]".to_string();
    }

    let mut entries: Vec<_> = vars.iter().collect();
    entries.sort_by_key(|(name, _)| *name);
    let omitted = entries.len().saturating_sub(limit);
    let mut parts: Vec<String> = entries
        .into_iter()
        .take(limit)
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    if omitted > 0 {
        parts.push(format!("... {omitted} more"));
    }
    format!("[{}]", parts.join(", "))
}

fn format_sprites(sprites: &SpriteSystem, limit: usize) -> String {
    let mut entries: Vec<_> = sprites.iter().collect();
    if entries.is_empty() {
        return "[]".to_string();
    }

    entries.sort_by_key(|(name, _)| *name);
    let omitted = entries.len().saturating_sub(limit);
    let mut parts: Vec<String> = entries
        .into_iter()
        .take(limit)
        .map(|(name, movie)| {
            format!(
                "{}(movie={} x={} y={} depth={} frame={} visible={} playing={} next={})",
                name,
                movie.movie,
                movie.x,
                movie.y,
                movie.depth,
                movie.frame,
                movie.visible,
                movie.playing,
                format_option(movie.next_frame)
            )
        })
        .collect();
    if omitted > 0 {
        parts.push(format!("... {omitted} more"));
    }
    format!("[{}]", parts.join(", "))
}

fn format_option<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(|| "none".to_string(), |value| value.to_string())
}
fn parse_bool(value: &str) -> bool {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "0" | "false" | "off" | "no" => false,
        "1" | "true" | "on" | "yes" => true,
        _ => parse_i64(value) != 0,
    }
}

fn parse_i64(value: &str) -> i64 {
    value.trim().parse::<f64>().unwrap_or(0.0) as i64
}

fn parse_i16(value: &str) -> i16 {
    parse_i64(value).clamp(i16::MIN as i64, i16::MAX as i64) as i16
}

fn parse_u16(value: &str) -> u16 {
    parse_i64(value).clamp(0, u16::MAX as i64) as u16
}

fn parse_u32(value: &str) -> u32 {
    parse_i64(value).clamp(0, u32::MAX as i64) as u32
}

fn parse_usize(value: &str) -> usize {
    parse_i64(value).max(0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_variable_cheat() {
        let rule = CheatRule::from_str("var:Lives=99").unwrap();
        assert_eq!(
            rule,
            CheatRule::Variable {
                name: "lives".to_string(),
                value: "99".to_string()
            }
        );
    }

    #[test]
    fn parses_sprite_cheat() {
        let rule = CheatRule::from_str("sprite:player.visible=0").unwrap();
        assert_eq!(
            rule,
            CheatRule::Sprite {
                name: "player".to_string(),
                field: SpriteField::Visible,
                value: "0".to_string()
            }
        );
    }

    #[test]
    fn configures_debug_logging() {
        let mut cheats = CheatManager::new();
        assert!(!cheats.debug_logging_enabled());
        assert_eq!(
            cheats.debug_interval_frames(),
            CheatManager::DEFAULT_DEBUG_INTERVAL_FRAMES
        );

        cheats.set_debug_logging(true, 0);
        assert!(cheats.debug_logging_enabled());
        assert_eq!(cheats.debug_interval_frames(), 1);
    }

    #[test]
    fn formats_discoverable_targets() {
        let mut vars = HashMap::new();
        vars.insert("lives".to_string(), "3".to_string());
        assert_eq!(format_vars(&vars, 64), "[lives=3]");

        let mut sprites = SpriteSystem::new();
        sprites.insert("hero".to_string(), MovieState::new(7, 12, 34, 5));
        let formatted = format_sprites(&sprites, 64);
        assert!(formatted.contains("hero(movie=7"));
        assert!(formatted.contains("x=12"));
        assert!(formatted.contains("y=34"));
    }

    #[test]
    fn applies_enabled_slots() {
        let mut cheats = CheatManager::new();
        cheats.set_slot(0, true, "var:score=1000").unwrap();
        cheats.set_slot(1, false, "var:score=0").unwrap();

        let mut vm = ActionVM::new();
        let mut sprites = SpriteSystem::new();
        let mut frame = FramePlayer::new();
        cheats.apply(&mut vm, &mut sprites, &mut frame);

        assert_eq!(vm.vars.get("score").map(String::as_str), Some("1000"));
    }

    #[test]
    fn applies_sprite_and_frame_cheats() {
        let mut cheats = CheatManager::new();
        cheats.set_slot(0, true, "sprite:hero.x=42").unwrap();
        cheats
            .set_slot(1, true, "sprite:hero.playing=false")
            .unwrap();
        cheats.set_slot(2, true, "frame:goto=7").unwrap();
        cheats.set_slot(3, true, "frame:playing=on").unwrap();

        let mut vm = ActionVM::new();
        let mut sprites = SpriteSystem::new();
        sprites.insert("hero".to_string(), MovieState::new(1, 0, 0, 0));
        let mut frame = FramePlayer::new();
        cheats.apply(&mut vm, &mut sprites, &mut frame);

        let hero = sprites.get("hero").unwrap();
        assert_eq!(hero.x, 42);
        assert!(!hero.playing);
        assert_eq!(frame.next_frame, Some(7));
        assert!(frame.playing);
        assert!(frame.playing);
    }
}
