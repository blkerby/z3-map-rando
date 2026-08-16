# BG1 streamer

Status: milestone 4 is implemented; gameplay validation is pending before
milestone 5 begins.

## Goal

Reduce playable-overworld BG1 from a 64x64 to a 64x32 VRAM tilemap while
retaining a logical 64x64 grid of 8x8 overlay tiles. Use the existing BG2
streamer's row, column, Map16, and VRAM-address code rather than adding a
second renderer.

Rain remains a separate static 64x32 overlay. Dungeons and presentation modes
retain their existing tilemap layouts.

Overlay scroll policies and transitions between different policies are out of
scope. Preserve vanilla scroll behavior; the streamer only follows the final
BG1 scroll coordinates it receives.

## BG1 coordinates

[`LoadSubOverlayFlatMap16`](../patches/src/overworld_map_data.asm#L106) places one
32x32 Map16 map at `$7E4000`. Rows retain the existing 64-Map16-wide WRAM
stride, so the populated upper-left quadrant represents a 64x64 grid of 8x8
tiles.

Vanilla BG1 is a 64x64 tilemap. Its scroll therefore addresses that overlay
modulo 512 pixels:

```text
logical_x = (final_bg1_hofs >> 3) & $003F
logical_y = (final_bg1_vofs >> 3) & $003F
```

Use finalized scrolls `$0120` and `$0124`, which include shake. Do not subtract
the current overworld area's world-space origin as BG2 does.

This follows from the existing data and scroll paths:

- `LoadSubOverlayFlatMap16` always loads the selected synthetic overlay at
  logical Map16 coordinate `(0,0)`.
- [`BuildBGOverlayFromMap16`](../jpdasm/bank_02.asm#L21110) expands those Map16
  rows and columns into matching positions in the vanilla 64x64 BG1 tilemap.
- [`Overworld_SetFixedColAndScroll`](../jpdasm/bank_0B.asm#L16702) writes
  `$E0/$E6` directly to BG1. The Pyramid deliberately uses values such as
  vertical `$0600-$06C0`; their low nine bits select pixels `$000-$0C0` of
  the same overlay.
- Fog can increase the scroll indefinitely. Vanilla hardware wraps it through
  the same 512-pixel tilemap, so the logical source must wrap at 64 tiles too.

Using `$007F`, or subtracting the area's BG2 origin, would select unpopulated
parts of `$7E4000-$7E5FFF` after the BG1 scroll crosses a 512-pixel boundary.

## Shared renderer

Keep layer-specific scroll and transition entry points, but make the tile
emitters layer-neutral. An active renderer call supplies:

| Parameter | BG2 | BG1 |
| --- | ---: | ---: |
| Map16 source offset from `$7E2000` | `$0000` | `$2000` |
| VRAM tilemap base | `$0000` | `$1000` |
| Logical coordinate mask | `$007F` | `$003F` |

The common Map16 loops can retain `LDA.l $7E2000,X`. Add the configured source
offset when calculating the first Map16 address for a segment; subsequent
pair iterations remain unchanged. Add the configured VRAM base once in the
VRAM-address routine. There must be no per-tile layer branch or indirect load.

The configuration is temporary call scratch, not state retained between
frames. Set it only after a scroll comparison determines that work is needed,
or before a bulk render.

Share these operations:

- build a bulk row list;
- emit a 33-tile row;
- emit a horizontal segment;
- emit a 29-tile column;
- emit a vertical segment;
- calculate a 64x32 VRAM address;
- terminate and request a nonempty `$1100/$18` list.

Keep these operations layer-specific:

- preserving the overwritten BG1 or BG2 scroll stores;
- calculating the logical window origin;
- checking whether BG1 is enabled or is rain;
- configuring `BG1SC` or `BG2SC`;
- deciding when bulk loads and transition margins are needed;
- immediate BG2 Map16 changes on `$1000/$14`.

## `$1100` list ownership

Gameplay uses one list cursor:

```text
BG2 horizontal -> BG2 vertical -> BG1 horizontal -> BG1 vertical -> finish
```

Each stage appends zero or more entries. BG1 horizontal saves the full Y
cursor in direct-page scratch before returning to the 8-bit-index caller, and
BG1 vertical restores it. Only the last stage terminates a nonempty list and
sets `$18`. The existing main-loop/NMI barrier remains the only synchronization
mechanism.

A BG1 and BG2 row plus column require about 280 bytes together, well within
`$1100-$197F`. Two 33x29 bulk windows do not fit together: each is 2148 bytes.
Schedule BG1 and BG2 bulk uploads on separate loading frames.

## Milestones

### 1. Refactor BG2 without changing behavior

Extract the existing BG2 bulk, row, column, segment, and VRAM-address code into
the shared routines described above. Keep the current BG2 hooks, list order,
bulk timing, transition handling, mirror margins, and list finalization.

The BG2 wrappers select the BG2 parameters and call the shared core. Do not add
BG1 hooks, change `BG1SC`, or disable any vanilla BG1 producer in this
milestone.

Before continuing:

1. Build all patches and run `theme_check`.
2. Confirm that representative bulk and edge `$1100` lists are byte-for-byte
   unchanged.
3. Retest ordinary and diagonal movement, all four area transitions, dynamic
   Map16 changes, mirror, portal, whirlpool, world-map return, and interior
   return.
4. Confirm that BG2 still never writes VRAM `$0800-$0FFF`.

This is a hard test gate. Fix any BG2 regression before adding BG1 behavior.

### 2. Establish clean BG1 ownership

For playable overworld only:

- select `BG1SC=$11`;
- retain the logical flat overlay load into `$7E4000`;
- skip `BuildBGOverlayFromMap16` and its full `$17=$04` upload;
- skip vanilla's later `$7E4000-$7E407E` rendered-row clears;
- disable the old mirror and whirlpool `$17=$0C/$0D` overlay uploads;
- retain overlay selection, BG1 scroll, color math, and enable controls.

Restore `BG1SC=$13` on underworld entry and preserve presentation paths such
as credits. Validate that visible overworld overlays remain blank while BG2,
sprites, HUD, and transitions continue normally.

### 3. Add BG1 bulk rendering

Add a thin BG1 bulk wrapper that:

1. skips disabled overlays and rain;
2. uses the `BG1SC=$11` layout selected unconditionally at overworld BG1 load;
3. calculates the 64x64 BG1 origin from `$0120/$0124`;
4. selects the BG1 renderer parameters;
5. calls the common 33x29 bulk builder.

Schedule BG1 and BG2 bulk lists on separate loading frames. Cover ordinary
overworld entry first, then mirror, portals, whirlpools, and world-map return.
The display must remain hidden or at its transition color until both layers
are resident.

Implementation: the BG1 bulk list replaces vanilla's earlier overlay upload.
The existing state machines already load the overlay one frame before they
build BG2, so no new phase state is needed. Mirror is the exception: BG2
uploads in step 7, then BG1 uploads in step 8 after its scroll is finalized.

### 4. Add gameplay BG1 streaming

Hook the finalized BG1 horizontal and vertical stores after the existing BG2
hooks. Compare old and new 8x8 coordinates and return immediately on zero.
Append an entering 33-tile row and/or 29-tile column through the shared
emitters.

BG1's observed gameplay movement changes one row at a time. Other
discontinuities use the bulk path.

Test the Pyramid across its complete camera range, Light and Dark Death
Mountain, both fog overlays, Lost Woods, special overworlds, diagonal motion,
screen shake, and simultaneous BG1/BG2 edge crossings.

Implementation: the four gameplay stages share one `$1100` list in BG2
horizontal/vertical then BG1 horizontal/vertical order. BG1 vertical
terminates the combined list. The full cursor is passed between the BG1 stages
through direct-page scratch because the caller uses 8-bit index registers.

### 5. Handle transition exposure

Audit per-scanline mirror HDMA after ordinary streaming works. Add BG1 margin
columns only where the wave can expose tiles outside the 33x29 window. Reuse
the common column emitter; do not add resident-window state.

Retest mirror and portal transitions at every camera edge, especially the
Pyramid and castle where BG1 does not simply follow BG2.

### 6. Install the static rain overlay

Apply the separately developed 64x32 rain tilemap and offset sequence. Rain
does not invoke the BG1 streamer. Test rain animation, thunder flashes,
Misery Mire state changes, mirror, and interior exits.

## Completion checks

- Every playable-overworld non-rain overlay uses the shared streamer.
- BG1 never writes VRAM `$1800-$1FFF`.
- BG2 never writes VRAM `$0800-$0FFF`.
- Frames without an 8-pixel boundary crossing perform no gameplay tilemap
  upload for that layer.
- BG1 and BG2 can append edge work in the same frame.
- Bulk NMI budgets and transition sequencing are recorded after final testing.
