# BG2 streamer

Status: complete. All five implementation milestones are active in
`bg_streamer.asm`.

## Scope

Start with playable-overworld BG2. Do not reuse the experimental BG1
streamer. Do not change dungeon, BG1, BG3, logical map, collision, or event
behavior.

The streamer owns every overworld BG2 tilemap write. Vanilla may continue to
load and modify the logical 64x64 grid of 16x16 Map16 IDs at
`$7E2000-$7E3FFF`, but it must not expand those IDs into BG2 VRAM. After
Map16 expansion, the logical map is 128x128 8x8 tiles.

The physical BG2 tilemap will be 64x32 tiles. Logical Y wraps into physical
Y with `Y & 31`, and logical X wraps into physical X with `X & 63`.

## Constraints

- No margin or resident-window state.
- No new transfer queue or per-frame synchronization mechanism.
- Bulk and camera-edge rendering use the existing `$1100/$18` NMI
  mechanism.
- Localized runtime Map16 changes retain the existing `$1000/$14` mechanism.
- Gameplay rendering prepares at most one entering row and one entering
  column per frame.
- NMI only transfers data already prepared by the main loop.
- Camera discontinuities are handled by a bulk `$18` list, not by gameplay
  edge streaming.
- The renderer uses the final BG2 scroll values, including shake.

## Frame synchronization and uploads

The game already synchronizes the main loop with NMI through `$12`:

1. Main prepares the next frame, including upload buffers.
2. Main clears `$12`.
3. The next NMI checks that `$12` is clear, so it consumes the prepared buffers and sets `$12`.
4. Main waits until `$12` is set before proceeding.

If main misses a frame, NMI skips the normal update work rather than letting
main overwrite a buffer being transferred. The BG2 streamer therefore needs
no additional lifetime or completion state.

The game has two suitable upload mechanisms. They do not use the same list
format.

With `$14=1`, `$1000` is the current byte offset and the stripe list starts
at `$1002`. Each stripe contains:

```text
2 bytes: VRAM destination, high byte first
2 bytes: direction, RLE flag, and transfer length
N bytes: tile data
```

An `$FF` byte marks the end. This is the existing format used by immediate
overworld Map16 changes. It supports horizontal and vertical stripes,
variable lengths, repeated data, and appending several changes in one frame.

With `$18` set, `$1100` contains an arbitrary DMA list. Each entry contains:

```text
2 bytes: VRAM destination
1 byte:  VMAIN value
1 byte:  transfer length in bytes
N bytes: tile data
```

A `$FFFF` word marks the end. NMI processes the whole list and clears `$18`.
Bulk loads and movement streaming use this format.

Vanilla also uses `$1100` with NMI mode `$17=$03`, but that is a third,
incompatible format: one common transfer header at `$1100`, followed by
fixed-size transfers starting at `$1102`. Milestone 1 removes the overworld
producers of that format before the streamer reuses `$1100/$18`.

Within NMI, `$1000/$14` is processed before `$1100/$18`.

The streamer emits two `$1100` entries for a 33-tile row and up to two for a
wrapped column, followed by `$FFFF`. A bulk window contains 58 row entries.
This reuses the game's list and NMI handler; it adds no queue format or ready
bitfield.

## Camera comparison

The PPU receives BG2 horizontal scroll from `$011E` and vertical scroll from
`$0122`. Near the end of the main loop, the game replaces them with:

```text
new_h = $E2 + $011A
new_v = $E8 + $011C
```

At each store, `$011E` or `$0122` still contains that axis's value from the
previous frame. Separate horizontal and vertical hooks can therefore compare
old and new finalized positions without storing previous-camera state.

For each axis:

```text
old_tile = old_scroll >> 3
new_tile = new_scroll >> 3
```

- Equal: prepare no edge on that axis.
- `new_tile = old_tile + 1`: prepare the bottom or right edge.
- `new_tile = old_tile - 1`: prepare the top or left edge.
- A vertical change of `-2`: prepare both entering top rows.
- Any larger change violates the gameplay-streaming invariant and must not
  trigger a bulk transfer.

The final frame of a northward area transition moves BG2 by ten pixels and
can cross two tile rows. Other normal movement crosses at most one tile
boundary per axis per frame.

## Tile lookup

For logical 8x8 coordinate `(x, y)`:

1. Read Map16 ID `(x >> 1, y >> 1)` from the 64x64 logical Map16 map at
   `$7E2000`.
2. Use `x & 1` and `y & 1` to select the top-left, top-right, bottom-left, or
   bottom-right 8x8 word from the split Map16 definition tables.

The physical BG2 VRAM word address is:

```text
$0000
+ (y & 31) * 32
+ (x & 31)
+ ((x & 32) ? $0400 : 0)
```

A 64x32 SNES tilemap consists of two separate 32x32 screen blocks. Within
each block, the tile after X 31 is X 0 of the next row, not X 32 of the same
row in the other block. The physical addresses of logical `(31, y)` and
`(32, y)` are therefore not adjacent. Wrapping from physical X 63 to 0 is
also discontinuous.

A horizontal transfer must split at either of those boundaries. Every
33-tile row crosses exactly one of them, so it always requires two `$18`
entries. A vertical transfer uses VMAIN `$81` and must split only when
physical Y wraps from 31 to 0.

## Bulk renderer

An overworld load path calls the bulk renderer after the logical map, events,
recovered changes, and camera position are final. Ordinary forced-blank loads
already do this; mirror warp will use the same renderer.

World-map return runs the same renderer in overworld submodule `$21`, one
forced-blank frame after submodule `$20` restores BG1.

The rectangle always starts at:

```text
left = scroll_x >> 3
top  = scroll_y >> 3
```

It is always 33x29. When the scroll is tile-aligned, the rightmost column
and bottom row are completely offscreen; drawing them is harmless and keeps
the calculation fixed. This is fixed overdraw, not a tracked margin.

The renderer writes the final `$18` list directly into `$1100-$197F`. For
each row it writes:

1. A four-byte header and tile data up to the screen-block boundary.
2. A second four-byte header and the rest of the row.

After 29 rows it writes the `$FFFF` terminator and sets `$18`. It also sets
`$0710` to suppress the normal graphics DMA group during this larger update;
the `$18` handler clears both controls after transferring the complete list.

The worst-case size is fixed:

```text
tile data:      29 * 33 * 2 = 1914 bytes
entry headers:  29 * 2 * 4  =  232 bytes
terminator:                       2 bytes
                               ----------
total:                         2148 bytes
available: $1980 - $1100 =     2176 bytes
unused:                          28 bytes
```

The renderer needs no separate row buffer and performs no direct VRAM
writes. Ordinary loads and mirror warp use the same NMI path.

Mirror warp rebuilds its destination's logical Map16 data and uses the common
bulk renderer to upload all 29 rows in animation step 7. The following
`$0C/$0D` pair draws the BG1 transition overlay and remains unchanged.

Its horizontal-scroll HDMA can displace scanlines by `-9..+9` pixels. A
small `$1100/$18` upload supplies the additional columns that can become
visible outside the bulk window and repairs its out-of-bounds right column at
an area edge. The destination margins load in animation step 10, and the
source margins load during initialization. HDMA remains disabled while the
screen is held white for loading. Palette progress and the software wave
state both pause so their relative vanilla timing is preserved. With those
two tasks paused, the animation advances one loading step per frame instead
of retaining vanilla's alternating loading/palette cadence.

Whirlpool warp prepares the same bulk list while the screen is solid blue.
It uploads the BG1 overlay half first, preserves the prepared `$1100` list,
then uploads BG2 on the next frame. This prevents `$17` and `$18` from
running in the same NMI; the HUD remains enabled.

## Gameplay renderer

The horizontal and vertical main-loop hooks compare the previous and new
finalized scroll positions. They may prepare:

- one or two 33-word rows at the entering top or bottom edge;
- one 29-word column at the entering left or right edge;
- both when both axes cross a tile boundary;
- neither when neither axis crosses a tile boundary.

For the new top-left tile `(left, top)`:

```text
moving up:    row y = top,      x = left .. left + 32
moving down:  row y = top + 28, x = left .. left + 32
moving left:  column x = left,      y = top .. top + 28
moving right: column x = left + 32, y = top .. top + 28
```

For the two-row north-transition move, scrolling emits `top` and `top+1`.

When an area transition loads a new logical map, an east transition preloads
the current offscreen `left+32` column. Its first scroll step assumes that
fixed-overdraw edge is already resident. Southward transitions need no
equivalent preload because their `$x11E` to `$x120` alignment correction
crosses a tile boundary and invokes normal vertical streaming.

The renderer appends the required transfer records and data to the existing
`$1100` arbitrary DMA list, terminates the list with `$FFFF`, and sets `$18`.
The horizontal routine starts with `Y=0` and returns the list cursor in Y.
The vertical routine receives that cursor, appends rows, and finishes the
list, so a row and column can coexist without persistent state.
The column may use the previous vertical tile; any newly exposed corner
tiles are overwritten by the rows appended afterward.

No rectangle bounds, overlay ID, or initialization state are retained.

## Removing vanilla BG2 drawing

Milestone 1 must block all playable-overworld BG2 tilemap population while
preserving logical map updates:

- Initial and reload builds through `BuildOverworldFromMap16`, including the
  mode `$04` upload requested by `LoadAndBuildOverworldScreen`.
- Area-transition and ordinary scrolling stripes built under
  `TriggerAndFinishMapLoadStripe_*`, `OverworldTransitionScrollAndLoadMap`,
  and `OverworldHandleMapScroll`, including NMI mode `$03`.
- Immediate stripes produced by `DrawMap16Anywhere`,
  `DrawPersistentMap16`, and `AlterMap16Hardcore`. These routines must still
  update `$7E2000` and persistence data. Milestone 5 will restore their
  visible writes using the corrected 64x32 address calculation.

Do not globally disable shared NMI handlers: mode `$04` also draws BG1
overlays, and the generic stripe machinery has non-BG2 users. Patch the
overworld BG2 producers or their requests instead.

Tilemap clearing is allowed. Character graphics DMA is outside this
ownership boundary.

## Milestones

### 1. Vanilla BG2 disabled

Patch out every playable-overworld vanilla BG2 tilemap producer and upload
request. Add no renderer.

Validation:

- Enter the overworld from a house or dungeon.
- Walk and perform scrolling area transitions in all four directions.
- Trigger visible Map16 changes such as lifting or hammering terrain and an
  animated entrance.
- BG2 remains blank throughout; BG1, BG3, sprites, and dungeon rendering
  remain unchanged.
- Normal blank-tilemap clears remain enabled. VRAM logging shows no later
  non-clear writes to the BG2 tilemap range during overworld gameplay.

### 2. Bulk renderer

Set playable-overworld BG2 to 64x32 and implement only the bulk renderer.

Validation:

- The initial view is correct after entering from each full-load path,
  including mirror warp.
- Moving far enough to reveal a new edge exposes blank or stale tiles,
  demonstrating that no streaming path is active yet.
- The bulk path produces a 2148-byte `$1100` list with 58 entries and one
  terminator.
- `$18` transfers the complete visible window during one NMI without running
  past vblank.

### 3. Vertical streaming

Prepare and upload the one or two entering 33-tile rows when finalized
vertical scroll crosses an 8-pixel boundary.

Validation:

- Ordinary north/south movement and north/south area transitions reveal
  correct rows.
- Shake does not reveal stale rows.
- Horizontal movement remains intentionally unsupported.

### 4. Horizontal streaming

Prepare and upload one entering 29-tile column when finalized horizontal
scroll crosses an 8-pixel boundary. Support a row and column in the same
frame.

Validation:

- Movement and scrolling area transitions work in all four directions.
- Diagonal movement and shake can append both transfers to `$1100` without
  overwriting either.
- NMI performs no BG2 work on frames with no tile-boundary crossing.

### 5. Runtime Map16 changes

Retain the existing `$1000/$14` immediate-stripe mechanism, but replace its
overworld BG2 destination calculation with the 64x32 mapping. A Map16 tile is
aligned to an even 8x8 coordinate, so each of its two-tile rows remains
within one 32x32 screen block and needs no split.

NMI processes `$14` before `$18`. Build camera-streaming data after logical
Map16 changes have been applied so a later `$18` transfer cannot overwrite
an immediate update with stale tile data.

Validation must cover liftable terrain, hammer changes, bomb entrances,
event overlays, and animated entrances, including a frame that also streams
a camera edge.
