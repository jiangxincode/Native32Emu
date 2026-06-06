// Native32 Emulator - shared library crate.
//
// This crate contains the platform-independent emulator core plus the two
// front-end integrations:
//   - `core_emulator`: the shared Emulator used by both front-ends.
//   - `libretro`: the libretro C API (only built with the "libretro" feature).
// The standalone binary (src/main.rs) links against this crate and adds the
// minifb window front-end.

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

// Shared emulator core, used by both the standalone and libretro front-ends.
pub mod core_emulator;

#[cfg(feature = "libretro")]
pub mod libretro;
