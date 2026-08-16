# Dark Mountain lightning

This documents the vanilla Dark Mountain lightning behavior for reference in
case it is re-enabled later. The current patch disables the entire effect,
including its palette changes and thunder sound, because the fixed colors and
palette slots do not match consolidated theme palettes.

## Activation

`FlashGanonTowerPalette` runs during normal overworld control and while the save
menu is open outdoors. It returns without doing anything when an entrance
cutscene is active (`$04C6 != 0`) or the current overworld area (`$8A`) is not
one of:

- `$43`: West Dark Death Mountain and Ganon's Tower
- `$45`: East Dark Death Mountain
- `$47`: Turtle Rock

There is no theme or palette condition.

## Lightning cycle

The effect uses the low byte of the global frame counter `$1A` and repeats every
256 frames:

| Frame | Action |
| --- | --- |
| `$03` | Install lightning colors |
| `$05` | Restore loaded colors |
| `$24` | Install lightning colors and play SFX2 `$36` |
| `$2C` | Restore loaded colors |
| `$58` | Install lightning colors |
| `$5A` | Restore loaded colors |

The colors persist between actions, producing flashes during frames `$03-$04`,
`$24-$2B`, and `$58-$59`. The effect replaces colors 1 through 7 in five fixed
background palette groups at `$7EC560`, `$7EC570`, `$7EC590`, `$7EC5E0`, and
`$7EC5F0`, then restores them from the corresponding loaded palette groups.

## Ganon's Tower and Turtle Rock palettes

The same routine also writes all eight colors of the background palette at
`$7EC5D0`:

- In areas `$43` and `$45`, it cycles through four hard-coded Ganon's Tower
  palettes while the Ganon's Tower entrance-opened flag (`$7EF2C3 & $20`) is
  clear. Once the entrance is open, these writes stop.
- In area `$47`, it continually installs a fifth hard-coded palette. The
  Ganon's Tower entrance flag is not checked there.

The entrance-opened flag affects only this palette group; it does not disable
the general lightning colors or thunder sound.

## Re-enabling

The generated rain context is unrelated to this effect. Re-enabling lightning
requires generated theme data that explicitly selects compatible areas or
palettes, plus theme-compatible replacement colors rather than the vanilla
hard-coded values.
