#!/bin/bash
# Build script for Native32Emu libretro core

set -e

echo "Building Native32Emu libretro core..."

# Create a temporary Cargo.toml for libretro build
cat > Cargo.toml.libretro << 'EOF'
[package]
name = "native32-emu"
version = "0.1.0"
edition = "2021"
description = "A Native32 game emulator written in Rust - libretro core"

[lib]
name = "native32emu"
crate-type = ["cdylib"]
path = "src/lib.rs"

[dependencies]
anyhow = "1"
thiserror = "2"
log = "0.4"
rand = "0.10"
image = "0.25"

[features]
libretro = []

[profile.release]
opt-level = 3
lto = true
strip = true
EOF

# Backup original Cargo.toml
cp Cargo.toml Cargo.toml.bak

# Use libretro Cargo.toml
cp Cargo.toml.libretro Cargo.toml

# Build the libretro core
cargo build --release --features libretro --no-default-features

# Restore original Cargo.toml
mv Cargo.toml.bak Cargo.toml

# Clean up
rm Cargo.toml.libretro

echo "Build complete!"
echo "Output: target/release/native32emu.dll (or .so/.dylib)"
