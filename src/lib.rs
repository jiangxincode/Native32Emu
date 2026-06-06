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

pub mod core;

#[cfg(feature = "libretro")]
pub mod libretro;
