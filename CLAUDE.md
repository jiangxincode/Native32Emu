# Project Instructions

## Documentation Sync

When making code changes that affect user-facing behavior, always check and update:

- `README.md` — usage examples, feature lists, command-line options, RetroArch integration details
- `docs/` — CONTRIBUTING.md, Game-Compatibility.md, Game-File-Formats.md, or add new docs as needed

Examples of changes that require doc updates:
- New or modified CLI options
- New or changed opcodes / VM behavior
- New game support or compatibility changes
- Audio/video feature changes
- RetroArch core behavior changes
- New dependencies or build requirement changes

## Debugging Runtime / Interaction Bugs

Use this playbook for "the game does X wrong at runtime" reports, especially
input-driven ones like "pressing a key freezes the screen". The emulator core
(`crates/native32emu-core`) is platform-independent and can be driven headlessly,
which is the key to fast, deterministic debugging.

### 1. Reproduce headlessly with scripted input

Don't debug through the window. Drive the core directly and script input per
frame. The Action VM is seeded deterministically, so a fixed input script always
produces the same run.

Reusable harness lives in `crates/native32emu-core/tests/smoke.rs`:

- `run_scripted(path, frames, |frame| -> Vec<u16>)` — drives the core with a
  per-frame input script and returns the final `Emulator` for inspection.
- `find_asset(dir, "GFSTART.SSL")` — locates a game asset by name.
- Native32 keycodes (see `input_handler.rs`): Left `0x0200`, Right `0x0400`,
  Up `0x1c00`, Down `0x1e00`, Z/A `0x4000`, X/B `0x8800`.

A throwaway `examples/` binary that prints per-frame state is fine for live
investigation; delete it once the regression test is written.

### 2. Quantify the symptom, then narrow from coarse to fine

Turn "looks stuck" into a number, then drill down layer by layer:

1. Main timeline: `emu.frame_player.current_frame` / `.playing`.
2. Script state: relevant `emu.vm.vars` entries (names are lowercased).
3. Sprites/movies: `emu.sprites.get("name")` → `.frame` / `.playing` / `.next_frame`.

Pin down the exact object that is misbehaving (e.g. "movie X is stuck on frame N
with playing=false") before reading any code.

### 3. Add gated tracing, not permanent logging

For a hard case, add temporary `eprintln!` guarded by an env var so it is
zero-noise by default and easy to rip out:

```rust
if std::env::var("VM_TRACE").is_ok() { eprintln!("[vm] pc={pc} op={op:?}"); }
```

Trace VM opcodes and host calls (`stop`/`play`/`goto_frame`) to find *who*
issues an unexpected action and *how often*. "Same action fires every frame"
usually points at a loop or an over-eager per-frame pass.

### 4. Check whether it's a regression

Use `git log -S "<unique code snippet>" --all` and `git show <commit>` to find
when the suspect code was introduced and why. This distinguishes a recent
regression from long-standing latent code.

### 5. Lock it in with a deterministic regression test

Add an `#[ignore]`d test to `tests/smoke.rs` (it needs local assets via
`NATIVE32_GAME_DIR`; see `gunfire_menu_advances_on_confirm` for the pattern).

- **Assert the bug's semantic outcome, not a surface symptom.** Example: assert
  the menu actually advances (loaded content switches), not "the framebuffer
  changed" — an animated background can keep pixels changing while the logic is
  frozen, giving false confidence.
- **Verify the test catches the bug.** Temporarily revert the fix
  (`git checkout HEAD~1 -- <file>`), confirm the new test fails, then restore the
  fix. A regression test that never fails on the buggy code is worthless.

Run with:

```text
cargo test -p native32emu-core --release --test smoke -- --ignored --nocapture
```
