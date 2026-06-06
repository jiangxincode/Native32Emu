@echo off
REM Build script for Native32Emu libretro core (Windows)

echo Building Native32Emu libretro core...

REM Create a temporary Cargo.toml for libretro build
(
echo [package]
echo name = "native32-emu"
echo version = "0.1.0"
echo edition = "2021"
echo description = "A Native32 game emulator written in Rust - libretro core"
echo.
echo [lib]
echo name = "native32emu"
echo crate-type = ["cdylib"]
echo path = "src/lib.rs"
echo.
echo [dependencies]
echo anyhow = "1"
echo thiserror = "2"
echo log = "0.4"
echo rand = "0.10"
echo image = "0.25"
echo.
echo [features]
echo libretro = []
echo.
echo [profile.release]
echo opt-level = 3
echo lto = true
echo strip = true
) > Cargo.toml.libretro

REM Backup original Cargo.toml
copy Cargo.toml Cargo.toml.bak >nul

REM Use libretro Cargo.toml
copy Cargo.toml.libretro Cargo.toml >nul

REM Build the libretro core
cargo build --release --features libretro --no-default-features

REM Restore original Cargo.toml
copy Cargo.toml.bak Cargo.toml >nul
del Cargo.toml.bak >nul

REM Clean up
del Cargo.toml.libretro >nul

echo Build complete!
echo Output: target\release\native32emu.dll
