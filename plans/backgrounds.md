# Retiled overworld backgrounds

## Goal

Make `theme_check` compile the BG1 layers, composition mode, and camera behavior
stored in ALTTPRetiling. Generated tables should replace vanilla's
screen-specific BG1 selection, color-math, and scrolling branches wherever the
retiling project provides data.

Rain remains state-dependent and uses a separate static background. Area `$80`
remains a small hard-coded exception because its two entrances use the same
area ID as two functionally different areas.

This plan supersedes the logical BG1 dimensions and preserved-scroll-policy
assumptions in [bg1_streamer.md](bg1_streamer.md). Its shared rendering and NMI
queue remain useful.

## Source model

ALTTPRetiling area JSON provides:

- ordered layers assigned to BG1 or BG2;
- a composition mode: `none`, `half_add`, or `backdrop`;
- horizontal and vertical camera-follow multipliers;
- horizontal and vertical automatic drift rates.

Layers targeting the same PPU background resolve at 8x8-tile granularity. A
placement in a higher layer replaces the complete lower placement at that
coordinate, including its color-zero pixels. Only the final BG1 and BG2 images
are composited per pixel by the PPU. Future edge-variant layers will follow the
same rule and will be resolved when a seed chooses its variants.

Ordinary areas currently have at most one applicable BG1 layer. Area `$80` has
two alternatives, `Grove Fog` and `Bridge Shadow`; they are not composited.

## Logical BG1 map

An authored BG1 uses the same logical capacity as BG2: a 64x64 grid of Map16
tiles in `$7E4000-$7E5FFF`. This is a 128x128 grid of 8x8 tiles, four times the
area of vanilla's populated 32x32-Map16 overlay.

`theme_check` will:

1. resolve the selected BG1 layers into one optional placement per 8x8 cell;
2. divide the result into 2x2 cells;
3. intern those cells as Map16 definitions using the allocated character and
   palette slots;
4. use a transparent 8x8 word for absent placements and zero collision
   properties for new BG1-only Map16 definitions;
5. emit four flat 32x32-Map16 quadrants for every active BG1 variant.

The quadrants use the existing flat-map format and can share its map-data
interner. A parallel 160-entry pointer table maps screen IDs to BG1 quadrants.
A zero pointer means that the retiling project supplies no BG1 for that screen.

All four quadrants are initialized when BG1 is active, including areas whose
playable BG2 region is smaller. Transparent or deliberately filled quadrants
must not retain data from the previous area.

### Pyramid

The Pyramid BG1 should contain the tan fill needed below its detailed artwork.
With valid data across the complete logical BG1 map, its camera can use the
same wrapping coordinate calculation as other backgrounds. The vanilla
vertical `$06C0` clamp and related special cases can then be removed after a
visual comparison confirms the authored fill covers every reachable camera
position.

## Graphics and palette allocation

BG1 placements participate in the existing palette and stable-character
allocation with BG2 and dynamic tiles. Only placements surviving same-background
layer resolution are required for a seed, except runtime alternatives such as
area `$80`, whose assets must coexist.

The compiler continues to emit the existing six palette rows and character-row
payloads. BG1 Map16 words refer to those allocated destinations, so runtime
loading needs no second graphics or palette mechanism.

Allocation checks must report:

- an area or neighboring-area set that cannot fit in six palette rows;
- stable character rows exceeding the existing 960-character capacity;
- a BG1 Map16 word referring to an asset absent from that area's full-load
  record;
- the fullest palette allocation and highest character slot after BG1 and rain
  dependencies are included.

## Area background table

Each playable screen receives generated background configuration containing:

- BG1 quadrant pointers, or no authored BG1;
- composition mode;
- X and Y camera-follow values;
- X and Y drift values.

The table is indexed by `$8A`. Runtime code reads it during the existing
overworld background-loading phase, loads the complete logical BG1 map, and
configures BG1, the main/subscreen selection, and color math. Screen IDs should
not otherwise select presentation behavior in ASM.

Edge variants do not require runtime condition records. The randomizer chooses
them before patching, and the patcher writes the resulting direct pointers.

### Area `$80`

Area `$80` keeps one narrow exception matching vanilla:

- `$A0 == $0181` selects `Bridge Shadow`;
- the other entrance selects `Grove Fog` unless `$7EF300 & $40` has removed
  the Master Sword grove overlay;
- one BG2 map continues to contain both functional halves of the area.

The exception should only select between generated pointers or no BG1. The
ordinary loader, renderer, allocation, and camera code remain shared.

## Camera behavior

Camera-follow values are dimensionless multipliers. Drift values are signed
pixels per frame. For each axis, the active background position advances as:

```text
background_delta = camera_delta * follow + drift
```

The runtime keeps fractional background positions in fixed-point and derives
the integer BG1 scroll and streamer source coordinate from them. The current
follow choices (`0`, `0.25`, `0.5`, `1`, and `1.5`) are shift-and-add friendly;
drift still needs fractional precision for values such as `0.125`.

On a full area load, initialize the position from the camera's local coordinate
within the area multiplied by `follow`. While active, add unshaken camera
movement and drift, then apply screen shake to the presented BG1 scroll. Reset
the fractional drift phase on a full load unless gameplay comparison shows a
vanilla path that requires continuity.

The full logical map uses the same 7-bit 8x8 coordinate mask as BG2. Coordinates
wrap within the 128x128-tile source; there are no Pyramid-specific clamps.
Authored transparent or filled regions determine what appears at every
reachable position.

The existing shared BG1/BG2 streamer remains responsible for the rolling PPU
tilemap. BG1 changes from the old `$003F` source mask to `$007F`; its VRAM base
and BG1-specific scroll hooks remain distinct from BG2.

## Rain

Rain is not represented as an ordinary ALTTPRetiling BG1 layer. It remains a
static 64x32-8x8-tile background with its existing four-step offset animation,
splashes, thunder flashes, sound, and state triggers. It bypasses the ordinary
BG1 streamer.

Rain is selected only when:

1. the current area and game state satisfy the intended vanilla rain predicate,
   including Misery Mire; and
2. the generated background table has no authored BG1 for the area.

Two sets of 8x8 rain graphics reproduce the same shapes with pixel indices
appropriate to their allocated ALTTPRetiling palettes:

| Context | Palette | Nontransparent colors and indices |
| --- | ---: | --- |
| Light World | `3` (`Light World 1`) | `1=[5,5,5]`, `10=[6,9,15]`, `11=[7,12,21]`, `12=[14,22,30]` |
| Misery Mire | `8` (`Dark World 1`) | `11=[4,6,2]`, `13=[0,10,5]`, `14=[6,17,12]` |

`theme_check` will construct these tiles and their Map16 definitions, add the
applicable palette and character dependencies to rain-capable areas, and emit
the existing optimized rain layout using the new definitions. No new rain
palette is needed.

## Implementation phases

### 1. Compile BG1 maps without runtime changes

- Parse composition and camera fields in `theme_check`.
- Resolve layers by complete 8x8 placements within each background.
- Build BG1 Map16 definitions and four flat quadrants per variant.
- Include the resulting assets in existing allocations.
- Emit allocation and map-bound diagnostics, but do not change ASM.

This phase establishes whether the six palette rows remain sufficient using
the assets that the final maps actually reference.

### 2. Emit generated background tables

- Add the parallel BG1 quadrant-pointer table.
- Encode composition, follow, and drift values in a compact per-screen record.
- Write both tables into declared ROM ranges with overlap and bounds checks.
- Add offline checks that every active BG1 has four initialized quadrants and
  all referenced assets.

The game still follows vanilla background selection in this phase.

### 3. Load authored BG1 maps

- Make the overworld BG1 load read the generated table.
- Load all four quadrants into `$7E4000-$7E5FFF`.
- Clear BG1 when the table has no authored map.
- Add the area `$80` pointer-selection exception.
- Retain vanilla composition and scrolling temporarily to isolate map-loading
  failures.

### 4. Drive presentation and camera from data

- Configure BG1 enablement, main/subscreen selection, and color math from the
  generated composition mode.
- Implement fixed-point follow and drift for both axes.
- Change the BG1 streamer source mask to `$007F`.
- Use the full authored Pyramid map and remove its clamp after validation.
- Cover full loads, gameplay streaming, scrolling transitions, mirror and
  portal travel, whirlpools, world-map return, and interior return.

### 5. Add generated rain assets

- Generate Light World and Mire rain tiles with their respective palette
  indices.
- Rebuild the optimized rain Map16 layout with allocated definitions.
- Add rain dependencies only to intended rain-capable areas without authored
  BG1.
- Route the existing rain predicate to the static rain path and keep it outside
  ordinary BG1 streaming.
- Test Link's House, Misery Mire, animation phases, splashes, thunder, state
  changes, and interior exits.

### 6. Remove superseded vanilla branches

- Delete or bypass hard-coded overlay selection, composition, camera-follow,
  drift, and Pyramid clamp paths now covered by generated data.
- Retain only the area `$80` exception and presentation paths outside playable
  overworld gameplay, such as dungeons and credits.
- Run allocation checks, patch assembly, `engine_check`, and representative
  emulator tests for every composition and camera mode.

## Completion checks

- Authored BG1 occupies a fully initialized 64x64 Map16 logical map.
- BG1 and BG2 layers resolve by 8x8 placement before PPU composition.
- Composition, follow, and drift come from generated area data.
- Pyramid traverses its full camera range without a clamp or exposed garbage.
- Areas with authored BG1 never select rain.
- Intended Light World and Mire rain paths use the correct palette-indexed
  tiles and retain animation, thunder, and sound.
- Area `$80` behaves like vanilla without introducing a general runtime variant
  system.
