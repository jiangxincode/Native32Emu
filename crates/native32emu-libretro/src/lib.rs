// native32emu-libretro - the libretro core front-end for Native32Emu.
//
// This crate is built as a `cdylib` named `native32emu_libretro`, producing
// `native32emu_libretro.{dll,so,dylib}` which RetroArch can load directly
// (no post-build renaming required). All emulation logic lives in the shared
// `native32emu` core crate; this crate only implements the libretro C API.

#![allow(clippy::upper_case_acronyms)]

pub mod libretro;
