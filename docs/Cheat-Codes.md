# Game Cheat Codes

This document lists game-specific cheat rules verified with Native32Emu. For the
shared rule syntax and target discovery options, see
[Standalone Emulator](Standalone-Emulator.md#cheats).

## Blades of Red / EBBLADE (赤刃)

Game file: `EPOP/EBBLADE.smf`

| Effect | Cheat rule | Status |
|---|---|---|
| Infinite health | `var:p_hp=96` | Verified |

### Standalone

```powershell
native32-emu --cheat "var:p_hp=96" "EPOP/EBBLADE.smf"
```

The rule is applied every frame, keeping the player's health at its normal
maximum value of `96` after taking damage.

To inspect other player-related variables:

```powershell
native32-emu --debug-cheats --cheat-debug-filter "p_*" "EPOP/EBBLADE.smf"
```

Known player targets include `p_hp`, `p_life`, `p_mp`, `p_invincible`, and
`p_score`. Only `p_hp` has been verified as a cheat target so far.

### RetroArch

Native32Emu rules are **Emulator Handled** cheats. They must be sent to the
core, rather than configured as RetroArch memory-address cheats.

1. Load the game with the Native32Emu core.
2. Open **Quick Menu > Cheats**.
3. Select **Add New Code to Top** or **Add New Code to Bottom**.
4. Open the newly added cheat entry.
5. Set **Handler** to **Emulator**.
6. Select **Code** and enter:

```text
var:p_hp=96
```

7. Optionally set **Description** to `Infinite health`.
8. Set **Enabled** to **On**.
9. Return to the Cheats menu and select **Apply Changes**.

The **RetroArch** handler is not compatible with Native32Emu's named variable
rules because it expects a memory address and value instead of passing the code
to the core. See the official
[RetroArch cheat code guide](https://docs.libretro.com/guides/cheat-codes/)
for the distinction between Emulator Handled and RetroArch Handled cheats.
