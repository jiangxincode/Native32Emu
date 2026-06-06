// Platform-independent emulator core.
//
// Everything in this module is shared by both the standalone (minifb window)
// front-end and the libretro core. It has no dependency on any specific
// windowing, audio output, or input device; those concerns live in the
// `standalone` and `libretro` front-ends.

pub mod action_vm;
pub mod actions;
pub mod audio_engine;
pub mod content_loader;
pub mod des_constants;
pub mod emulator;
pub mod error;
pub mod file_loader;
pub mod frame_player;
pub mod header_decryptor;
pub mod image_decoder;
pub mod input_handler;
pub mod renderer;
pub mod save_manager;
pub mod sprite_system;
