# Native32 Emulator —— A Native32 game emulator written in Rust

<p align="center">
  <img src="res/logo-banner.svg" alt="Native32 Emulator" width="600">
</p>

<p align="center">
  <a href="https://github.com/jiangxincode/Native32Emu/actions/workflows/ci.yml"><img src="https://github.com/jiangxincode/Native32Emu/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://sonarcloud.io/dashboard?id=jiangxincode_Native32Emu"><img src="https://sonarcloud.io/api/project_badges/measure?project=jiangxincode_Native32Emu&metric=alert_status" alt="Quality Gate Status"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-BSD%203--Clause-blue.svg" alt="License: BSD 3-Clause"></a>
</p>

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
- **RetroArch integration** — libretro core for use with RetroArch frontend

## Building

Requires [Rust](https://www.rust-lang.org/tools/install) (stable).

### Standalone Mode (Default)

```bash
cargo build --release
```

### Libretro Core (for RetroArch)

**Windows:**
```cmd
build-libretro.bat
```

**Linux/macOS:**
```bash
chmod +x build-libretro.sh
./build-libretro.sh
```

This produces a dynamic library (`native32emu.dll` on Windows, `libnative32emu.so` on Linux, `native32emu.dylib` on macOS) that can be loaded by RetroArch.

**Note:** The libretro build uses a separate Cargo.toml configuration to avoid Windows DLL export symbol limits.

## Usage

### Standalone Mode

```bash
# Basic usage
cargo run -- path/to/game.smf

# With options
cargo run -- --scale 2 --volume 80 path/to/game.smf

# Release build
cargo run --release -- path/to/game.smf
```

### RetroArch Mode

1. **Build the libretro core** (see Building section above)
2. **Install the core**:
   - Copy `target/release/native32emu.dll` (or `.so`/`.dylib`) to RetroArch's `cores/` directory
   - Copy `core.info` to the same `cores/` directory
3. **Load the core in RetroArch**:
   - Open RetroArch
   - Select "Load Core" → "Native32 (Native32Emu)"
   - Select "Load Content" and choose a `.smf`, `.sgm`, or `.ssl` game file
4. **Controls**: Use keyboard or gamepad (D-Pad for directions, Z=A, X=B)

### Command-line Options

| Option | Description | Default |
|---|---|---|
| `<GAME_PATH>` | Path to the game file (`.smf`, `.sgm`, or `.ssl`) | *required* |
| `-s, --scale <1-16>` | Integer scaling factor | `1` |
| `-f, --fullscreen` | Run in fullscreen mode | off |
| `-v, --volume <0-100>` | Volume level (0 = mute, 100 = original) | `100` |
| `-S, --screenshot <PATH>` | Take a screenshot and exit (saves as PNG) | — |
| `--screenshot-frames <N>` | Frames to run before screenshot | `30` |
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

# Take a screenshot after 30 frames and exit
cargo run -- --screenshot screenshot.png --screenshot-frames 30 native32_game/EACT/EBBLADE.smf
```

## Architecture

```
src/
├── main.rs              # Emulation loop and VmHost implementation (standalone)
├── lib.rs               # Library entry point (for libretro core)
├── cli.rs               # Command-line argument parsing (standalone only)
├── actions.rs           # Action opcode enum (36 opcodes)
├── action_vm.rs         # Stack-based virtual machine
├── audio_engine.rs      # MP3/PCM audio playback (rodio for standalone, buffer for libretro)
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
├── sprite_system.rs     # Movie/sprite instance management
├── core_emulator/       # Core emulator for libretro mode
│   ├── mod.rs
│   └── emulator.rs      # Emulator implementation without window/audio dependencies
└── libretro/            # libretro API implementation
    ├── mod.rs
    ├── api.rs           # Exported libretro functions (retro_init, retro_run, etc.)
    ├── callbacks.rs     # Callback management for video/audio/input
    ├── constants.rs     # libretro constants
    └── types.rs         # libretro type definitions
```

## Dependencies

### Standalone Mode

| Crate | Purpose |
|---|---|
| `clap` | Command-line argument parsing |
| `minifb` | Window creation and pixel rendering |
| `rodio` | Audio playback (MP3 + PCM) |
| `anyhow` / `thiserror` | Error handling |
| `log` / `env_logger` | Logging |
| `rand` | Random number generation (for VM `RandomNumber` opcode) |

### Libretro Mode

The libretro core has minimal dependencies - only the core emulation libraries are used. Window management and audio playback are handled by the RetroArch frontend through callbacks.

## RetroArch Integration

Native32Emu can be used as a libretro core with RetroArch, allowing you to play Native32 games with RetroArch's features like shaders, netplay, and achievements.

### Supported Features

- ✅ Video output (XRGB8888 pixel format)
- ✅ Audio output (RAW PCM, stereo)
- ✅ Input handling (D-Pad + A/B buttons)
- ✅ Game loading (.smf, .sgm, .ssl files)
- ⚠️ MP3 audio (not yet implemented - only RAW PCM works)
- ❌ Save states (not yet implemented)
- ❌ Core options (not yet implemented)

### RetroPad Button Mapping

| RetroPad Button | Native32 Keycode | Action |
|----------------|------------------|--------|
| D-Pad Left | 0x0200 | Left |
| D-Pad Right | 0x0400 | Right |
| D-Pad Up | 0x1c00 | Up |
| D-Pad Down | 0x1e00 | Down |
| A (SNES East) | 0x8800 | B / Menu |
| B (SNES South) | 0x4000 | A |

### Audio Notes

- **RAW PCM**: Fully supported (11025Hz for YUV games, 22050Hz for ARGB games)
- **MP3**: Not yet supported in libretro mode (games using MP3 will have no music)
- Audio is output as stereo (mono sources are duplicated to both channels)

## Game Compatibility

Game resources can be downloaded from [Baidu Netdisk](https://pan.baidu.com/s/1b5sY3JFEP2HtxiKngOJ3VA?pwd=aloy).

All 84 Native32 games in the test suite load and run without fatal errors.

| Category | Count | Status |
|----------|-------|--------|
| Main Menu | 1 | ✅ Pass |
| EACT (Action) | 11 | ✅ Pass |
| EELA (Educational) | 32 | ✅ Pass |
| EPOP (Hot/Featured) | 9 | ✅ Pass |
| EPUZ (Puzzle) | 24 | ✅ Pass |
| ESPG (Sport) | 3 | ✅ Pass |
| ETAB (Chess/Board) | 4 | ✅ Pass |
| **Total** | **84** | **✅ All Passed** |

For detailed game list with screenshots and descriptions, see the [Game Compatibility](https://github.com/jiangxincode/Native32Emu/wiki/Game-Compatibility) wiki page.

## Contribute

Contributions are welcome! Whether you're interested in fixing bugs, adding features, improving documentation, or testing game compatibility, we'd love your help.

### How to Contribute

1. **Fork** this repository
2. **Create** a feature branch (`git checkout -b feature/your-feature`)
3. **Commit** your changes (`git commit -m 'Add your feature'`)
4. **Push** to the branch (`git push origin feature/your-feature`)
5. **Open** a Pull Request

### Areas That Need Help

- **Game compatibility testing** — test more games and report issues with screenshots
- **VM opcode implementation** — some rare opcodes are not yet fully implemented
- **Audio/video support** — `.mpg` video playback is not yet supported
- **Platform ports** — macOS and Linux testing and packaging
- **Documentation** — improve wiki pages and code comments
- **Bug reports** — if you find a game that doesn't work correctly, please open an issue
- **RetroArch integration** — MP3 audio support, save states, core options

### Getting Started

Check the [open issues](https://github.com/jiangxincode/Native32Emu/issues) for tasks labeled `good first issue` or `help wanted`. If you have questions, feel free to open a discussion issue.

To understand the Native32 game file formats (`.smf`, `.SSL`, `.dat`, `.mpg`, `.ssl_sav`) and their relationships, see the [Game File Formats](https://github.com/jiangxincode/Native32Emu/wiki/Game-File-Formats) wiki page.

## Acknowledgments

- [n32emu](https://github.com/gatecat/n32emu) by Myrtle Shah — the Python reference implementation
- [BootlegGames Wiki](https://bootleggames.fandom.com/wiki/Native_32) — hardware documentation and game catalog

## License

This project is licensed under the [BSD 3-Clause License](LICENSE).
