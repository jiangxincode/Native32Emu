// Native32 Emulator - library entry point for libretro core.

#![allow(dead_code)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::manual_memcpy)]
#![allow(clippy::needless_range_loop)]

pub mod action_vm;
pub mod actions;
pub mod audio_engine;
pub mod content_loader;
pub mod des_constants;
pub mod error;
pub mod file_loader;
pub mod frame_player;
pub mod header_decryptor;
pub mod image_decoder;
pub mod input_handler;
pub mod renderer;
pub mod save_manager;
pub mod sprite_system;

#[cfg(feature = "libretro")]
pub mod libretro;

#[cfg(feature = "libretro")]
pub mod core_emulator;
