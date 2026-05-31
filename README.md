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

## Game Compatibility

All 84 Native32 games in the test suite load and run without fatal errors. Each game was tested by launching the emulator, loading the ROM, running for 5 seconds, and checking for panics or crashes.

### Main Menu — FHUI.smf (1 game)

| # | Game | Status |
|---|------|--------|
| 1 | FHUI.smf | ✅ Pass |

### EACT — Action Games (11 games)

| # | Game | File | Status |
|---|------|------|--------|
| 1 | Bloody Blade | EBBLADE.smf | ✅ Pass |
| 2 | Gun Fire | EGUNFIRE.smf | ✅ Pass |
| 3 | Metal Storm | EMETAL.smf | ✅ Pass |
| 4 | Pirate | Epirate.smf | ✅ Pass |
| 5 | Storm | ESTORM.smf | ✅ Pass |
| 6 | Little Me | LittleMe.smf | ✅ Pass |
| 7 | Lost Sword | LostSwor.smf | ✅ Pass |
| 8 | Music Game | MusicGam.smf | ✅ Pass |
| 9 | PoPo Fun | PoPoFun.smf | ✅ Pass |
| 10 | ShaoLin K | ShaoLinK.smf | ✅ Pass |
| 11 | Three Pigs | ThreePig.smf | ✅ Pass |

### EELA — Educational Games (30 games)

| # | Game | File | Status |
|---|------|------|--------|
| 1 | Adding 21 | Adding21.smf | ✅ Pass |
| 2 | Alphabetical Order | AlpOrder.smf | ✅ Pass |
| 3 | Animal Friends | AnimalFr.smf | ✅ Pass |
| 4 | Animals 1 | Animals1.smf | ✅ Pass |
| 5 | Animals 2 | Animals2.smf | ✅ Pass |
| 6 | Chicklin | Chicklin.smf | ✅ Pass |
| 7 | Colors Magic | ColorsMa.smf | ✅ Pass |
| 8 | Comprehension | Comprehe.smf | ✅ Pass |
| 9 | Digital Hunt | DigiHunt.smf | ✅ Pass |
| 10 | Find The Match | FindTheM.smf | ✅ Pass |
| 11 | Fruits 1 | Fruits1.smf | ✅ Pass |
| 12 | Fruits 2 | Fruits2.smf | ✅ Pass |
| 13 | Geography | Geograph.smf | ✅ Pass |
| 14 | Living Things | LivingTi.smf | ✅ Pass |
| 15 | Magic Chain | MagicCha.smf | ✅ Pass |
| 16 | Magical A | MagicalA.smf | ✅ Pass |
| 17 | Math Bargain | MathBarg.smf | ✅ Pass |
| 18 | Monkey Army | MonkeyAr.smf | ✅ Pass |
| 19 | More Or Less | MoreOrLe.smf | ✅ Pass |
| 20 | Music Basic | MusicBas.smf | ✅ Pass |
| 21 | Ordered Blocks | OrdBlock.smf | ✅ Pass |
| 22 | Re-Letter | ReLetter.smf | ✅ Pass |
| 23 | Reads Picture | ReadsPic.smf | ✅ Pass |
| 24 | School 1 | SCHOOL1.smf | ✅ Pass |
| 25 | School 2 | SCHOOL2.smf | ✅ Pass |
| 26 | Seeker | Seeker.smf | ✅ Pass |
| 27 | Simple Arithmetic | SimpleAr.smf | ✅ Pass |
| 28 | Speed Arithmetic | SpeedAri.smf | ✅ Pass |
| 29 | Super Add | SuperAdd.smf | ✅ Pass |
| 30 | Us Time | UsTime.smf | ✅ Pass |
| 31 | Word Choice | WordChoi.smf | ✅ Pass |
| 32 | Workshop | Workshop.smf | ✅ Pass |

### EPOP — Hot/Featured Games (9 games)

| # | Game | File | Status |
|---|------|------|--------|
| 1 | Bloody Blade | EBBLADE.smf | ✅ Pass |
| 2 | Express | EExpress.smf | ✅ Pass |
| 3 | Gun Fire | EGUNFIRE.smf | ✅ Pass |
| 4 | Metal Storm | EMETAL.smf | ✅ Pass |
| 5 | Pirate | Epirate.smf | ✅ Pass |
| 6 | Rune Word | ERuneWod.smf | ✅ Pass |
| 7 | Storm | ESTORM.smf | ✅ Pass |
| 8 | School 1 | SCHOOL1.smf | ✅ Pass |
| 9 | School 2 | SCHOOL2.smf | ✅ Pass |

### EPUZ — Puzzle Games (25 games)

| # | Game | File | Status |
|---|------|------|--------|
| 1 | Bad Boy | Bad Boy.smf | ✅ Pass |
| 2 | Bell Girls | BellGirl.smf | ✅ Pass |
| 3 | Cat Run | Cat Run.smf | ✅ Pass |
| 4 | CE Castle | CeCastle.smf | ✅ Pass |
| 5 | Dragon | Dragon.smf | ✅ Pass |
| 6 | Dr. Fairy | DrFairy.smf | ✅ Pass |
| 7 | Element | Element.smf | ✅ Pass |
| 8 | Rune Word | ERuneWod.smf | ✅ Pass |
| 9 | Food Rain | FoodRain.smf | ✅ Pass |
| 10 | Frog | Frog.smf | ✅ Pass |
| 11 | Fruit Party | FruParty.smf | ✅ Pass |
| 12 | Gem Woods | GemWoods.smf | ✅ Pass |
| 13 | Guess | Guess.smf | ✅ Pass |
| 14 | Hide & Seek | HideSeek.smf | ✅ Pass |
| 15 | Lucky Rabbits | LRabbits.smf | ✅ Pass |
| 16 | Mouse | Mouse.smf | ✅ Pass |
| 17 | Mushu Mus | MushuMus.smf | ✅ Pass |
| 18 | Nau Orang | NauOrang.smf | ✅ Pass |
| 19 | Orchard | Orchard.smf | ✅ Pass |
| 20 | Pirate C | PirateC.smf | ✅ Pass |
| 21 | Puzzle | Puzzle.smf | ✅ Pass |
| 22 | Snake Mania | SnakeMa.smf | ✅ Pass |
| 23 | Sudoku | Sudoku.smf | ✅ Pass |
| 24 | Zero Hunt | ZeroHunt.smf | ✅ Pass |

### ESPG — Sport Games (3 games)

| # | Game | File | Status |
|---|------|------|--------|
| 1 | Basketball | Basketba.smf | ✅ Pass |
| 2 | Bowling | Bowling.smf | ✅ Pass |
| 3 | Express | EExpress.smf | ✅ Pass |

### ETAB — Chess/Board Games (4 games)

| # | Game | File | Status |
|---|------|------|--------|
| 1 | Lucky 21 | Lucky 21.smf | ✅ Pass |
| 2 | Lucky Box | LuckyBox.smf | ✅ Pass |
| 3 | Paradise 777 | Parad777.smf | ✅ Pass |
| 4 | Sky Fighter | SkyFight.smf | ✅ Pass |

### Summary

| Category | Count | Passed | Failed |
|----------|-------|--------|--------|
| Main Menu | 1 | 1 | 0 |
| EACT (Action) | 11 | 11 | 0 |
| EELA (Educational) | 30 | 30 | 0 |
| EPOP (Hot/Featured) | 9 | 9 | 0 |
| EPUZ (Puzzle) | 25 | 25 | 0 |
| ESPG (Sport) | 3 | 3 | 0 |
| ETAB (Chess/Board) | 4 | 4 | 0 |
| **Total** | **84** | **84** | **0** |

## Acknowledgments

- [n32emu](https://github.com/gatecat/n32emu) by Myrtle Shah — the Python reference implementation this emulator is based on
- [BootlegGames Wiki](https://bootleggames.fandom.com/wiki/Native_32) — hardware documentation and game catalog

## License

This project is licensed under the [BSD 3-Clause License](third_party/n32emu/COPYING), consistent with the reference implementation.
