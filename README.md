# Native32 Emulator

A Native32 game emulator written in Rust, based on the [n32emu](https://github.com/gatecat/n32emu) Python reference implementation.

Native32 is a game format developed by Sunplus for DVD player and TV chipsets (circa 2005–2011). Games use `.smf`, `.sgm`, or `.ssl` file extensions and feature a stack-based, ActionScript-like virtual machine with raster graphics.

## Features

- **Full Native32 format support** — file loading, header decryption, resource table parsing
- **YUV & ARGB image decoding** — with packbits/RLE decompression and color space conversion
- **Action bytecode VM** — 36 opcodes covering arithmetic, logic, string ops, control flow, sprites, and I/O
- **Sprite/movie system** — animation, cloning, visibility control, depth-sorted rendering
- **Audio playback** — MP3 music and raw 16-bit PCM sound effects
- **Keyboard input** — configurable key remapping
- **Save system** — `.ssl_sav` file persistence
- **SSL multi-file content** — seamless switching between game levels/files
- **CLI controls** — scaling, fullscreen, volume adjustment

## Building

Requires [Rust](https://www.rust-lang.org/tools/install) (stable).

```bash
cargo build --release
```

## Usage

```bash
# Basic usage
cargo run -- path/to/game.smf

# With options
cargo run -- --scale 2 --volume 80 path/to/game.smf

# Release build
cargo run --release -- path/to/game.smf
```

### Command-line Options

| Option | Description | Default |
|---|---|---|
| `<GAME_PATH>` | Path to the game file (`.smf`, `.sgm`, or `.ssl`) | *required* |
| `-s, --scale <1-16>` | Integer scaling factor | `1` |
| `-f, --fullscreen` | Run in fullscreen mode | off |
| `-v, --volume <0-100>` | Volume level (0 = mute, 100 = original) | `100` |
| `--debug` | Enable debug/development mode | off |
| `--remap <keycode:key>` | Remap a Native32 keycode to a physical key | — |

### Default Key Mappings

| Native32 Keycode | Physical Key | Action |
|---|---|---|
| `0x0200` | ← Left | Left |
| `0x0400` | → Right | Right |
| `0x1c00` | ↑ Up | Up |
| `0x1e00` | ↓ Down | Down |
| `0x4000` | Z | A |
| `0x8800` | X | B / Menu |

### Examples

```bash
# 2x scaling with 50% volume
cargo run -- --scale 2 --volume 50 native32_game/FHUI.smf

# Remap A button to Space
cargo run -- --remap "0x4000:space" native32_game/FHUI.smf

# Fullscreen mode
cargo run -- --fullscreen native32_game/EACT/EBBLADE.smf
```

## Architecture

```
src/
├── main.rs              # Emulation loop and VmHost implementation
├── cli.rs               # Command-line argument parsing
├── actions.rs           # Action opcode enum (36 opcodes)
├── action_vm.rs         # Stack-based virtual machine
├── audio_engine.rs      # MP3/PCM audio playback (rodio)
├── content_loader.rs    # SSL multi-file content switching
├── des_constants.rs     # DES permutation tables and S-boxes
├── error.rs             # Error types
├── file_loader.rs       # File I/O, header parsing, resource tables
├── frame_player.rs      # Main timeline frame playback (30fps)
├── header_decryptor.rs  # Custom DES ECB header decryption
├── image_decoder.rs     # YUV 4:2:0 and ARGB1555 image decoders
├── input_handler.rs     # Keyboard input to keycode mapping
├── renderer.rs          # Frame rendering with depth sorting
├── save_manager.rs      # Save data persistence (.ssl_sav)
└── sprite_system.rs     # Movie/sprite instance management
```

## Dependencies

| Crate | Purpose |
|---|---|
| `clap` | Command-line argument parsing |
| `minifb` | Window creation and pixel rendering |
| `rodio` | Audio playback (MP3 + PCM) |
| `anyhow` / `thiserror` | Error handling |
| `log` / `env_logger` | Logging |
| `rand` | Random number generation (for VM `RandomNumber` opcode) |

## Acknowledgments

- [n32emu](https://github.com/gatecat/n32emu) by Myrtle Shah — the Python reference implementation this emulator is based on
- [BootlegGames Wiki](http://bootleggames.wikia.com/wiki/Native_32) — hardware documentation and game catalog

## License

This project is licensed under the [BSD 3-Clause License](third_party/n32emu/COPYING), consistent with the reference implementation.
