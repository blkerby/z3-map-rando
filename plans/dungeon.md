# Dungeon rendering and room data

Status: deferred. The current randomizer does not need custom dungeon layouts
or larger dungeon character sets, so this work is not planned yet.

## Direction

If dungeon generation becomes necessary, move dungeon gameplay to the same
VRAM layout as the overworld and replace the vanilla room-object interpreter
with compiled room data. These changes belong together in the eventual design:
the unified layout provides 960 shared BG1/BG2 characters, while compiled room
data permits arbitrary layouts instead of layouts expressible only through the
vanilla object catalog.

Do not require every presentation module to use the gameplay layout. File
select and name entry, for example, intentionally use a different BG3 shape.
Keep any necessary presentation layouts explicit and switch them only at their
module boundaries.

## Current dungeon representation

The vanilla room loader interprets room objects and expands them into complete
64x64 maps of final 8x8 tilemap words:

- `$7E2000-$7E3FFF`: BG2, called layer 1 by dungeon code.
- `$7E4000-$7E5FFF`: BG1, called layer 2 by dungeon code.

Dungeons do not retain an overworld-style Map16 intermediate. The current
quadrant loaders copy 32x32 portions of these maps into the vanilla 64x64 VRAM
tilemaps.

Collision is stored independently as one byte per 8x8 tile:

- `$7F2000-$7F2FFF`: layer 1/BG2.
- `$7F3000-$7F3FFF`: layer 2/BG1.

`LoadBasicObjectAttributes` initially derives these bytes from the character
index and flip bits in each tilemap word, using the tile-property table at
`$7EFE00`. Later passes override collision for doors, stairs, pegs, and other
special objects. Collision checks read these complete 64x64 WRAM maps; they do
not inspect VRAM.

Ordinary dynamic changes also preserve the WRAM tilemaps. A drawing routine
updates `$7E2000` or `$7E4000`, then `Underworld_PrepNextTilemapUpdateDMA`
copies the affected words into a stripe command at `$7E1100`. NMI later writes
that snapshot to VRAM. Collision changes are made separately when required.
Animated characters and some special effects use dedicated upload paths.

## Compiled rooms

The future build step should compile vanilla and generated rooms into their
initial flat tilemap and collision maps. The runtime would load these maps
directly instead of interpreting drawing objects. This is analogous to
removing overworld Map32 data: the generated representation should express the
final layout rather than preserve an unnecessary construction language.

Flat collision maps are sufficient for behavior local to a tile, including
ordinary walls, pits, slopes, water, liftables, switches, and pegs. Some
stateful features also use auxiliary tables built by the object interpreter.
For example, a door's collision byte identifies a slot whose table entries
provide its type, tilemap location, and direction; stairs and hidden chests
similarly retain locations needed after room construction.

Compiled rooms can populate the existing auxiliary tables directly, or replace
them with generic tilemap/collision patches and transition data. This does not
require retaining the drawing-object system or designing a new semantic object
catalog. Choose the smaller representation only when generated dungeons need
it.

Keep the flat data uncompressed initially. Measure the complete dungeon bundle
before choosing a compression format or adding residency machinery.

## Unified gameplay VRAM layout

Dungeon gameplay would use the milestone 9A layout documented in `vram.md`:

- BG1/BG2 characters at `$0000-$3BFF`.
- BG3 tilemap at `$3C00-$3FFF`.
- OBJ characters at `$4000-$5FFF`.
- BG2 tilemap at `$6000-$67FF`, 64x32.
- BG1 tilemap at `$6800-$6FFF`, 64x32.
- BG3 characters at `$7000-$7FFF`.

Static and animated dungeon BG-character uploads would move down from their
vanilla `$2000` base. HUD, text, and other BG3 tilemap destinations would move
from `$6000` to `$3C00`. OBJ and BG3 character locations would not change.

Once all gameplay uses this layout, overworld-versus-dungeon checks can be
removed from shared graphics and NMI paths. Any remaining checks should
distinguish an explicitly selected presentation layout instead.

## Dungeon tilemap streaming

Each physical BG1/BG2 tilemap holds 64x32 tiles while each WRAM room map remains
64x64. A forced-blank load can render the 32-row band containing the initial
viewport. Display-on movement must upload a row or column whenever the camera
crosses an 8-pixel boundary.

A direct mapping may be sufficient:

```text
physical x = logical x & 63
physical y = logical y & 31
```

This must be verified across every room-transition direction before relying on
it. Horizontal transitions can retain outgoing and incoming 32-tile screens
side by side. During vertical transitions, logical rows separated by 32 alias
the same physical row; incoming rows must replace outgoing rows only after the
outgoing rows leave the viewport.

BG1 and BG2 require independent streaming state. Dungeon code deliberately
allows their scroll positions to differ for several room effects, rather than
always calling `UnderworldSyncBG1and2Scroll`.

Dynamic stripes must translate a logical tile position to its currently
resident physical position. An update to an offscreen logical row must not
overwrite the other logical row which aliases it modulo 32. The existing
stripe mechanism may be reusable, but its buffer capacity and interaction with
other NMI requests must be measured before choosing between it and a dedicated
tilemap-transfer handler.

The source tilemaps already contain final 8x8 words, so the dungeon renderer
does not need the overworld renderer's Map16 expansion. It may still reuse its
physical-address and wrap-splitting logic if doing so is simpler than a small
direct-tile renderer.

## Transition and effect coverage

Validation must include more than ordinary room-edge scrolling:

- Initial room loads and dungeon entrances.
- Free camera movement within large rooms.
- Intra-room and inter-room transitions in all four directions.
- Straight, spiral, and wide stairs; pits and floor changes.
- Independently scrolling BG1/BG2 effects and moving walls.
- Doors, shutters, bomb walls, torches, water, overlays, push blocks, chests,
  crystal pegs, and other dynamic tile or collision changes.
- Dungeon map, save-and-quit, menus, credits, and other transitions between
  gameplay and presentation layouts.

The likely NMI limit is coordination rather than transfer size. Streaming a
new boundary is smaller than vanilla's 2 KiB quadrant uploads, but it can
coincide with dynamic stripes, animated characters, HUD work, or asset loads.

## Possible implementation order

1. Compile vanilla rooms to flat tilemap, collision, and interaction data, and
   compare the results against the vanilla room builder without changing the
   game.
2. Load the compiled data into the existing WRAM maps while retaining the
   vanilla VRAM layout and quadrant renderer.
3. Select the unified layout for dungeon gameplay, rebase graphics and BG3,
   and add forced-blank bulk rendering plus ordinary camera streaming.
4. Add scrolling transitions and convert dynamic stripe destinations.
5. Cover special effects and transitions, then remove the obsolete object and
   quadrant-loading paths and simplify layout-dependent hooks.

Each runtime step should remain playable and continue to reproduce vanilla
rooms before generated dungeon content is introduced.
