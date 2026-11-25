# Overworld backgrounds

The patcher compiles each area's BG1 layers, composition mode, and camera behavior from ALTTPRetiling. Generated tables replace vanilla's screen-specific BG1 selection, color-math, and scrolling branches. The patcher builds the maps in [`retiling.rs`](../../crates/patcher/src/retiling.rs) and [`rom.rs`](../../crates/patcher/src/rom.rs); the engine loads them in [`overworld_map_data.asm`](../../patches/src/overworld_map_data.asm) and streams them in [`overworld_bg_tilemaps.asm`](../../patches/src/overworld_bg_tilemaps.asm). Table addresses are in [`symbols.inc`](../../patches/src/symbols.inc) and the [ROM map](../../patches/rom_map/banks_A0-BF.txt).

## Source model

An ALTTPRetiling area provides ordered layers, each assigned to BG1 or BG2, a composition mode (`none`, `half_add`, or `backdrop`), and horizontal and vertical camera-follow multipliers and drift rates.

Layers on the same PPU background resolve at 8x8-tile granularity: a placement in a higher layer replaces the complete lower placement at that coordinate, including its color-zero pixels. Only the final BG1 and BG2 images are composited per pixel by the PPU. Alternative BG1 layers, described below, are never composited with each other.

## BG1 maps

An authored BG1 has the same dimensions as BG2 in its area: a 4x4-screen area uses a 64x64 grid of Map16 tiles in `$7E4000-$7E5FFF`, and a 2x2-screen area uses one 32x32-Map16 flat map. The patcher resolves the selected BG1 layers into one optional placement per 8x8 cell, interns 2x2 cells as Map16 definitions using the area's allocated characters and palettes, and emits the same arrangement of flat maps as BG2. Absent placements use a transparent 8x8 word, and BG1-only Map16 definitions have zero collision properties. BG1 maps share the flat-map format and interner with BG2.

`BG1FlatMap16ScreenPointers` maps each screen to its BG1 map, parallel to the BG2 screen table; a zero pointer means the area has no authored BG1. When BG1 is active, every map covered by the area's BG2 dimensions is initialized, so transparent or filled regions never keep data from the previous area. BG1 placements take part in palette and character allocation with BG2 and dynamic tiles, so BG1 Map16 words refer to assets in the area's full-reload record.

## Alternatives

Two areas select between BG1 alternatives at runtime, so the assets of both alternatives coexist in the area's record:

- **Lost Woods** shows `Woods Fog` until the Master Sword is pulled (`$7EF300 & $40`), then `Woods Clear`, whose maps are in `LostWoodsClearBG1FlatMap16Pointers`.
- **Area `$80`** contains both Sacred Grove and Under the Bridge in one BG2 map. Entering through `$A0 == $0181` selects `Bridge Shadow`; the other entrance selects `Grove Fog` unless the Master Sword has been pulled, which leaves no authored BG1. The two maps are in `Area80BG1FlatMap16Pointers`. `Bridge Shadow` scrolls in lockstep with BG2, including vanilla's `$0100` vertical offset, and has no drift. See [splitting area $80](../../plans/splitting_area_80.md) for giving the two parts separate themes.

## Composition and camera

The area record's composition and camera fields (see [overworld asset loading](overworld_assets.md#area-record)) configure BG1 enablement, main and subscreen selection, and color math, with the value `$FF` keeping vanilla behavior. Screen IDs do not otherwise select presentation behavior in the engine.

Camera-follow values are dimensionless multipliers and drift values are signed pixels per frame, both in eighths. For each axis:

```text
background_delta = camera_delta * follow + drift
```

The runtime keeps background positions in eighth-pixel fixed point and derives the integer BG1 scroll and streamer coordinate from them. A full area load initializes the position from the camera's local coordinate within the area multiplied by `follow` and resets the fractional drift phase. During gameplay, unshaken camera movement and drift advance the position, and screen shake applies only to the presented scroll. With generated camera behavior, BG1 coordinates wrap at the area's dimensions, 64 tiles for a 2x2-screen area and 128 for a 4x4-screen area, so no area needs camera clamps; the Pyramid's authored fill covers its full camera range.

## Rain

Rain is not an ALTTPRetiling layer. It is a static 64x32 background of 8x8 tiles with its existing four-step offset animation, splashes, thunder flashes, sound, and state triggers, and it bypasses the BG1 streamer. Rain is selected only when the game state satisfies the rain predicate and the area has no authored BG1.

The patcher generates two sets of rain tiles with the same shapes, using pixel indices that match their allocated ALTTPRetiling palettes, and adds their palette and character dependencies to every area without authored BG1. `RainContexts` selects the set per screen.

| Context | Palette | Nontransparent colors and indices |
| --- | ---: | --- |
| Light World | `3` (`Light World 1`) | `1=[5,5,5]`, `10=[6,9,15]`, `11=[7,12,21]`, `12=[14,22,30]` |
| Dark World (sampled from Mire) | `8` (`Dark World 1`) | `11=[4,6,2]`, `13=[0,10,5]`, `14=[6,17,12]` |
