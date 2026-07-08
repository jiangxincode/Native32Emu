# Libretro Core Options

When running Native32Emu as a libretro core, these options are configurable
from RetroArch's *Quick Menu → Core Options*. Changes apply live without
reloading the game.

| Option | Values | Default | Effect |
|--------|--------|---------|--------|
| Audio Volume (%) | 0–100 (steps of 10) | 100 | Output volume; 0 mutes |
| Key auto-repeat delay (frames) | 2–30 | 12 | Frames a held key waits before auto-repeat starts (~0.4s at 30fps) |
| Key auto-repeat period (frames) | 1–10 | 3 | Frames between auto-repeat pulses (~0.1s at 30fps) |
| Swap A/B buttons | disabled / enabled | disabled | Swaps the A and B button actions |
| Auto-skip cutscene videos | disabled / enabled | disabled | Skips logo/intro/cutscene videos automatically |
| Cheat target debug logging | disabled / enabled | disabled | Periodically logs VM variables and sprites to help build cheat rules |
| Cheat debug interval (frames) | 15–300 | 30 | Frames between cheat target debug logs |

The auto-repeat options reproduce the original keypad's typematic behavior that
some games rely on (e.g. walk vs. run on a held direction).

## Cheats

Cheats are exposed through RetroArch's cheat interface, not as a core option.
Use *Quick Menu → Cheats* and enter shared Native32Emu cheat rules such as
`var:lives=99` or `sprite:player.visible=0`.

Enable *Cheat target debug logging* when you need to discover real variable and sprite names from the running game log.
