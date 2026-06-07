# Native32 Emulator —— A Native32 game emulator written in Rust

<p align="center">
  <img src="res/logo-banner.svg" alt="Native32 Emulator" width="600">
</p>

<p align="center">
  <a href="https://github.com/jiangxincode/Native32Emu/actions/workflows/ci.yml"><img src="https://github.com/jiangxincode/Native32Emu/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/jiangxincode/Native32Emu/releases/latest"><img src="https://img.shields.io/github/v/release/jiangxincode/Native32Emu" alt="Release"></a>
  <a href="https://github.com/jiangxincode/Native32Emu/releases"><img src="https://img.shields.io/github/downloads/jiangxincode/Native32Emu/total" alt="Downloads"></a>
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

## Usage

### Standalone Mode

Download the latest binary from the [Releases](https://github.com/jiangxincode/Native32Emu/releases) page.

```bash
# Basic usage
native32-emu path/to/game.smf

# With options
native32-emu --scale 2 --volume 80 path/to/game.smf

# 2x scaling with 50% volume
native32-emu --scale 2 --volume 50 game.smf

# Remap A button to Space
native32-emu --remap "0x4000:space" game.smf

# Fullscreen mode
native32-emu --fullscreen game.smf

# Take a screenshot after 30 frames and exit
native32-emu --screenshot screenshot.png --screenshot-frames 30 game.smf
```

#### Command-line Options

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

#### Default Key Mappings

| Native32 Keycode | Physical Key | Action |
|---|---|---|
| `0x0200` | ← Left | Left |
| `0x0400` | → Right | Right |
| `0x1c00` | ↑ Up | Up |
| `0x1e00` | ↓ Down | Down |
| `0x4000` | Z | A |
| `0x8800` | X | B / Menu |

### RetroArch Mode

Native32Emu can be used as a libretro core with RetroArch, allowing you to play Native32 games with RetroArch's features like shaders, netplay, and achievements.

1. **Download the libretro core** from the [Releases](https://github.com/jiangxincode/Native32Emu/releases) page
2. **Install the core**:
   - Copy `native32emu_libretro.dll` (or `.so`/`.dylib`) to RetroArch's `cores/` directory
   - Copy `native32emu_libretro.info` to RetroArch's `info/` directory
3. **Load the core in RetroArch**:
   - Open RetroArch
   - Select "Load Core" → "Native32 (Native32Emu)"
   - Select "Load Content" and choose a `.smf`, `.sgm`, or `.ssl` game file

#### Supported Features

- ✅ Video output (XRGB8888 pixel format)
- ✅ Audio output (RAW PCM, stereo)
- ✅ Input handling (D-Pad + A/B buttons)
- ✅ Game loading (.smf, .sgm, .ssl files)
- ⚠️ MP3 audio (not yet implemented — only RAW PCM works)
- ❌ Save states (not yet implemented)
- ❌ Core options (not yet implemented)

#### RetroPad Button Mapping

| RetroPad Button | Native32 Keycode | Action |
|----------------|------------------|--------|
| D-Pad Left | 0x0200 | Left |
| D-Pad Right | 0x0400 | Right |
| D-Pad Up | 0x1c00 | Up |
| D-Pad Down | 0x1e00 | Down |
| A (SNES East) | 0x8800 | B / Menu |
| B (SNES South) | 0x4000 | A |

#### Audio Notes

- **RAW PCM**: Fully supported (11025Hz for YUV games, 22050Hz for ARGB games)
- **MP3**: Not yet supported in libretro mode (games using MP3 will have no music)
- Audio is output as stereo (mono sources are duplicated to both channels)

## Building

Requires [Rust](https://www.rust-lang.org/tools/install) (stable).

### Standalone Mode (Default)

```bash
cargo build -p native32emu --release
```

The binary is produced at `target/release/native32-emu`.

### Libretro Core (for RetroArch)

```bash
cargo build -p native32emu-libretro --release
```

This produces `native32emu_libretro.dll` on Windows (`libnative32emu_libretro.so`
on Linux, `libnative32emu_libretro.dylib` on macOS) under `target/release/`.

#### Distributing via RetroArch's Online Updater

To make the core installable directly from RetroArch (Online Updater > Core
Downloader), it needs to be added to the Libretro build infrastructure. The
repository ships the two files the buildbot requires:

- `Makefile` — wraps `cargo build` so the buildbot's `make` invocation produces
  `native32emu_libretro.<ext>` (it maps the libretro `platform`/`arch`
  variables to the matching Rust target triple for cross-compilation).
- `.gitlab-ci.yml` — the buildbot recipe that builds the core for Windows,
  Linux and macOS using the official `libretro-infrastructure/ci-templates`.

Remaining steps (done against Libretro's own repositories):

1. Submit `crates/native32emu-libretro/native32emu_libretro.info` to the
   [libretro-super `dist/info`](https://github.com/libretro/libretro-super/tree/master/dist/info)
   directory via a pull request.
2. Ask the Libretro team to register this repository's `.gitlab-ci.yml` recipe
   so the buildbot starts building the core (see the
   [forum thread on adding a new core](https://forums.libretro.com/t/i-have-a-new-core-what-to-do/37582)).
3. (Optional, for playlists) add core icons to
   [retroarch-assets](https://github.com/libretro/retroarch-assets) and game
   entries to the [libretro-database](https://github.com/libretro/libretro-database).
4. (Optional) add core documentation to [libretro/docs](https://github.com/libretro/docs#adding-a-new-core).

## Testing

Run the unit tests:

```bash
cargo test --workspace
```

There is also a smoke test that loads every available game, runs it for a number
of frames, and checks that the emulator neither panics nor produces a blank
frame. It needs the (non-distributed) game assets, so it is `#[ignore]`d by
default and run on demand:

```bash
# Uses <repo>/native32_game by default, or set NATIVE32_GAME_DIR
cargo test -p native32emu-core --test smoke -- --ignored --nocapture
```

## Architecture

```
crates/
├── native32emu-core/            # Platform-independent emulator engine (library)
│   └── src/
│       ├── lib.rs               # Crate root (module declarations)
│       ├── emulator.rs          # Shared Emulator + VmHost (both front-ends)
│       ├── actions.rs           # Action opcode enum (36 opcodes)
│       ├── action_vm.rs         # Stack-based virtual machine
│       ├── audio_engine.rs      # MP3/PCM audio (rodio for standalone, buffer for libretro)
│       ├── content_loader.rs    # SSL multi-file content switching
│       ├── des_constants.rs     # DES permutation tables and S-boxes
│       ├── error.rs             # Error types
│       ├── file_loader.rs       # File I/O, header parsing, resource tables
│       ├── frame_player.rs      # Main timeline frame playback (30fps)
│       ├── header_decryptor.rs  # Custom DES ECB header decryption
│       ├── image_decoder.rs     # YUV 4:2:0 and ARGB1555 image decoders
│       ├── input_handler.rs     # Input to keycode mapping (keyboard / RetroPad)
│       ├── renderer.rs          # Frame rendering with depth sorting
│       ├── save_manager.rs      # Save data persistence (.ssl_sav)
│       └── sprite_system.rs     # Movie/sprite instance management
├── native32emu/                 # Standalone binary (-> native32-emu)
│   └── src/
│       ├── main.rs              # Window loop and thin front-end
│       └── standalone/
│           ├── cli.rs           # Command-line argument parsing
│           └── gamepad_overlay.rs  # On-screen virtual gamepad overlay
└── native32emu-libretro/        # libretro cdylib (-> native32emu_libretro.{dll,so,dylib})
    ├── native32emu_libretro.info   # RetroArch core metadata
    └── src/
        ├── lib.rs               # cdylib crate root
        └── libretro/
            ├── api.rs           # Exported libretro functions (retro_init, retro_run, etc.)
            ├── callbacks.rs     # Callback management for video/audio/input
            ├── constants.rs     # libretro constants
            ├── logger.rs        # Bridges the `log` crate to the libretro log interface
            └── types.rs         # libretro type definitions
```

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

For detailed game list with screenshots and descriptions, see [Game Compatibility](docs/Game-Compatibility.md).

## Contribute

Contributions are welcome! Whether you're interested in fixing bugs, adding features, improving documentation, or testing game compatibility, we'd love your help. See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for details.

## Acknowledgments

- [n32emu](https://github.com/gatecat/n32emu) by Myrtle Shah — the Python reference implementation
- [BootlegGames Wiki](https://bootleggames.fandom.com/wiki/Native_32) — hardware documentation and game catalog

## Community

Welcome to join the QQ group to discuss Native32 games, report issues, or share childhood memories:

<img src="res/qrcode_1780795368223.jpg" alt="QQ交流群" width="200">

## License

This project is licensed under the [BSD 3-Clause License](LICENSE).
