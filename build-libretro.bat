@echo off
REM Build script for Native32Emu libretro core (Windows)

echo Building Native32Emu libretro core...

REM Use cargo rustc to specify cdylib crate-type without modifying Cargo.toml
cargo rustc --lib --release --features libretro --no-default-features --crate-type cdylib

echo Build complete!
echo Output: target\release\native32emu.dll
