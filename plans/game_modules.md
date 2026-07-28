# Game modules

## State hierarchy

The game runs one state-machine step per logic frame:

1. NMI sets `$12`, allowing `MainGameLoop` to start a frame.
2. `MainGameLoop` calls `RunModule`.
3. `RunModule` uses `$10` to select one top-level module.
4. The selected module may use `$11` and then `$B0` or `$0200` for nested
   state machines.
5. The main loop prepares OAM, clears `$12`, and waits for the next NMI.

The relevant variables are:

| Address | Disassembly name | Meaning |
| --- | --- | --- |
| `$7E0010` | `MODE` | Top-level module selected by `RunModule`. |
| `$7E0011` | `SUBMODE` | State within the current module. |
| `$7E00B0` | `SUBSUB` | Commonly a state within the current submode. |
| `$7E0200` | — | Another nested-state counter used by some larger sequences. |
| `$7E0012` | `LAG` | Synchronizes the main loop with NMI. |

These are conventions, not a type system. The meaning of `$11`, `$B0`, and
`$0200` depends on the active module. A routine may advance a sequence by
incrementing one of them, jump elsewhere by assigning a new value, or return
to ordinary play by clearing it.

The authoritative top-level dispatcher is
[`RunModule`](../jpdasm/bank_00.asm#L61). Individual modules usually have
their own local dispatch table.

## Top-level modules

| `$10` | Entry point | Role |
| --- | --- | --- |
| `$00` | `Module00_Intro` | Startup, title sequence, and intro. |
| `$01` | `Module01_FileSelect` | File-selection screen. |
| `$02` | `Module02_CopyFile` | Copy-file interface. |
| `$03` | `Module03_KILLFile` | Delete-file interface. |
| `$04` | `Module04_NameFile` | Player-name entry. |
| `$05` | `Module05_LoadFile` | Load a save and choose the next gameplay loader. |
| `$06` | `Module06_UnderworldLoad` | Load an interior room before underworld play. |
| `$07` | `Module07_Underworld` | Interior and dungeon gameplay. |
| `$08` | `Module08_OverworldLoad` | Load the normal overworld, usually after leaving an interior. |
| `$09` | `Module09_Overworld` | Normal overworld gameplay and transitions. |
| `$0A` | `Module0A_OverworldSpecialLoad` | Load a special overworld area. |
| `$0B` | `Module0B_OverworldSpecial` | Special-overworld gameplay; shares the `$09` dispatcher. |
| `$0C` | `Module0C_Unused` | Unused wrapper that temporarily runs another submodule. |
| `$0D` | `Module0D_Unused` | Second unused wrapper of the same kind. |
| `$0E` | `Module0E_Interface` | Item menu, text, maps, potions, flute menu, and save menu. |
| `$0F` | `Module0F_SpotlightClose` | Closing iris transition. |
| `$10` | `Module10_SpotlightOpen` | Opening iris transition. |
| `$11` | `Module11_UnderworldFallingEntrance` | Falling entrance into an interior. |
| `$12` | `Module12_GameOver` | Death and game-over sequence. |
| `$13` | `Module13_PendantBossVictory` | Pendant boss victory sequence. |
| `$14` | `Module14_Attract` | Title-screen attract/demo sequence. |
| `$15` | `Module15_MirrorWarpFromAga` | Agahnim mirror-warp sequence. |
| `$16` | `Module16_CrystalBossVictory` | Crystal boss victory sequence. |
| `$17` | `Module17_SaveAndQuit` | Save-and-quit sequence. |
| `$18` | `Module18_GanonEmerges` | Ganon emergence sequence. |
| `$19` | `Module19_TriforceRoom` | Triforce-room ending sequence. |
| `$1A` | `Module1A_Credits` | Ending credits. |
| `$1B` | `Module1B_SpawnSelect` | Select and load a respawn location. |

The common gameplay flow is:

```text
load file ($05)
  ├─> underworld load ($06)
  │     └─> underworld play ($07:$0F, iris opens)
  │           └─> underworld play ($07:$00)
  └─> overworld load ($08)
        └─> spotlight open ($10)
              └─> overworld play ($09)

enter interior:
overworld play ($09)
  ─> spotlight close ($0F)
  ─> underworld load ($06)
  ─> underworld play ($07:$0F, iris opens)
  ─> underworld play ($07:$00)

exit interior:
underworld play ($07)
  ─> spotlight close ($0F)
  ─> overworld load ($08)
  ─> spotlight open ($10)
  ─> overworld play ($09)
```

Here `$07:$0F` means top-level module `$07`, submode `$0F`. Opening the iris
outdoors is top-level module `$10`; opening it indoors is submode
`Module07_0F_LandingWipe`. Module `$0F` closes the iris and then selects the
destination module previously stored in `$010C`.

Module `$0E` temporarily owns the frame while an interface is open. The
spotlight, game-over, boss-victory, warp, and ending modules are separate
multi-frame sequences that eventually select another top-level module.

## Overworld module family

The playable overworld uses four top-level values but only two implementations:

| Load | Play | Meaning |
| --- | --- | --- |
| `$08` | `$09` | Normal Light World or Dark World area. |
| `$0A` | `$0B` | Special overworld area. |

`Module08_OverworldLoad` and `Module0A_OverworldSpecialLoad` share one
three-entry load table. `Module09_Overworld` and
`Module0B_OverworldSpecial` share one gameplay table and common per-frame
sprite, Link, camera-shake, and rain handling.

“Special overworld” is a game-engine category, not another size of ordinary
area. It includes isolated outdoor scenes handled separately from the normal
Light World and Dark World maps.

### Modules `$08` and `$0A`: load sequence

| `$11` | Routine | Work |
| --- | --- | --- |
| `$00` | `Module08_00_LoadProperties` | Establish area state, graphics, palettes, music, sprites, Link state, and scroll registers. |
| `$01` | `Overworld_LoadSubscreenAndSilenceSFX1` | Select and load the optional BG1 overlay. |
| `$02` | `Module08_02_LoadAndAdvance` | Build the logical overworld map and vanilla BG2 tilemap, then enter module `$10` for the opening iris. |

Module `$08` loads an area reached from an interior.
Module `$0A` uses the same stages but gets its source state from another
special overworld.

### Modules `$09` and `$0B`: gameplay sequence

`$11 = $00` is ordinary player control. Other values temporarily replace
player control with a transition or scripted action. The dispatcher is
[`Module09_Overworld`](../jpdasm/bank_02.asm#L7017).

| `$11` | Purpose |
| --- | --- |
| `$00` | Normal player control, camera movement, and ordinary scrolling. |
| `$01`–`$08` | Scroll into an adjacent overworld area: load graphics, prepare tilemap data and sprites, scroll, settle the camera, then restore control. |
| `$09`–`$0C` | Door and entrance animations. |
| `$0D`–`$16` | Mosaic-based area change followed by the same graphics, map, sprite, scroll, and fade-in stages. |
| `$17`–`$1C` | Return from a special overworld through a mosaic/load/fade sequence. |
| `$1D` | Fade out and switch to special-overworld loader `$0A`. |
| `$1E` | Fade out and load another special overworld while remaining in overworld play. |
| `$1F` | Move Link during the final part of a special-overworld transition. |
| `$20` | Reload the optional BG1 subscreen overlay. |
| `$21` | Rebuild the vanilla BG2 tilemap after `$20`. |
| `$22` | Fade brightness from zero back to normal, then restore player control. |
| `$23` | Mirror-warp state machine. |
| `$24`–`$29` | Another mosaic/load path used to prepare and reveal an overworld destination. |
| `$2A` | Recover Link after drowning. |
| `$2B` | Conditional black-and-white palette effect. |
| `$2C` | Mirror-warp state machine, sharing the `$23` handler. |
| `$2D` | Wait while the flute bird controls the transition. |
| `$2E` | Whirlpool-warp state machine. |
| `$2F` | Create the Turtle Rock portal and return to control. |

Several ranges reuse the same routines. For example, `$01` and `$0F` both
load auxiliary graphics, while `$02`, `$10`, `$1B`, and `$27` all request a
tilemap update. Their meaning comes from the surrounding sequence rather
than from the shared routine alone.

Some submodes use `$B0` for another state machine. Mirror warp (`$23`/`$2C`),
whirlpool (`$2E`), drowning recovery (`$2A`), and the mosaic stages are
examples. This is why a label such as `Module09_21` identifies one state in a
larger flow rather than a separately scheduled module.

## Relevance to the BG streamer

The BG streamer changes renderer ownership without replacing the module
system:

- Logical overworld loading still belongs in the existing load and transition
  states.
- Bulk BG rendering belongs in a forced-blank load state.
- Per-frame streaming belongs in module `$09`/`$0B` after final camera and
  shake offsets are known.
- NMI remains responsible for transferring tile data prepared by the main
  loop.

For milestone 1, replacing `Module09_21` removes only one vanilla BG2 rebuild
path. `LoadAndBuildOverworldScreen` and the mirror-warp sequence contain
other calls to `BuildOverworldFromMap16`, so those paths must be handled
separately before the global builder hook can be removed.

## Related references

- [`overworld_banks.md`](overworld_banks.md) describes where overworld code
  and data live.
- [`bg_streamer.md`](bg_streamer.md) describes the replacement BG renderer.
- [`vram.md`](vram.md) describes tilemap and upload ownership.
