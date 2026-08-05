# Engine modifications

The vanilla game's system for arranging palettes and graphics heavily constrain the ability the retheme areas or cleanly rearrange how they are connected, so we are replacing this with a more flexible system. The primary goal is to be able to inject the custom palettes and tilesets defined in the [retiling project](https://github.com/kjbranch/ALTTPRetiling), in such a way that area transitions do not show any visible artifacts of new palettes or tile graphics being loaded. A secondary goal is to reduce the lag at the start of area transitions.

As the engine work is complex, we break it down in detail. Components that are useful for testing the engine changes will be developed along the way, including the retiling catalog builder, the patcher, and CLI; on the other hand, the item placement logic and web backend and frontend are deferred until after the engine work is complete.

The planned engine work includes the following:

1. Replace compressed Map32 maps with generated, flat Map16 data (requiring expansion to a 2 MiB ROM).
2. Generate Map16 definitions from 8x8 tiles and make their four 8x8 collision properties authoritative and independent of graphics.
3. Reduce the overworld BG tilemaps, to free up VRAM for expanded tilesets.
4. Replace the fixed overworld palettes and tilesets with generated 4bpp tileset/palette bundles.
5. Separate an area's identity and gameplay metadata from its grid position.

It is expected that playtesting will likely uncover the need for additional engine work. Therefore, we want to prioritize the work in a way that allows playtesting as soon as possible. The planned milestone order is:

1. Set up the basic project structure, including the IPS build process, the Rust patcher library, and CLI.
2. Switch to using flat Map16 instead of Map32, while retaining their vanilla contents.
3. Expand Map16 definitions across 4 banks, still retaining their vanilla contents.
4. Switch collision queries to the four generated 8x8/quadrant properties of each Map16, independent of its graphics.
5. Convert BG3 in both overworld and dungeons from 64x64 to 32x64 by removing its unused right half.
6. Convert BG3 in both overworld and dungeons from 32x64 to a streamed 32x32 tilemap.
7. Convert BG2 to a 64x32 tilemap, streaming newly visible 8x8 edges during gameplay.
8. Convert BG1 to a streamed 64x32 tilemap, then add the separate 64x32 rain treatment.
9. Complete the graphics and palette work through three playable checkpoints:
   - **9A:** Install the final overworld VRAM layout while retaining vanilla assets.
   - **9B:** Load a generated 4bpp bundle that reproduces the vanilla appearance.
   - **9C:** Compile and install the `Desert` world from `ALTTPRetiling` on the vanilla arrangement.
10. Implement area rearrangement, using a hand-specified arrangement and the `Desert Normalized` theme.
11. Relocate all gameplay objects and special cases, including flute destinations.
12. Generate the seed-accurate Mode 7 world map and markers, including while using the flute.
13. Reload destination BG1 logical overlays during ordinary area transitions.
14. Make BG1 parallax deterministic across bulk loads and scrolling transitions by separating logical BG1 coordinates from their physical tilemap placement.

Each milestone and lettered checkpoint should end in a playable ROM. The last three milestones will all be based on the same hand-defined rearrangement. Randomized rearrangements and edge variants are deferred until after the engine work is complete.

Current status: Milestones 1 through 9A are complete.

Note: Everything below is mostly raw, unreviewed AI-generated notes. Especially for the not-yet-complete milestones, it should not be taken as a solid plan of what we will actually end up doing.

## Milestone 1: patching foundation

### Objective

Establish the minimum build and patching path needed to make playable engine changes: assemble source files into IPS patches, apply them safely to one known vanilla ROM, and write a 2 MiB test ROM.

### Implemented

- Add Asar as a submodule and scripts to build it and assemble each file in `patches/src` into a corresponding file in `patches/ips`. Assembly uses a dummy ROM with reads disabled, so building the IPS files does not require a vanilla ROM.
- Add `expand.asm`, which changes the LoROM size byte at `$80FFD7` from 1 MiB to 2 MiB. The Rust tool explicitly resizes the output buffer to 2 MiB before applying patches.
- Add a Cargo workspace containing the `patcher` library and the small `engine_check` CLI.
- In `patcher`, support LoROM address conversion, contextual writes, overlap detection, IPS records including RLE records, and bounds checking when patches are applied.
- In `engine_check`, accept input and output ROM paths, require the known 1 MiB unheadered vanilla ROM by SHA-256 digest, resize a copy, apply the IPS patches through `Patcher`, and write the result. Later milestones extend this tool with their test data and patches.

### Validation

- The patcher tests cover address conversion, ordinary and RLE IPS records, adjacent writes, and conflicting writes.
- `engine_check` rejects inputs with the wrong size or digest.
- The generated ROM is kept as a local test artifact; the source ROM is not required to build the IPS files.

Milestone 1 deliberately does not provide a general-purpose randomizer CLI, manifests or versioned ABIs, checksum repair, seed JSON handling, WebAssembly or TypeScript integration, browser tests, or a release-artifact pipeline. Those should be added only when a later milestone needs them.

## Milestone 2: flat Map16 storage

### Design

On the expanded ROM produced by milestone 1, replace the compressed Map32 maps with flat, 16-bit Map16 data, removing both decompression and Map32-to-Map16 expansion from an overworld load. Retain the vanilla layout, palettes, tilesets, Map16 definitions, collision behavior, and 64x64 PPU tilemaps.

The existing 124 distinct Map32 submaps are screens `$00-$61` and `$80-$9F`. Each expands to a 32x32 array of 16-bit Map16 IDs, or 2 KiB. Storing all of those submaps flat therefore requires exactly 248 KiB. This is small enough that the editable, uncompressed source representation is not a ROM-space concern after expansion.

The current logical WRAM maps should remain unchanged: `$7E2000-$7E3FFF` is the 64x64 main Map16 map and `$7E4000-$7E5FFF` is the corresponding overlay map. Keeping this row-major format preserves collision, scrolling, entrances, persistent tile changes, and the runtime stripe source. This storage change is independent of the PPU tilemap dimensions.

### Vanilla data build and playable checkpoint

- Generate the flat maps offline from the vanilla Map32 data and definitions. Custom palettes, graphics, and `ALTTPRetiling` data are not inputs to this milestone.
- Preserve vanilla Map16 IDs so the old and new logical maps can be compared directly while using the current single-bank definition table.
- Verify the generated flat maps against the original Map32 expansion for every screen ID, including overlays and recovered persistent changes.
- Produce a vanilla-layout ROM using the normal vanilla graphics and palette loaders. This is the first playable checkpoint and must pass ordinary gameplay, transitions, dynamic Map16 changes, mirror/whirlpool reloads, interior returns, save/load, and ending-path smoke tests before definition expansion begins.

### Strided loads

A flat 32x32 submap is contiguous in ROM, but its rows are 64 bytes long while rows in the 64x64 WRAM map are 128 bytes apart. Load each row with one `MVN` and skip 64 destination bytes before the next row.

Large areas load four submaps into the four WRAM quadrants. Small areas load only their top-left quadrant; the other three quadrants remain stale and are not valid map data for that area.

[`BuildOverworldFromMap16`](../jpdasm/bank_02.asm#L21128) must still expand resident Map16 tiles into PPU tilemap words. Screen-specific rocks, bomb doors, overlays, and recovered persistent changes also remain between loading the base image and building the PPU map.

### Expanded-ROM data placement

- Add ASM through `patches/src` and generated data through contextual `Patcher` writes, without modifying the disassembly itself. Record occupied and free ranges in `patches/rom_map`.
- Use assembler assertions for code placement and `Patcher` checks for overlapping or out-of-bounds writes.

Banks `$B8-$BF` hold the 248 KiB flat maps. The 480-byte screen-to-map pointer table starts at `$BFE000`.

### Map32 removal: code and data to replace

The main conversion is concentrated in bank 02:

- Replace [`Overworld_DecompressAndDrawAllQuadrants`](../jpdasm/bank_02.asm#L19974) and [`Overworld_DecompressAndDrawOneQuadrant`](../jpdasm/bank_02.asm#L20033) with a flat-map loader. The call sites in [`DrawOverworldQuadrantsAndOverlays`](../jpdasm/bank_02.asm#L18615) and [`SomeTilemapChange`](../jpdasm/bank_02.asm#L18836) must use the new loader.
- Replace [`LoadSubOverlayMap32`](../jpdasm/bank_02.asm#L20474) with a flat overlay load into `$7E4000`. `LoadOverworldOverlay` and the later BG1 builder can otherwise retain their roles.
- Replace the Map32 pointer lookup with a generated 24-bit flat-map pointer table. Preserve the existing aliases for nonexistent or shared screen IDs.
- Remove the map-specific high/low reconstruction routines `BlockMoveMap32Chunks_High` and `BlockMoveMap32Chunks_Low`, the `$7F4400` map scratch use, and [`ParseMap32Definition`](../jpdasm/bank_02.asm#L20260).
- Do not remove `Decompress_bank02`: [`DecompressEnemyDamageSubclasses`](../jpdasm/bank_02.asm#L21396) also uses the generic decompressor for unrelated data.

The obsolete ROM data are the compressed `OverworldMap32_Screen*_High/Low` streams in [`bank_0B.asm`](../jpdasm/bank_0B.asm#L5) and the beginning of [`bank_0C.asm`](../jpdasm/bank_0C.asm#L5), plus `Tile32_TopLeft`/`Tile32_TopRight` in [`bank_03.asm`](../jpdasm/bank_03.asm#L5) and `Tile32_BottomLeft`/`Tile32_BottomRight` at the beginning of [`bank_04.asm`](../jpdasm/bank_04.asm#L5). Bank 04 also contains live overworld code after those tables, and banks 0B/0C contain non-map code after their map data, so the whole files cannot be discarded.

The new load must preserve the existing post-load order. In particular, [`Overworld_HandleOverlaysAndBombDoors`](../jpdasm/bank_02.asm#L18681), hard-coded screen fixes, event overlays, and [`RecoverTilesFromMirrorBonk`](../jpdasm/bank_02.asm#L21358) modify the logical Map16 buffer before it is converted to PPU words. A direct copy performed after those operations would incorrectly erase their changes.

### Implementation and validation sequence

1. Generate flat vanilla maps and verify their main and overlay WRAM buffers against the original loader for every screen ID after overlays and recovered changes.
2. Load those maps while retaining legacy Map16 IDs and the current single-bank definition table.
3. Run the playable vanilla-layout checkpoint through ordinary gameplay, transitions, dynamic Map16 changes, mirror/whirlpool reloads, interior returns, save/load, and ending paths.

Milestone 2 is complete when no runtime path depends on Map32 data, the generated logical maps match the original loader, and the vanilla-layout ROM remains playable through the full smoke-test set. Measure ROM size and load time here so later milestones can attribute their own changes independently.

## Milestone 3: expanded Map16 definitions

### Design

Expand Map16 definitions across four banks while retaining the milestone 2 flat maps, vanilla definition contents, collision behavior, palettes, tilesets, and 64x64 PPU tilemaps. Store one quadrant word per bank so every table uses the same `ID * 2` index. Do not make the original coarse per-Map16 property table part of the final representation; the milestone 4 quadrant-property work replaces it.

Banks `$A1-$A4` contain the top-left, top-right, bottom-left, and bottom-right words respectively. The original coarse-property data remain unchanged until milestone 4 replaces them.

### Expanding the Map16 ID namespace

The WRAM maps and persistent-change records already store 16-bit Map16 IDs, so they do not need to become wider. Removing Map32 also removes its packed 12-bit Map16 encoding. The limiting assumption is instead the definition lookup: current IDs are shifted left three times and used as a 16-bit index into [`Map16Definitions`](../jpdasm/bank_0F.asm#L9). The table presently ends at tile `$E9D`, and each interleaved entry occupies eight bytes.

A simple banked layout splits each definition into four parallel word tables:

```text
bank $A1: top-left words
bank $A2: top-right words
bank $A3: bottom-left words
bank $A4: bottom-right words

address in each bank = $8000 + (id * 2)
```

Each 32 KiB table holds 16,384 words, so this layout supports Map16 IDs `$0000-$3FFF`: four times the current 4,096-definition address space.

### Definition-access patch summary

The current disassembly has 30 direct definition loads plus two fixed data-bank setup references. Most are simple local edits: shift the Map16 ID once instead of three times and replace each interleaved offset with the corresponding quadrant-bank label. Keep the four rendering loads inline because full builds and scrolling stripes perform a definition lookup for every Map16 tile.

| Accesses | Change |
| --- | --- |
| [`CreateMap16Stripes_Horizontal`](../jpdasm/bank_02.asm#L19743) and [`CreateMap16Stripes_Vertical`](../jpdasm/bank_02.asm#L19920) | Simple local edits: use `ID * 2` and four fixed long-addressed quadrant loads. |
| [`DrawMap16Anywhere`](../jpdasm/bank_1B.asm#L14478) and [`AlterMap16Hardcore`](../jpdasm/bank_1B.asm#L14544) | Simple local edits using the same four-load replacement. |
| [`UseOverworldEntrance`](../jpdasm/bank_1B.asm#L12048) | Simple local edits: its four reads already name fixed quadrant offsets, so replace them with the corresponding bank labels. |
| [`PickHammerSFX`](../jpdasm/bank_1B.asm#L12647) | Simple local edit to the fixed top-left bank. |
| [`CheckForSpecialOverworldTrigger`](../jpdasm/bank_04.asm#L4640), [`GetMap16Tile`](../jpdasm/bank_04.asm#L4703), and [`CheckForReturnTrigger`](../jpdasm/bank_04.asm#L4775) | Simple shared-index edit in `GetMap16Tile`, followed by fixed top-left loads in both callers. |
| [`BuildBGOverlayFromMap16`](../jpdasm/bank_02.asm#L21110), [`BuildOverworldFromMap16`](../jpdasm/bank_02.asm#L21128), and [`CopyOneMap16Segment`](../jpdasm/bank_02.asm#L21292) | Local but slightly larger: the copy loop currently relies on the data bank for 16-bit definition loads. Change those to four long-addressed quadrant loads and remove the assumption that all definitions share one bank. |
| [`GetOverworldTileType`](../jpdasm/bank_00.asm#L1450) | More involved: it encodes a runtime-selected quadrant into the interleaved byte offset. Preserve that selection with a four-way bank choice until milestone 4 replaces the graphics-derived property lookup. |
| The terrain query in [`OverworldTileAction_Bush`](../jpdasm/bank_1B.asm#L12529) and [`Overworld_GetLiftableTileType`](../jpdasm/bank_1B.asm#L12842) | More involved for the same reason: each selects a quadrant at runtime and therefore needs a four-way bank choice until milestone 4. |

Preserve the register widths, X/Y values, and data bank expected by each caller. Benchmark high-ID maps as well as legacy maps.

There are two different collision tables that must not be confused:

- [`OverworldTileTypes`](../jpdasm/bank_0F.asm#L4123) is indexed by the underlying 8x8 character number after masking a PPU tile word with `$01FF`. It need not grow merely because more Map16 combinations are available.
- [`OverworldTileTypeTable`](../jpdasm/bank_1B.asm#L18453) is indexed directly by Map16 ID in [`ReadOverworldTileType`](../jpdasm/bank_05.asm#L22505). It currently has one byte for each definition. Retaining the original representation would require expanding it and making it bank-aware; the 8x8-authored collision plan below instead replaces this lookup with quadrant-level properties.

Expanding Map16 definitions does not by itself expand the set of 8x8 graphical characters available to BG1/BG2. New Map16 tiles may freely make new combinations of the loaded characters, palettes, priority, and flips. Supporting more character graphics at once would be a separate VRAM, graphics-loading, and possibly tile-property project.

Hard-coded tile IDs such as `$0DBE`, `$0D9E`, and `$0DA0` in banks 02/04/07/1B can remain valid if the legacy definitions retain their numbers. The map converter should reject undefined IDs and reserve any chosen sentinel values explicitly.

### Implementation and validation

- `engine_check` imports the 3,742 vanilla definitions and writes their four quadrants to banks `$A1-$A4` without changing IDs or words.
- `expand_map16.asm` migrates every direct definition load, including full builds, scrolling stripes, dynamic tile changes, entrances, terrain actions, and liftable objects.
- Runtime-selected quadrants use banked lookup routines; fixed-quadrant paths load their bank directly.
- The split-definition unit test checks quadrant ordering and byte layout.
- `engine_check` zeros the original table at `$8F8000-$8FF4EF`, so any remaining runtime dependency is visible during playtesting.

Milestone 3 is complete. The banked representation supports IDs `$0000-$3FFF` while retaining vanilla collision, palettes, tilesets, and 64x64 PPU tilemaps.

## Milestone 4: 8x8-authored overworld collision properties

This milestone changes collision while retaining the milestone 2 flat maps, milestone 3 banked Map16 definitions, vanilla graphics and palettes, and 64x64 PPU tilemaps.

### Compatibility findings

The editor assigns a collision/property byte to each 8x8 tile and allows artists to compose screens freely from those tiles. The original game also has [`OverworldTileTypeTable`](../jpdasm/bank_1B.asm#L18453), which assigns one coarse property to an entire Map16 tile. That coarse byte cannot always be reconstructed unambiguously from the four constituent properties:

- Of the 3,742 existing Map16 definitions, 3,731 Map16 properties (99.7%) equal at least one of their four 8x8 properties.
- Selecting the top-left property reproduces only 2,916 entries (77.9%).
- Selecting the majority property reproduces only 2,710 entries (72.4%).
- Eleven definitions use coarse property `$5C` even though none of their constituent 8x8 tiles has that property.
- Four identical ordered property quartets have both `$02` and `$5C` Map16-level results, proving that the original coarse value is not strictly a function of the four constituent values.

For the only two current consumers, `$02` and `$5C` are behaviorally equivalent: both produce no collision through `GeneralizedProjectileTileInteraction`, and neither is treated as water by the hammer-splash check. The exceptions therefore do not justify retaining lossy whole-Map16 collision for the current engine behavior.

### Representation

Each Map16 ID has an eight-byte graphics definition and a separate four-byte quadrant-property record:

```text
graphics:    top-left word, top-right word, bottom-left word, bottom-right word
properties:  top-left byte, top-right byte, bottom-left byte, bottom-right byte
```

`engine_check` derives the vanilla records from the four 8x8 tiles in each Map16 and the vanilla `OverworldTileTypes` table. It also folds the graphics word's horizontal-flip bit into directional slope properties `$10-$1B`, matching the vanilla runtime calculation. Milestone 9C will generate the same records directly from properties authored in `ALTTPRetiling`.

The four dense 16 KiB quadrant tables occupy `$A58000`, `$A5C000`, `$A68000`, and `$A6C000`. They are indexed directly by Map16 ID and cover the full `$0000-$3FFF` namespace in 64 KiB.

### Implementation and validation

- `independent_tile_type.asm` replaces `GetOverworldTileType` in place. After vanilla locates the Map16 ID, bit 3 of the Y coordinate and bit 0 of the X-divided-by-8 coordinate select one of the four property tables.
- `ReadOverworldTileType` retains its vanilla WRAM-map lookup and tail-jumps to the same quadrant resolver. Its callers remain the guard and archer terrain probes and the hammer-splash check.
- Terrain actions use the shared quadrant resolver. Hammer sound selection and liftable objects read top-left directly; liftable coordinates are already rounded to a 16x16 boundary.
- The older graphics-derived implementations remain commented in `expand_map16.asm` for use when that patch is assembled without `independent_tile_type.asm`.
- The split-property unit test covers quadrant ordering, table padding, character masking, and horizontal slope flips.
- An audit found no remaining runtime reads of either legacy property table. After importing `OverworldTileTypes`, `engine_check` zeros it and the coarse `OverworldTileTypeTable`, making any missed dependency visible during playtesting.

Mixed-property Map16 tiles now intentionally use the property of the contacted quadrant. The legacy `$02`/`$5C` discrepancies are harmless to the two former coarse-property consumers: both values allow guard probes through and neither creates a hammer splash.

Milestone 4 is complete. The vanilla-layout checkpoint uses independent quadrant properties throughout while retaining the milestone 2 maps, milestone 3 definitions, vanilla graphics and palettes, and 64x64 PPU tilemaps.

## Milestone 5: 32x64 BG3 tilemap

### Design

Unlike the other engine milestones, milestones 5 and 6 affect both overworld and dungeon gameplay. BG3 is shared by their HUD and pause-menu paths, so both modes adopt the reduced layout. There is no plan to use the extra VRAM space that this frees in dungeons, but it could be used later if the project expands to include dungeon changes.

The playable game configures BG3 as a 64x64 tilemap at `$6000`, but uses only its left screen-block column:

- `$6000` contains the HUD and other ordinary BG3 text.
- `$6800` contains the pause menu, which is revealed by vertical scrolling.
- No content upload targets the right-hand blocks at `$6400` or `$6C00`; only the blanket BG3 clear currently writes them.

The milestone checkpoint set `BG3SC` from `$63` to `$62` and moved the pause-menu block from `$6800` to `$6400`. The milestone 6 implementation subsequently replaced `reduce_bg3.asm` with the final 32x32 version.

### Validation

The generated IPS contains only those two byte changes. Automated patch and workspace checks pass, and gameplay testing confirmed that the reduced BG3 layout works.

Milestone 5 is complete. BG3 uses only `$6000-$67FF`, freeing 4 KiB without changing its visible behavior.

## Milestone 6: streamed 32x32 BG3 tilemap

### Implementation

BG3 uses one 32x32 screen block at `$6000`, selected by `BG3SC=$60`. This applies to both overworld and dungeons, and the shared BG3 clear covers only that screen block.

The fixed HUD fits, but the item menu can no longer remain below it in a second block. The tilemap acts as a 32-row vertical ring during the existing 8-pixel menu animation:

- Build the item menu in its existing `$1000-$17FF` buffer without uploading it over the HUD.
- While opening, upload each newly visible menu row from that buffer.
- Before closing, rebuild the existing HUD buffer at `$7EC700` without uploading it immediately; then restore each newly visible HUD or blank row.
- Keep full-screen uploads for item and bottle-menu changes after the menu is completely visible.

NMI tile-update mode `$06`, unused by vanilla, performs the 64-byte row DMA. The opening row is `(BG3VOFS / 8) & $1F`. Because the PPU's effective BG vertical coordinate is one pixel ahead, the bottom scanline comes from the closing row 28 rows after the new top row.

The ending credits already stream attribution rows as BG3 scrolls. Their first row is delayed until row 0 has moved offscreen, and all subsequent destinations wrap within `$6000-$63FF`. The name-entry stripe list ends before its unused vanilla right-half records would write into `$6400-$67FF`. Other BG3 users already remain within one screen block.

### Validation

Automated patch and workspace checks pass. Gameplay testing confirmed that the item menu streams without a visible seam and that closing restores transparent rows and the HUD correctly. The name-entry and ending-credits paths are confined to the reduced tilemap.

Milestone 6 is complete. BG3 uses `$6000-$63FF` in both overworld and dungeons, and `$6400-$6FFF` is free.

## Milestone 7: BG2 64x32 tilemap and streaming

### Design

Convert playable-overworld BG2 from 64x64 to 64x32 while leaving the logical
Map16 map at `$7E2000-$7E3FFF`. Keep VRAM blocks `$0000` and `$0400` and free
`$0800-$0FFF`.

### Implementation strategy

The detailed design and implementation checkpoints are in
[`bg_streamer.md`](bg_streamer.md). In summary:

1. Disable every vanilla playable-overworld BG2 tilemap producer while
   preserving logical Map16 updates. First validate that BG2 remains blank.
2. For a full load, render the 33x29 tile window beginning at the tile
   containing the viewport's top-left pixel into the existing `$1100/$18`
   arbitrary DMA list.
3. During gameplay, compare the previous and new finalized BG2 scroll values,
   including shake. Crossing an 8-pixel boundary prepares at most one entering
   33-tile row and one entering 29-tile column.
4. Append the prepared transfers to the existing `$1100/$18` arbitrary DMA
   list. The existing `$12` main-loop/NMI barrier protects the buffer, so no
   new queue, ready bits, margin, or resident-window state is needed.
5. Keep immediate Map16 changes on `$1000/$14`, with their VRAM destination
   calculation changed for the 64x32 tilemap.

Rows split at 32-tile VRAM screen-block boundaries and columns split at the
32-row wrap. Camera discontinuities use the bulk-list path rather than
gameplay edge streaming. Shared NMI handlers remain available to non-BG2
users; only the vanilla overworld BG2 producers are removed.

### Validation and exit criteria

1. Complete the five incremental checkpoints in `bg_streamer.md`.
2. Test ordinary movement, diagonal movement, shake, and scrolling area
   transitions in all four directions.
3. Test dynamic tiles near wrap boundaries, flute travel, mirror, whirlpools,
   interior exits, pits, special overworlds, and ending credits.
4. Confirm that BG2 never writes `$0800-$0FFF` and that frames without an
   8-pixel boundary crossing perform no BG2 tilemap upload.

Milestone 7 is complete. BG2 uses a 64x32 tilemap on every
playable-overworld path, including bulk loads, scrolling transitions,
immediate Map16 changes, mirror warps, portals, whirlpools, and world-map
returns.

## Milestone 8: streamed 64x32 BG1 tilemap

### Design

Convert playable-overworld BG1 to a 64x32 tilemap while leaving its logical
64x64 8x8-tile overlay map at `$7E4000-$7E5FFF`. Keep VRAM blocks `$1000` and
`$1400` and free `$1800-$1FFF`.

Adapt the proven milestone 7 design to BG1: a 33x29 forced-blank load followed
by at most one entering 8x8 row and column per gameplay frame. Generalize only
the renderer code that BG1 and BG2 actually share. Both layers must be able to
append transfers in the same frame without adding a second synchronization
mechanism.

Rain remains separate from the streamer. Use a modified rain tilemap and
offset sequence that fits in 64x32.

The detailed design and incremental test gates are in
[`bg1_streamer.md`](bg1_streamer.md).

### Validation and exit criteria

1. Exercise every non-rain overlay across its full camera range, including
   the Pyramid, fog, special overworlds, and screen shake.
2. Test simultaneous BG1 and BG2 edge updates, plus mirror, whirlpool,
   flute/world-map return, interior return, and save/reload.
3. Confirm through VRAM/DMA logging that BG1 never writes `$1800-$1FFF` and
   BG2 never writes `$0800-$0FFF`.
4. Add and test the separate 64x32 rain treatment.

Milestone 8 is complete. BG1 uses a 64x32 tilemap on every
playable-overworld path, including rain and the mirror margins exposed by its
HDMA wave. Milestone 14 separately corrects the Castle/Pyramid parallax
relationship across ordinary scrolling transitions.

## Milestone 9A: final VRAM layout with vanilla assets

### Objective

Install the final overworld VRAM layout while retaining the vanilla graphics,
palettes, maps, and collision. Rebase the existing graphics loader so this
checkpoint isolates memory-layout and mode-switching changes from the new
bundle format and custom assets.

### Overworld VRAM layout

Install the final overworld-specific layout described in [`vram.md`](vram.md):

- Put 960 shared BG1/BG2 4bpp characters at `$0000-$3BFF`.
- Move the 32x32 BG3 tilemap to `$3C00-$3FFF`.
- Leave OBJ graphics at `$4000-$5FFF` and BG3 graphics at `$7000-$7FFF`.
- Move the BG2 and BG1 tilemaps to `$6000-$67FF` and `$6800-$6FFF`.

This uses `BG12NBA=$00`, `BG1SC=$69`, `BG2SC=$61`, and `BG3SC=$3C`. Character IDs `$3C0-$3FF` are unavailable because they overlap the BG3 tilemap.

Milestone 9A does not rearrange dungeon VRAM. Its BG1, BG2, graphics, and OBJ regions retain their vanilla locations; BG3 remains at its reduced milestone 6 size at `$6000`. Dungeon scrolling does not stream rows or columns: it keeps a complete room resident and uploads whole 32x32 destination quadrants before inter-room scrolling.

Keep the shared code small:

- Leave the dungeon/default register setup and tilemap clear intact; add overworld-specific setup and clear paths.
- Reuse the generic tilemap DMA with overworld destinations.
- Reuse `$0219` as the active BG3 tilemap destination so HUD and pause-menu updates work at `$3C00` in the overworld and `$6000` in dungeons.
- Rebase only overworld stripe, ring, and dynamic-tile address calculations. Dungeon quadrant builders remain unchanged.
- Restore the appropriate layout and graphics under forced blank whenever gameplay or a presentation mode changes between them.

Milestone 9A is complete. The game remains visually vanilla, all overworld
scenes use the final layout, dungeons retain their milestone 8 layout, and
transitions restore the correct registers, tilemaps, and graphics.

## Milestone 9B: generated 4bpp bundle with vanilla appearance

Have `engine_check` convert the existing vanilla overworld graphics and
palettes into one flat 4bpp bundle. Load it through generated descriptors into
the milestone 9A layout while retaining the existing maps, Map16 definitions,
collision records, and visual appearance.

The implementation sequence begins with the module `$08` forced-blank path
described in [`asset_loading.md`](asset_loading.md).

Submilestones 9B.1-9B.4 are implemented, with gameplay validation pending.
Generated static assets now cover forced-blank entry, scrolling, mirror and
whirlpool effects, mosaic recovery, flute travel, world-map restoration,
credits scenes, the Triforce room, and the four area-dependent OBJ slots.
Submilestone 9B.5 covers animation and is deferred while work begins on 9C.

Define the runtime ABI here: bundle descriptors contain 24-bit source
pointers and fixed-size VRAM or palette-row destinations. Use the same
descriptor path for forced-blank overworld entry, flute travel, interior
return, non-scrolling travel, and restoration from presentation modes. Store
the data flat unless ROM-size measurement demonstrates that compression is
needed.

Replace the old overworld graphics and palette loaders at their common entry
points while leaving dungeon loading unchanged. The principal hooks are
`LoadGraphicsAndScreenSize`, the overworld load state machines in
`bank_02.asm`, `InitializeTilesets` and the graphics/DMA routines in
`bank_00.asm`, and `OverworldPalettesLoader` in `bank_0C.asm`.

Milestone 9B is complete when the ROM remains visually vanilla and playable,
all overworld load and restoration paths use the generated 4bpp bundle and
descriptor ABI, and no runtime path requires the old overworld graphics or
palette loaders.

## Milestone 9C: compiled Desert world

### First playable checkpoint

Add a separate `theme_check` binary which reads the editable `Desert.json`
area data and palette definitions directly from the checked-out
`ALTTPRetiling` submodule. Like `engine_check`, it verifies a vanilla ROM,
applies the current patches, and writes a test ROM. It expands this checkpoint
ROM to 4 MiB so it can retain the milestone 9B fixed-row descriptor ABI without
new engine ASM.

Compile each 2x2 group of editor screens into one vanilla-layout flat Map16
screen. Preserve the vanilla definitions at their existing IDs and append
deduplicated Desert definitions and independent quadrant properties. Assign
palette halves and character slots deterministically across screens: assets
which coexist must have distinct slots, while non-coexisting assets may reuse
them. Emit six complete palette rows and 32 complete character rows per screen
through the existing generated bundle loader.

Use Desert maps, palettes, graphics, priority, flips, and collision for every
authored screen. Retain vanilla sprites and unauthored special/credits scenes.
The compiled catalog and vanilla sanitization mapping described below remain
later 9C work.

Known gaps in this checkpoint:

- Animated tiles use a static authored frame while 9B.5 is deferred.
- Dynamic Map16 changes and vanilla BG1/event overlays can show incorrect
  graphics because their expected vanilla characters are not resident.
- Reused slots can show artifacts while a scrolling transition replaces the
  source screen's fixed rows with the destination rows.
- Mode 7 map art, rain/fog effects, dungeon openings, and other dynamic
  presentation paths are not yet theme-aware.

This checkpoint is successful when the ordinary Light and Dark Worlds render
from Desert data on the vanilla arrangement, authored collision is playable,
the output remains usable across ordinary travel and dungeon entry/exit, and
generation rejects any palette, character, Map16, metadata, payload, or ROM
region overflow.

### Compiled retiling data and first asset target

- Implement the offline retiling-data builder described in the overall workflow. Parse the `ALTTPRetiling` JSON and referenced palette/tile definitions into versioned Rust source types, then emit one deterministic compact blob; the randomizer and patcher must not load the editable JSON tree directly.
- Use a verified vanilla reference ROM during this build to perform RGB-resolved 8x8 matching and replace every match with a vanilla tile reference, `$0-$7` to `$0-$F` color-index map, and flip flags. Reconstruct and pixel-validate all such references and emit the sanitization audit manifest alongside the blob.
- Validate area dimensions, component-screen positions, array sizes, palette/tile references, flips, priority, collision metadata, envelope/payload lengths, compiled indexes, stable IDs, the root `type_hash`, and source/content hashes.
- Treat the imported 8x8 placements and properties as the canonical source. Generate deduplicated Map16 graphics definitions, independent four-byte quadrant-property records, and flat logical maps from each 2x2 group without requiring artists to author Map16 or Map32 tiles.
- Preserve the vanilla physical layout and area footprints. Generate Light World/Dark World members consistently, but defer rearranged placement and edge metadata to milestone 10.
- Read the blob through the production reader, render every imported `Desert` area in software, and compare it to the checked-in `Desert.png` reference before patching a ROM.

### Asset compilation

The patcher converts compiled `ALTTPRetiling` records into SNES-specific seed data:

- Convert each 16-color palette to BGR555 CGRAM words and each tile to planar 4bpp character data.
- Resolve palette and tile IDs into seed-global CGRAM and VRAM slots. Emit final SNES tilemap words containing character index, BG palette number, priority, and horizontal/vertical flip bits.
- Deduplicate identical tiles, including legal flipped forms. Collision remains attached to the generated Map16 quadrant-property record and is never inferred from the assigned VRAM character.
- Reserve stable slots for engine-owned animated tiles and graphics used by dynamic overlays. Compile animated frames into the same bundle or reject conflicting authored slots.

Shared seed-generation and patcher validation must enforce the measured milestone 7 and 8 character, CGRAM, DMA, NMI, and ROM budgets, reporting the source asset and coordinate responsible for any failure.

Before designing transition-local loading, measure the complete compiled
`Desert` world against the 960-character and available BG-palette limits. If
it fits, assign one seed-global slot to each asset and load one global bundle;
do not add residency tracking.

### Palette allocation

Replace `OverworldPalettesScreenToSet`, `OverworldPaletteSet`, and `OverworldPalettesLoader` for custom areas with generated palette descriptors. Allocation must cover every source/destination band, BG1 overlay, animated color, and dynamic tile that can coexist during a scrolling transition.

Use seed-global palette and graphics slots when the measured bundle fits:
each asset has one runtime slot everywhere in the seed. Reject an assignment
that exceeds capacity with useful per-asset diagnostics. Add transition-local
loading or remapping only if measurements prove that a global bundle cannot
support the required assets.

Audit rain, fog, darkening, mirror/whirlpool transitions, damage flashes, palette cycling, and Light World/Dark World background colors. Custom bundles must reserve the locations those effects require or make the effects descriptor-aware. OBJ palettes remain outside BG allocation.

### Graphics loading

Reuse the milestone 9B descriptor loader to upload the generated 4bpp bundle
to `$0000-$3BFF`. With a global bundle, forced-blank entry and restoration
paths load it as one stable layout and scrolling transitions require no
graphics loading or residency state.

Only if the global bundle does not fit, extend the descriptors and loader to
support:

- A forced-blank full load when entering the overworld, using flute travel, returning from an interior, or taking a non-scrolling transition.
- Loading missing blocks needed by upcoming visible bands before a scrolling transition, without remapping an asset already on screen.
- A residency record so already-loaded blocks can be skipped and mirror, whirlpool, special-overworld, attract, credits, and world-map restoration paths cannot leave stale graphics.

Generate assertions for every reserved VRAM region and reject any bundle that
exceeds the tilemap character-index range.

### Runtime patch points

Milestone 9C reuses the milestone 9A layout switching and milestone 9B bundle
ABI. Its runtime changes should be limited to consuming the generated Desert
maps, Map16 definitions, collision records, tile words, graphics, and
palettes. Dungeon tilemaps, graphics, and quadrant loaders retain their
existing layout. Shared HUD and menu code uses the active BG3 destination.

Map16 generation and graphics allocation share one tile-word ABI. `Map16Definitions`, the edge stripe builders, dynamic `DrawMap16Anywhere` updates, and BG1 overlay construction consume the compiler's final character and palette slots; runtime code never interprets editor IDs.

### Validation and exit criteria

For every imported area and allowed vanilla-layout adjacency:

1. Compare a software rendering of the compiled SNES tile words against the editor image, including priority and flips.
2. Capture CGRAM and VRAM before, during, and after transitions in all four directions and verify every visible tile's character and palette.
3. Compare collision queries against the compiled quadrant-property records to prove that graphics allocation has no effect on gameplay properties.
4. Test animated water, grass, signs, houses, dungeon entrances, event overlays, rain/fog/parallax screens, mirror and whirlpool effects, flute travel, interior return, save-and-quit reload, and world-map exit.
5. Repeatedly enter and leave dungeons, open the item and bottle menus in both layouts, and verify that each transition restores the correct registers, tilemaps, and graphics.
6. Measure forced-blank load time, per-frame DMA volume, NMI time, and ROM size, and enforce those budgets during generation.

Milestone 9C is complete when the entire vanilla-layout `Desert` build uses generated direct-4bpp graphics and CGRAM data in the 960-character overworld layout, dungeons retain their full resident tilemaps, collision still comes only from independent quadrant properties, all restoration paths are descriptor-aware, and the ROM remains playable through the earlier checkpoint tests. If the measured global bundle fits, completion also requires that no transition-local graphics loading or residency system was added.


## Milestone 10: paired-world area topology and transitions

### Separate physical placement from area identity

The vanilla engine overloads the overworld screen ID as a grid coordinate, area-size selector, graphics/palette selector, sprite and secret list index, event identity, music/environment selector, and persistence key. Rearrangement needs at least two explicit concepts:

- **Slot ID:** the physical location used for world coordinates, camera bounds, and lookup of north/east/south/west neighbors.
- **Area ID:** the stable identity of the authored location used for its entrances, sprites, events, secrets, music, and persistent state.
- **Area-pair ID:** the Light World area and its corresponding Dark World area, which are placed and themed as one randomization unit.

Keep `$8A` as a compatibility value only after deciding which meaning minimizes patching; do not let new code depend implicitly on both meanings. Prefer a new generated descriptor lookup that starts from the current slot and exposes the area ID and all runtime metadata. Screen-specific vanilla branches can then be converted deliberately to either physical placement or logical identity.

The physical topology is fixed to the vanilla 8x8 overworld grid, and every area keeps its vanilla dimensions and footprint. The layout generator places an entire area pair into corresponding Light World/Dark World slots and assigns the same theme to both members. It does not independently shuffle the worlds, resize areas, create holes outside the vanilla footprint rules, or introduce non-rectangular areas.

The current `ALTTPRetiling` schema describes only background tiles: 67 areas have a 2x2 screen grid and 17 have a 4x4 grid, with each component screen containing 32x32 8x8 placements. It does not yet describe typed edges, gameplay objects, or map art. The first retiled-asset target in milestone 9C therefore uses the existing `Desert` variant on the vanilla layout. Edge-aware randomization waits for a versioned edge schema; its final representation remains intentionally undecided.

### Generated topology and transition metadata

Replace the arithmetic assumptions in `ActualOverworldScreenID`, `OverworldScreenIDChange`, `OverworldScreenTilemapChangeByScreen`, the fixed transition-position tables, and `OverworldScreenSizeFlag` with generated per-slot metadata. Each slot should provide:

- Area ID, area-pair ID, world, vanilla footprint membership, logical map pointer, BG1 overlay mode, tileset/palette bundle, music, ambient/fixed-color settings, and camera bounds.
- Four neighbor entries containing destination slot, destination entry edge, coordinate offset along that edge, and transition kind. A missing edge must not transition even if the player reaches the outer coordinate.
- Enough information to distinguish movement inside a multi-slot 4x4 area from movement to a different area, because only the latter may require palette, graphics, sprite, music, and area-identity changes.
- Return/spawn metadata for entrances, holes, flute landings, mirror destinations, whirlpools, save-and-quit, and special overworlds.
- The corresponding slot in the paired world. Mirror and portal travel uses this generated pairing while retaining the area's authored relative destination coordinate.

Normalized matching edge variants should preserve the player's coordinate along the border. If a future connection maps different offsets or widths, represent that transform explicitly rather than embedding another table of screen-specific arithmetic in ASM. The generator must verify that both sides agree on opening type, traversable pixels, collision, ledges, water continuity, and any one-way restriction.

Part Five calls for large, small, split, upper-only/lower-only, and specialized river connections, plus multiple material variants. Add these as authored edge records rather than deriving them solely from pixels. Pixel/collision comparison should then validate the record. Explicit metadata is needed for layout search, useful diagnostics, map-art selection, and connections whose semantics cannot be inferred safely from appearance.

### JSON layout output

The milestone 10 seed generator reads the production compiled-retiling blob and writes the fully resolved paired-world topology to the JSON seed artifact. For each logical area it records stable area/pair IDs, Light World and Dark World physical slots, vanilla footprint, selected theme, selected edge variants, directed adjacency, counterpart mapping, and travel/map metadata known at this stage. It also records the RNG seed, settings, schema version, compiled-blob identities, and required engine ABI.

The JSON uses logical IDs and coordinates rather than ROM addresses or packed runtime structures. Generation performs topology, edge, capacity, and reachability validation before publishing the JSON and its seed-specific retiling subset. The shared Rust patcher—native in the CLI and WebAssembly in the browser—later consumes those artifacts and deterministically builds the slot descriptors, transition tables, graphics/palette assignments, and other binary structures in the IPS-reserved seed-data regions; it does not rerun layout randomization. For web generation, the service returns only the selected variants and their transitive retiling dependencies, not the full catalog.

### Validation and exit criteria

1. First generate descriptors for the vanilla placement and prove that slot lookup, area identity, paired-world lookup, internal large-area movement, and all four transition directions reproduce vanilla behavior.
2. Verify that every Light World/Dark World pair has matching placement, footprint, and selected theme, and that mirror/portal counterpart lookup reaches the corresponding relative coordinate.
3. Once edge metadata exists, generate shuffled layouts using only the vanilla 8x8 grid and vanilla-sized footprints. Reject overlaps, out-of-grid placements, unpaired worlds, and incompatible or missing edge records.
4. Exercise internal and external transitions for every small/large footprint combination and every connection type, including milestone 7 BG2 streaming, milestone 8 BG1 overlays, and milestone 9C graphics and palette loading.

Milestone 10 is complete when the engine no longer derives topology from vanilla screen-ID arithmetic, vanilla placement works entirely through generated descriptors, and—after the edge schema becomes available—paired areas can be shuffled with every directed adjacency validated before ROM generation.

## Milestone 11: gameplay-data relocation and validation

### Relocate gameplay data and hard-coded behavior

Changing only the background map is insufficient. The following must become generated per-area or per-slot data, or be proven intentionally fixed:

- Overworld sprites and overlords, including phase/state variants and the large-area proximity loader in `bank_09.asm`.
- Entrances, doors, holes, special-overworld triggers and returns, item locations, hidden items, dig spots, bonk prizes, signs, lift/hammer/bomb targets, and persistent Map16 changes in banks 04, 07, and 1B.
- Event overlays and animated dungeon entrances. Prefer authored overlay/delta data keyed by area ID over screen-number dispatch such as `ApplyOverworldOverlay`; all changed tiles still need the 64x32 residency check.
- Link spawn/return coordinates, follower restrictions, flute landing coordinates, whirlpools, portals, mirror mappings, and Light World/Dark World correspondence.
- Screen-specific music, ambient sound, weather, fixed color, parallax/overlay behavior, and any special camera or transition rule.
- Enemy/event persistence bits. Keys must remain stable for an area across placement, while any state that truly belongs to a physical slot must be identified separately.

The initial object-placement rule is deliberately conservative: entrances, sprites, secrets, portals, and other placed gameplay objects retain their vanilla coordinates relative to their logical area, and every tiling/theme variant for that area uses the same coordinates. Extract the vanilla objects into area-relative metadata, translate them to the paired area's generated world position in Rust, and emit ordinary runtime coordinates. Large-area members, objects on boundaries, paired-world destinations, and multi-tile overlays must be validated after relocation. This keeps general coordinate math out of the ASM patch and avoids making theme variants duplicate gameplay data.

This placement rule is provisional rather than a permanent schema restriction. Store stable object IDs and area-relative coordinates in a form that can later accept per-variant offsets, enable/disable flags, or replacement records if a retiling makes a vanilla position visually or mechanically unsuitable. Until that extension is adopted, the compiler must reject a variant when a retained object position no longer has valid terrain, clearance, or interaction tiles.

A systematic audit is required for literal overworld IDs and screen-indexed tables across all banks, not only the primary overworld banks listed above. Each occurrence should be classified as physical-slot, logical-area, area-pair, world, presentation-only, or obsolete. This audit is a required deliverable because missed special cases are likely to create seed-specific softlocks rather than obvious startup failures.

### Fast travel

Flute destinations need generated physical landing slots and coordinates. Fixed-slot mode can retain landmark numbering while changing destinations; geographic mode must sort the selected landmarks by final placement. Milestone 12 renders the resulting numbers and icons on the map. Every landing must be checked for valid ground, adequate sprite clearance, and correct graphics/palette loading before control returns to the player.

### Compiled logic-data build and reachability

Create the separate logic project with a versioned JSON schema, then implement the offline compiler described in the overall workflow. The compiler emits one deterministic blob; seed generation must use its production reader rather than parsing the logic project's JSON directly.

The initial schema and compiler must support:

- Stable area and dungeon-room scopes that can be joined to the retiling/game-data IDs without using physical ROM addresses.
- Stable directed node and strat IDs, node kinds including item locations, and multiple alternative strats for one directed traversal.
- Requirement expressions over acquired items and enabled tech, with nested Boolean alternatives where needed.
- Configured start nodes, goals, and item-location bindings used to validate each logic-data release.
- Compact dictionaries, adjacency indexes, and interned requirement encodings with deterministic ordering and strict bounds/reference checks.

At seed generation time, instantiate traversal state from the selected starting state and enabled-tech configuration. Traverse only directed strats whose requirements are satisfied, collect the reachable item-location nodes, and use that result while placing or revealing items. After an assumed or acquired progression item is added, reevaluate reachability to construct the next logical step/sphere. Cache or incrementally update graph results only after the straightforward evaluator is correct and covered by equivalence tests.

Test the compiled evaluator against a reference evaluator over the source JSON using representative item and tech combinations. Also test directionality, alternative strats, cycles, unreachable components, unknown requirements, and deterministic sphere construction. The logic build manifest records the source JSON schema version, root `type_hash`, pinned `bincode-next` configuration, source/content hashes, stable-ID dictionaries, counts, and validation diagnostics.

### JSON gameplay output

Extend the JSON seed schema with resolved gameplay data keyed by stable logical IDs: relocated entrances, sprites, secrets, portals, event/persistence identities, flute destinations, and other area-owned objects. When item randomization is implemented, the same artifact also records every item placement, relevant item-randomizer settings, objective requirements, and other choices needed to reproduce the seed without rerunning RNG.

The JSON remains the authoritative description of the randomized seed. The patcher translates its logical, area-relative data into physical coordinates and ROM tables defined by the engine manifest. Derived ROM offsets, compressed byte streams, checksums, and other patcher implementation details do not replace the portable JSON representation.

### Generator validation and play testing

Before emitting a seed, verify:

1. Every occupied edge has a mutually compatible neighbor and every unoccupied edge is impassable.
2. Every required entrance, exit, return point, world-state counterpart, and fast-travel destination resolves to a valid slot and coordinate.
3. Relocated sprites and objects are in bounds, do not collide with forbidden terrain, and have all required graphics and palettes.
4. Persistent state has a unique stable key, event deltas reference valid generated Map16 IDs, and no hard-coded vanilla screen behavior is still reachable unintentionally.
5. The seed's area graph is traversable under the intended item logic, including one-way ledges, water connections, world swaps, and special transitions.

Runtime testing should include every directed edge variant, all four approaches to multi-slot areas, transitions while dashing or carrying/followed, dynamic tiles adjacent to borders, save/reload from every area class, Light World/Dark World swaps, and a full completion playthrough on randomized layouts.

Milestone 11 is complete when all relocated gameplay data follow logical areas rather than vanilla physical screen IDs, vanilla-relative objects have been validated on every selected tiling variant, persistence survives movement and save/reload, flute destinations match the JSON seed, the screen-ID audit has no unexplained cases, and representative JSON seeds can be patched and completed without engine or topology failures. Once item randomization is implemented, completion also requires a deterministic validated compiled logic blob, equivalence between its reachability evaluator and the source-JSON reference evaluator, and item placements in the JSON reproducing exactly in the patched ROM.

## Milestone 12: generated Mode 7 world map

Generate the Mode 7 tilemap, graphics, edge art, and Link, dungeon-reward, portal, objective, and flute markers from the final topology and milestone 11 gameplay data. Keep these assets separate from the playable-overworld tileset: bank 0A and bank 18 use a distinct presentation path and VRAM mode. Derive them from the authoritative JSON rather than creating a second representation of the world layout.

### Validation and exit criteria

1. Verify that the Light World and Dark World maps match the generated topology and selected edge art.
2. Verify every Link, reward, portal, objective, and flute marker against its generated world coordinate.
3. Test map zooming and panning, flute selection and landing, menu exit, and restoration of the playable-overworld graphics and palettes.
4. Run a final completion playthrough to catch map, flute, or restoration regressions.

Milestone 12 is complete when both Mode 7 maps accurately represent the seed, all markers and flute choices agree with the JSON data, returning from the map restores the playable overworld correctly, and the final ROM passes completion testing.

## Milestone 13: BG1 logical overlays during area transitions

Ordinary overworld transitions currently retain the logical BG1 overlay in
`$7E4000`; leaving and re-entering an area changes its BG1 enable and scroll
state but does not call `LoadOverworldOverlay`. This only works when adjacent
areas share the already-loaded overlay.

Use the generated destination-area descriptor to load its logical BG1 overlay
when crossing into a different area. Sequence the reload so tiles still
visible from the source area remain correct while destination tiles stream
into view. Handle transitions between two different overlays, between an
overlay and no overlay, and between areas sharing one overlay.

Milestone 13 is complete when all four transition directions replace the
logical overlay without stale rows, visible source-area changes, or requiring
a full overworld reload.

## Milestone 14: deterministic BG1 parallax and tilemap offsets

### Coordinate model

Separate the BG1 coordinate used to select logical overlay tiles from the
coordinate used to address the physical 64x32 tilemap ring:

- **BG2 scroll** is the ordinary overworld camera position in `$E2/$E8`.
- **Logical BG1 scroll** selects pixels from the 64x64 logical overlay in
  `$7E4000-$7E5FFF`.
- **Physical BG1 scroll** in `$E0/$E6` selects pixels from the 64x32 PPU
  tilemap ring.
- **BG1 X/Y tile offsets** map physical ring tiles to logical overlay tiles.
  They are arbitrary integer tile counts, reduced modulo 64.

If physical tile `(x, y)` contains logical tile
`(x + offset_x, y + offset_y)`, the normal-movement relationship is:

```text
physical BG1 scroll = logical BG1 scroll - 8 * tile offset
```

Ordinary areas retain their existing canonical 1:1 relationship:

```text
logical BG1 = BG2
```

Castle and Pyramid use one simpler parallax rule on both axes:

```text
logical BG1 = floor(BG2 / 2)
```

This deliberately replaces the vanilla centered horizontal formula and
piecewise vertical clamp. The old horizontal formula differs from exact
half-speed by four pixels modulo one tile, so it cannot be represented
exactly by an integer tile offset. Shift the Castle/Pyramid overlay data by
whole tiles offline to restore the closest useful framing, and validate the
remaining approximation visually rather than adding a sub-tile runtime
exception.

Select the canonical relationship once when area state changes. Renderer hot
paths consume coordinates and offsets; they must not branch on Castle or
Pyramid IDs.

### Normal movement and bulk loads

During normal movement, derive the logical scroll directly from BG2 using the
active 1:1 or half-speed policy, then derive the physical scroll from the
logical scroll and current tile offset. The old fractional accumulators are
unnecessary. Bulk loading, world-map return, mirror, whirlpool, interior
return, and save/reload must all initialize the same canonical relationship
and render the same logical viewport.

Keep physical tile coordinates as the renderer API. They already match VRAM
ring addressing and its 32-tile split boundaries. Add the active BG1 tile
offset only when selecting the logical Map16 source. BG2 supplies zero source
offsets to the same renderer.

### Scrolling transitions

At the start of an ordinary transition:

1. Determine the final BG2 position and the destination area's canonical
   logical BG1 position.
2. Calculate the final physical BG1 position produced by lockstep movement
   and choose destination X/Y tile offsets so it maps to the canonical logical
   viewport.
3. Advance physical BG1 in lockstep with BG2 for the entire transition.
4. Use those destination offsets for entering BG1 rows and columns during the
   transition.

Transition displacements and the new half-speed formula are tile-aligned, so
the destination offsets are always integral. Validate that difference before
dividing by eight; do not round an unexpected sub-tile remainder. A full
scrolling transition replaces the visible BG1 window with tiles selected
using the destination mapping; no end-of-transition bulk reload or scroll
correction is needed.

The offset state and transition calculation apply to every area. The first
playable checkpoint enables the half-speed policy only for Castle/Pyramid and
uses the ordinary 1:1 policy elsewhere, but entering or leaving those areas
may produce nonzero physical-to-logical offsets in either destination.

### Renderer and DMA constraints

Rows and columns continue to split only at physical 32-tile VRAM boundaries,
so each requires at most two DMA entries. A physical DMA segment may cross
the 64-tile logical-source wrap. In that case, fill its contiguous WRAM
payload from two logical source runs, wrapping the source while retaining one
DMA header.

Convert coordinates once per source run, not once per tile. Apply the same
offset mapping to bulk windows, gameplay edges, mirror margins, and any
immediate BG1 tile update that addresses the physical ring.

### Playable checkpoints and validation

1. Add generic X/Y offset state and source-run wrapping while all area
   policies remain 1:1 and initial offsets remain zero. Confirm no rendering
   change.
2. Enable the half-speed policy and adjusted overlay data for Castle/Pyramid.
   Compare bulk entry, all four scrolling entries, normal movement, and
   subsequent exits at the same BG2 positions.
3. Exercise arbitrary X/Y offsets for ordinary areas, including logical
   source wraps on both axes and transitions into and out of parallax areas.
4. Verify through DMA logging that a row or column still emits no more than
   two DMA entries and that every tile visible at transition completion was
   rendered with the destination mapping.
5. Retest mirror, whirlpool, flute/world-map return, interior return,
   save/reload, screen shake, and dynamic overlay changes.

Milestone 14 is complete when BG1's visible logical viewport is determined
only by BG2 position and the active area policy after every entry path,
scrolling transitions leave a fully rendered destination viewport, arbitrary
tile offsets work for every area without per-area renderer branches, and the
Castle/Pyramid framing is acceptably close to vanilla with the adjusted
overlay data.

## Deferred design decisions

The initial direction is fixed above: playable vanilla-asset checkpoints for flat Map16, independent collision, streamed 64x32 BG1 and BG2, then seed-global graphics/palette assignments, paired Light/Dark areas on the vanilla grid, gameplay-data relocation, and finally Mode 7 map generation. The following details remain intentionally deferred:

- The schema for edge connection types, material variants, and map art that will eventually be added to `ALTTPRetiling`.
- Whether gameplay-object metadata belongs in `ALTTPRetiling` or in a project-owned companion layer. In either case it needs stable IDs and area-relative coordinates.
- Whether particular tiling variants may eventually override the initial vanilla-relative object positions.
- Whether asset measurements force a transition-local graphics/palette fallback, and what behavior to use when a seed-global assignment fails. The initial implementation rejects the seed rather than silently changing transition style.
- Whether logic strats remain monotonic predicates over acquired items and enabled tech or later model consumable resources, counters, dungeon keys, world state, glitches with parameters, or other state transitions. These require explicit schema semantics rather than being inferred by the blob compiler.
