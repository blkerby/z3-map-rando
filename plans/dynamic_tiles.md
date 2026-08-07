# Dynamic overworld tiles

This is an inventory of runtime changes to the playable-overworld Map16 grid
at `$7E2000`. It includes direct interactions, scripted events, and persistent
overlays restored when an area reloads. It excludes ordinary map loading and
changes that affect only 8x8 graphics.

## Terrain interactions

| Interaction | Original Map16 IDs | Tile property | Normal replacement |
| --- | --- | --- | --- |
| Cut grass | `$037D` | `$40` | `$0DBF` |
| Dig terrain | `$0034`, `$0035`, `$0071`, `$00DA`, `$00E1`, `$00E2`, `$00F8`, `$010D`, `$010E`, `$010F` | `$48` | `$0DC3` |
| Cut, lift, bomb, or powder a green bush | `$0036` | `$50` | `$0DC1` |
| Cut, lift, bomb, or powder a yellow/heavy bush | `$0727` | `$51` | `$0DC2` |
| Hammer a peg | `$021B` | `$27` (hookable) | `$0DC5` |
| Lift a sign | `$0101` | `$54` | `$0DC0` |
| Lift a small gray rock | `$020F` | `$52` | `$0DC4` |
| Lift a small black rock | `$0239` | `$53` | `$0DC4` |
| Lift a 2x2 large gray rock | `$036C`, `$036D`, `$0373`, `$0374` | `$55` | Usually `$0DC7-$0DCA` |
| Lift a 2x2 large black rock | `$023B-$023E` | `$56` | Usually `$0DC7-$0DCA` |
| Smash a 2x2 rock pile by dashing | `$0226-$0229` | `$57` | Usually `$0DC7-$0DCA` |

A secret at the affected coordinate overrides the normal replacement. A bush,
grass tile, or rock formation can therefore reveal one of these instead:

| Secret | Replacement Map16 IDs |
| --- | --- |
| Hole | `$0DC6` |
| Portal | `$0212` |
| Bombable entrance | `$0DAE`, `$0DAF` |
| Stairs | `$0912-$0915` |

The source checks and ordinary replacements are in
[`HandleItemTileAction_Overworld`](../jpdasm/bank_1B.asm#L12374),
[`HandleOverworldLiftables`](../jpdasm/bank_1B.asm#L12752), and
[`BombOverworldTiles`](../jpdasm/bank_1B.asm#L13093). Secret result IDs are in
[`SecretObjectTypes`](../jpdasm/bank_1B.asm#L14294).

## Doors and graves

Ordinary two-tile wooden entrances use these states:

| State | Map16 IDs |
| --- | --- |
| Open | `$0D9E`, `$0DA0` |
| Closed | `$0D9F`, `$0DA1` |

The source is recognized from the constituent 8x8 door graphics, so there is
no fixed list of source Map16 IDs. The handling is in
[`UseOverworldEntrance`](../jpdasm/bank_1B.asm#L12049) and
[`DrawWoodenDoor`](../jpdasm/bank_1B.asm#L14438).

Larger door and grave changes replace a 2x2 group:

| Change | Replacement Map16 IDs |
| --- | --- |
| Sanctuary door animation | `$0DA2-$0DAD` |
| Hyrule Castle door animation | `$0DB0-$0DB7` |
| Open grave with corpse | `$0DCB-$0DD0` |
| Open grave with stairs | `$0DCB`, `$0DCC`, `$0DD1-$0DD4` |
| Open grave with pit | `$0DCB`, `$0DCC`, `$0DD5-$0DD8` |

These groups are defined by
[`Map32UpdateTiles`](../jpdasm/bank_02.asm#L8153).

## Scripted and persistent changes

| Change | Map16 IDs written |
| --- | --- |
| Lumberjack tree removed | `$0E2C-$0E38` |
| Turtle Rock Light World portal | `$0212` |
| Bonk-rock and hidden-stair entrances | `$0912-$0915` |
| King's Tomb opened | `$0DCB`, `$0DCC`, `$0DD1-$0DD4` |
| Weather vane broken | `$0E1B-$0E1F` |
| Hyrule Castle gate opened | `$0DB8-$0DBD` |
| Dam drained | `$0DD9-$0DFF` |
| Thieves' Town entrance opened | `$0E15-$0E1A` |
| Pyramid hole opened | `$0E39-$0E41` |
| Palace of Darkness entrance | `$0E20`, `$0E22-$0E25`, `$0E27`, `$0E28`, `$0E2A`, `$0E2B` |
| Skull Woods entrance | `$0E00-$0E14` |
| Misery Mire entrance | `$0E42-$0E59`, `$0E5E-$0E71` |
| Turtle Rock entrance | `$0E72-$0E81` |
| Ganon's Tower entrance | `$0E82-$0E84`, `$0E86-$0E98`, `$0E9A-$0E9C` |
| Hammer-peg puzzle completion | `$0912-$0915` |

The live dungeon-entrance animations begin at
[`EntranceCutscene`](../jpdasm/bank_1B.asm#L14694). Persistent reload overlays
are collected in the `OverworldOverlay_*` routines in
[`bank_07.asm`](../jpdasm/bank_07.asm#L24645).

## Persistence

Selected localized changes are written to a temporary WRAM list by
[`MemorizeMap16Change`](../jpdasm/bank_04.asm#L4429). Each entry pairs an offset
in `$7E2000` with its replacement Map16 ID. The list exists so
[`RecoverTilesFromMirrorBonk`](../jpdasm/bank_02.asm#L21476) can restore live
changes after a failed mirror attempt rebuilds the current overworld map.

This is not persistent save data. A normal scrolling transition
[`clears the list`](../jpdasm/bank_02.asm#L7981), so a lifted bush returns after
leaving its area and coming back. Changes that survive area reloads instead
use saved screen-state bits and overworld overlays. The especially temporary
cut-grass and dirt-patch replacements `$0DBF` and `$0DC3` are not added to the
WRAM list at all.

## Generated replacements

`ALTTPRetiling/DynamicTiles/replacements.json` describes graphical variants for
the terrain interactions, doors, graves, and secrets above. It excludes the
scripted and persistent changes. Each variant contains a `before` tiling and
one or more `after_frames`, expressed using ALTTPRetiling palette, tile, and
flip values.

Rust will compile these tilings into the same global Map16 definition tables as
the ordinary area maps. The compilation order is:

1. Load the area maps and dynamic replacements.
2. Find occurrences of each `before` tiling in the source area maps.
3. Add the palettes and graphics used by its `after_frames` to those areas'
   asset requirements. These dependencies participate in the existing neighbor
   conflict calculations.
4. Allocate palette and graphics slots.
5. Intern the `before` and `after_frames` Map16 definitions together with the
   ordinary map definitions.
6. Emit runtime lookup tables containing the resulting Map16 IDs.

Adding the dependencies before allocation ensures that replacement graphics
and palettes are loaded even when they do not otherwise appear in an area's
initial map. Variants whose before tiling does not occur in the theme are not
allocated or emitted.

### Runtime lookup

The ASM will retain property-based interaction dispatch. Once a property has
identified an interaction, it will search only that interaction's small table
for the current before tiling. This replaces hard-coded Map16 ID comparisons
without adding table scans to unrelated terrain checks.

A fixed directory at `$A78300` contains 21 little-endian 24-bit group pointers
to table bodies in the generated-data region. Eight single-cell terrain groups
and two single-cell secret groups are populated; later-stage pointers remain
zero. A table entry identifies an interactable source cell and the variant
containing it:

```text
group:
    +$00  1 byte   entry count
    +$01           entries

entry:
    +$00  2 bytes  contacted before Map16 ID
    +$02  1 byte   signed X displacement from the contacted cell to the origin
    +$03  1 byte   signed Y displacement from the contacted cell to the origin
    +$04  3 bytes  variant descriptor pointer

variant descriptor:
    +$00  1 byte   width in Map16 cells
    +$01  1 byte   height in Map16 cells
    +$02  1 byte   frame count
    +$03           before Map16 IDs, row-major
    +...           after-frame Map16 IDs, row-major
```

After finding an entry, the resolver verifies the complete before footprint.
This distinguishes variants and handles repeated Map16 definitions within a
larger object. The same descriptor supports single-cell replacements, larger
footprints, and the existing multi-frame door animations.

### Interaction anchors

Rust will emit entries only for cells through which the corresponding action
can occur, rather than every cell in the graphical footprint. For
property-routed interactions, a cell is an anchor when one of its quadrants has
the group's distinctive property:

| Group | Property |
| --- | --- |
| `cut_grass` | `$40` |
| `dig_terrain` | `$48` |
| `green_bush` | `$50` |
| `heavy_bush` | `$51` |
| `hammer_peg` | `$27` |
| `small_gray_rock` | `$52` |
| `small_black_rock` | `$53` |
| `lift_sign` | `$54` |
| `large_gray_rock` | `$55` |
| `large_black_rock` | `$56` |
| `rock_pile` | `$57` |

The property dispatch has already selected the contacted quadrant, so the
runtime entry does not need a quadrant mask. Limiting entries to reachable
anchors reduces ambiguity and supports future variants containing decorative,
non-interactable cells.

Secret replacements use the property of the object concealing them. The
current data uses `$50` for holes, `$48`/`$52`/`$53` for portals, and
`$55`/`$57` for stairs. The same contacted cell can therefore select either
the ordinary replacement or the secret replacement after
`RevealOverworldSecret` determines whether a secret is present.

Bombable entrances are an exception because their before tilings use ordinary
properties. The existing secret-coordinate lookup will filter these before the
replacement table is searched. Doors and graves also enter through
event-specific paths; where those paths already know the structure's origin,
they can match a descriptor at that origin without displaced interaction
entries.

## Implementation stages

### 1. Single-cell terrain interactions (implemented)

- Load all dynamic replacement data.
- Add dynamic palette and graphics dependencies to the allocators.
- Intern the Map16 definitions for dynamic variants used by the theme.
- Emit the group directory, lookup entries, and descriptors.
- Add the shared ASM resolver.
- Replace the hard-coded IDs for grass, digging, bushes, pegs, signs, and small
  rocks.

This stage establishes the final table ABI, including arbitrary footprint and
frame counts, but exercises it only with ordinary single-cell replacements.

### 2. Single-cell secrets (implemented)

- Select secret replacement groups after `RevealOverworldSecret` succeeds.
- Add hole variants concealed by bushes.
- Add portal variants concealed by diggable terrain and small rocks.

This isolates the secret override behavior while continuing to use single-cell
footprints.

### 3. Larger interactions and secrets

- Match complete footprints and recover their origins from interaction anchors.
- Handle large gray rocks, large black rocks, and dash-smash rock piles.
- Handle secret stairs and bombable entrances.
- Replace the fixed `RockSmashReplaceOffset` and replacement layouts with the
  generated descriptors.

### 4. Doors and graves

- Handle ordinary wooden doors.
- Supply the existing Sanctuary and Hyrule Castle door timing with generated
  frames.
- Handle grave corpse, stairs, and pit replacements.

These use event-specific entry points rather than the ordinary terrain-property
dispatch. Existing event state and animation timing remain responsible for
when their generated frames are drawn.
