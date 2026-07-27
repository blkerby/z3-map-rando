# Engine modifications

The vanilla game's system for arranging palettes and graphics heavily constrain the ability the retheme areas or cleanly rearrange how they are connected, so we are replacing this with a more flexible system. The primary goal is to be able to inject the custom palettes and tilesets defined in the [retiling project](https://github.com/kjbranch/ALTTPRetiling), in such a way that area transitions do not show any visible artifacts of new palettes or tile graphics being loaded. A secondary goal is to reduce the lag at the start of area transitions.

As the engine work is complex, we break it down in detail. Components that are useful for testing the engine changes will be developed along the way, including the retiling catalog builder, the patcher, and CLI; on the other hand, the item placement logic and web backend and frontend are deferred until after the engine work is complete.

The planned engine work includes the following:

1. Replace compressed Map32 maps with generated, flat Map16 data (requiring expansion to a 2 MiB ROM).
2. Generate Map16 definitions from 8x8 tiles and make their four 8x8 collision properties authoritative and independent of graphics.
3. Reduce the overworld BG1 and BG2 tilemaps from 64x64 to 64x32, to free up VRAM for expanded tilesets.
4. Replace the fixed overworld palettes and tilesets with generated 4bpp tileset/palette bundles.
5. Separate an area's identity and gameplay metadata from its grid position.

It is expected that playtesting will likely uncover the need for additional engine work. Therefore, we want to prioritize the work in a way that allows playtesting as soon as possible. The planned milestone order is:

1. Set up the basic project structure, including the IPS build process, the Rust patcher library, and CLI.
2. Switch to using flat Map16 instead of Map32, while retaining their vanilla contents.
3. Expand Map16 definitions across 4 banks, still retaining their vanilla contents.
4. Switch collision queries to the four generated 8x8/quadrant properties of each Map16, independent of its graphics.
5. Convert BG1 to a 64x32 tilemap, including special treatments for rain and the Pyramid.
6. Convert BG2 to a 64x32 tilemap, streaming newly appearing tiles during all scrolling transitions.
7. Import the `Desert` theme from `ALTTPRetiling` and implement generated 4bpp tileset and palette bundles on the vanilla arrangement.
8. Implement area rearrangement, using a hand-specified arrangement and the `Desert Normalized` theme.
9. Relocate all gameplay objects and special cases, including flute destinations.
10. Generate the seed-accurate Mode 7 world map and markers, including while using the flute.

Each milestone change should end in a playable ROM. The last three milestones will all be based on the same hand-defined rearrangement. Randomized rearrangements and edge variants are deferred until after the engine work is complete.

Current status: Milestones 1 through 4 are complete.

Note: Everything below is mostly raw, unreviewed AI-generated notes. Especially for the not-yet-complete milestones, it should not be taken as a solid plan of what we will actually end up doing.

## Milestone 1: patching foundation

### Objective

Establish the minimum build and patching path needed to make playable engine changes: assemble source files into IPS patches, apply them safely to one known vanilla ROM, and write a 2 MiB test ROM.

### Implemented

- Add Asar as a submodule and scripts to build it and assemble each file in `patches/src` into a corresponding file in `patches/ips`. Assembly uses a dummy ROM with reads disabled, so building the IPS files does not require a vanilla ROM.
- Add `expand.asm`, which changes the LoROM size byte at `$00FFD7` from 1 MiB to 2 MiB. The Rust tool explicitly resizes the output buffer to 2 MiB before applying patches.
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

Banks `$20-$27` hold the 248 KiB flat maps. The 480-byte screen-to-map pointer table starts at `$27E000`, leaving banks `$28-$3F` for later milestones.

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

Banks `$28-$2B` contain the top-left, top-right, bottom-left, and bottom-right words respectively. The original coarse-property data remain unchanged until milestone 4 replaces them.

### Expanding the Map16 ID namespace

The WRAM maps and persistent-change records already store 16-bit Map16 IDs, so they do not need to become wider. Removing Map32 also removes its packed 12-bit Map16 encoding. The limiting assumption is instead the definition lookup: current IDs are shifted left three times and used as a 16-bit index into [`Map16Definitions`](../jpdasm/bank_0F.asm#L9). The table presently ends at tile `$E9D`, and each interleaved entry occupies eight bytes.

A simpler LoROM layout splits each definition into four parallel word tables:

```text
bank $28: top-left words
bank $29: top-right words
bank $2A: bottom-left words
bank $2B: bottom-right words

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

- `engine_check` imports the 3,742 vanilla definitions and writes their four quadrants to banks `$28-$2B` without changing IDs or words.
- `expand_map16.asm` migrates every direct definition load, including full builds, scrolling stripes, dynamic tile changes, entrances, terrain actions, and liftable objects.
- Runtime-selected quadrants use banked lookup routines; fixed-quadrant paths load their bank directly.
- The split-definition unit test checks quadrant ordering and byte layout.
- `engine_check` zeros the original table at `$0F8000-$0FF4EF`, so any remaining runtime dependency is visible during playtesting.

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

`engine_check` derives the vanilla records from the four 8x8 tiles in each Map16 and the vanilla `OverworldTileTypes` table. It also folds the graphics word's horizontal-flip bit into directional slope properties `$10-$1B`, matching the vanilla runtime calculation. Milestone 7 will generate the same records directly from properties authored in `ALTTPRetiling`.

The four dense 16 KiB quadrant tables occupy `$2C8000`, `$2CC000`, `$2D8000`, and `$2DC000`. They are indexed directly by Map16 ID and cover the full `$0000-$3FFF` namespace in 64 KiB.

### Implementation and validation

- `independent_tile_type.asm` replaces `GetOverworldTileType` in place. After vanilla locates the Map16 ID, bit 3 of the Y coordinate and bit 0 of the X-divided-by-8 coordinate select one of the four property tables.
- `ReadOverworldTileType` retains its vanilla WRAM-map lookup and tail-jumps to the same quadrant resolver. Its callers remain the guard and archer terrain probes and the hammer-splash check.
- Terrain actions use the shared quadrant resolver. Hammer sound selection and liftable objects read top-left directly; liftable coordinates are already rounded to a 16x16 boundary.
- The older graphics-derived implementations remain commented in `expand_map16.asm` for use when that patch is assembled without `independent_tile_type.asm`.
- The split-property unit test covers quadrant ordering, table padding, character masking, and horizontal slope flips.
- An audit found no remaining runtime reads of either legacy property table. After importing `OverworldTileTypes`, `engine_check` zeros it and the coarse `OverworldTileTypeTable`, making any missed dependency visible during playtesting.

Mixed-property Map16 tiles now intentionally use the property of the contacted quadrant. The legacy `$02`/`$5C` discrepancies are harmless to the two former coarse-property consumers: both values allow guard probes through and neither creates a hammer splash.

Milestone 4 is complete. The vanilla-layout checkpoint uses independent quadrant properties throughout while retaining the milestone 2 maps, milestone 3 definitions, vanilla graphics and palettes, and 64x64 PPU tilemaps.

## Milestone 5: static 64x32 BG1 tilemap

### Design

Convert only playable-overworld BG1 from 64x64 to 64x32 while BG2 remains unchanged. BG1 does not need runtime row or column streaming: its periodic overlays already fit, its bounded special-overworld views use no more than the 32-pixel margin, and rain and the Pyramid background can be adapted directly to the smaller tilemap.

The 64x32 BG1 layout keeps the screen blocks at VRAM word addresses `$1000` and `$1400`, freeing `$1800-$1FFF` (4 KiB). Keep the logical overlay map at `$7E4000-$7E5FFF`; only its PPU representation becomes smaller.

### BG1 usage inventory

[`ReloadSubscreenOverlay`](../jpdasm/bank_02.asm#L8500) selects and loads overlay pseudo-screens, and [`BuildBGOverlayFromMap16`](../jpdasm/bank_02.asm#L21110) expands them into BG1.

| Use | 64x32 treatment |
| --- | --- |
| `$95`, `$97`, `$9C`, `$9E` | Their rendered rows repeat every 256 pixels, so native tilemap wrapping is correct. |
| `$93/$88`, `$94/$80` | Their special-overworld camera ranges are at most 32 pixels, so one static window is sufficient. |
| Rain `$9F` | Replace the vanilla 64x64 pattern with a native 64x32 rain pattern and adjust [`OverworldOverlay_HandleRain`](../jpdasm/bank_02.asm#L7095) offsets to create a similar effect. Pixel-exact vanilla rain is not required. |
| Pyramid `$96` | Clamp BG1 vertical scroll when the viewport bottom reaches the bottom of the 256-pixel tilemap. With the current `$0600` base, the maximum top offset is `$0620`. Extend the solid tan region upward by the minimum whole 8x8 rows needed to cover transparency behind the bottom trees, while keeping it below the skyline visible at the top. |
| Inactive BG1 | Do not build or upload an overlay when BG1 is disabled. |

Full overlay reloads occur during normal overworld loading, mosaic transitions, mirror, whirlpool, flute/world-map return, special-overworld, and credits paths. No ordinary outdoor path dynamically changes the logical BG1 Map16 map.

### Required conversion

- Install `$12` in `BG1SC` and `$03` in `BG2SC` for the playable overworld while forced blank is active. Presentation modes may retain their own settings; the following overworld reload must restore this pair.
- Split the shared `BuildOverworldFromMap16`/`BuildBGOverlayFromMap16` machinery only where needed so BG1 builds 16 Map16 rows while BG2 continues to build 32.
- In the BG1 path, make [`CopyMap16ToBuffer`](../jpdasm/bank_02.asm#L21198) wrap at 16 Map16 rows, remove the vertical `$0800` screen-block offset, and retain the `$0400` horizontal-half offset.
- Make [`NMI_UpdateSubscreenOverlay`](../jpdasm/bank_00.asm#L2403) upload 4 KiB for BG1 and 8 KiB for the still-64x64 BG2 full load. Remove or bypass the latter BG1 half requested by mirror/whirlpool state machines.
- Add the 64x32 rain data and offset sequence.
- Clamp the Pyramid BG1 scroll at `$0620` and extend its tan tile rows as described above.
- Do not add BG1 residency state, boundary tracking, or edge-streaming code.

### Validation and exit criteria

1. Exercise every overlay screen/state, including long waits on fog and rain, the special overworlds, and movement across each overlay's full camera range.
2. Verify that the replacement rain remains convincing under ordinary movement and lightning flashes.
3. On the Pyramid, verify that the tan extension always covers the transparency behind the bottom trees, never appears in the top skyline, and that BG1 stops without wrapping.
4. Test mirror, whirlpool, flute/world-map return, interior return, save/reload, and ending credits.
5. Confirm through VRAM/DMA logging that BG1 never writes `$1800-$1FFF`, BG2 still uses its original 64x64 map, and no BG1 edge updates occur.

Milestone 5 is complete when every BG1 overlay/reload/scroll path works from a static 64x32 tilemap, the replacement rain and clamped Pyramid treatment pass visual testing, and the vanilla-layout ROM remains playable through milestones 2 through 4. Record the freed 4 KiB and measured full-load DMA/NMI budgets.

## Milestone 6: BG2 64x32 tilemap and transition streaming

### Design

Convert playable-overworld BG2 from 64x64 to a 64x32 circular window over the larger logical map. BG1 is already 64x32 but remains static. The original BG2 blocks are `$0000`, `$0400`, `$0800`, and `$0C00`; keep `$0000` and `$0400`, freeing `$0800-$0FFF` and bringing the combined saving to 8 KiB.

A 64x32 map is 512x256 pixels. The 224-pixel playfield leaves only two spare Map16 rows. [`OverworldCameraBoundaryCheck`](../jpdasm/bank_02.asm#L11415) already detects ordinary 16-pixel camera crossings, but vertical transitions currently preload the destination into the half of the 64x64 map that this milestone removes.

### Implementation strategy

1. Keep `$7E2000-$7E3FFF`, the milestone 2 flat maps, the milestone 3 banked definitions, the milestone 4 collision records, and their logical coordinates unchanged.
2. Treat BG2 as a ring of 16 Map16 rows while retaining its 32-Map16-column width.
3. Build only the camera-centered 16-row window on a full load.
4. On each 16-pixel vertical crossing, replace the two PPU rows that moved off screen with the next logical Map16 row.
5. Stream north/south destination rows throughout the transition instead of preloading a destination half. Progressively build east/west destination columns as well.

Graphics and palettes must be resident before the first tile that uses them is drawn, so measure asset DMA separately from tilemap construction.

### Required code changes

#### PPU configuration and full loads

- Change the playable-overworld register pair from `$12`/`$03` to `$12`/`$02`.
- Convert `BuildOverworldFromMap16` to the same 16-row output size established for BG1, removing the temporary mixed-height branch from milestone 5.
- In the BG2 path, [`CopyMap16ToBuffer`](../jpdasm/bank_02.asm#L21198) must use `($88 & $0F)` as the row, remove the vertical `$0800` offset, and retain the horizontal `$0400` offset.
- Both playable-overworld full-load builders now produce 4 KiB and can use the same descriptor count.

#### Runtime edge streaming

- [`BufferAndBuildMap16Stripes_Vertical`](../jpdasm/bank_02.asm#L19808) must wrap across 16 Map16 rows and stop adding `$0800`.
- [`BufferAndBuildMap16Stripes_Horizontal`](../jpdasm/bank_02.asm#L19647) must build only the 32 resident PPU rows and select the current logical 16-row window.
- Audit vertical ring index `$88` in initial-load, ordinary-scroll, automatic-walk, credits, special-overworld, whirlpool, mirror, and transition paths. Change 32-row wraps and sentinels to 16-row equivalents, but leave horizontal index `$86` at 32 columns.
- Reuse [`OverworldHandleMapScroll`](../jpdasm/bank_02.asm#L19345), the existing stripe-list format, and [`NMI_UpdateOWScroll`](../jpdasm/bank_00.asm#L2344).

#### Scrolling transitions

- Replace the north/south preload through [`CreateInitialNewScreenMapToScroll`](../jpdasm/bank_02.asm#L18890).
- Coordinate [`TriggerAndFinishMapLoadStripe_Vertical`](../jpdasm/bank_02.asm#L18742), [`OverworldTransitionScrollAndLoadMap`](../jpdasm/bank_02.asm#L19246), and [`OverworldScrollTransition`](../jpdasm/bank_02.asm#L11811) so each leading row is queued before its aliased PPU row becomes visible.
- Retain east/west transition geometry, but use the shortened horizontal stripe and progressive rendering.
- Stress-test the fastest transition step and screen shake.

#### Dynamic tile changes

- Update [`FindMap16VRAMAddress`](../jpdasm/bank_1B.asm#L14641) and the duplicate calculation in [`AlterMap16Hardcore`](../jpdasm/bank_1B.asm#L14544) to wrap into the 16-row BG2 ring while retaining the `$0400` right-half selection.
- `DrawMap16Anywhere` must always update logical WRAM, but issue an immediate VRAM stripe only when that row is resident.
- Test doors, bushes, rocks, bomb holes, weather-vane changes, and animated dungeon entrances near a ring boundary.

### Validation and exit criteria

1. Use distinct debug rows, then walk and dash across repeated 256-pixel vertical wraps.
2. Test north/south transitions between every small/large screen combination, then east/west transitions with the shortened column uploader.
3. Test dynamic tiles near wrap boundaries, flute travel, mirror, whirlpools, interior exits, pits, special overworlds, and ending credits.
4. Retest every milestone 5 BG1 overlay, especially rain and the Pyramid, to ensure BG2 streaming never modifies BG1.
5. Confirm that BG2 never writes `$0800-$0FFF` and BG1 never writes `$1800-$1FFF`.

Milestone 6 is complete when BG2 uses a 64x32 ring on every playable-overworld path, every scrolling transition renders destination stripes progressively without a tilemap staging pause, the milestone 5 BG1 checkpoint remains intact, and the vanilla-layout ROM remains playable through all earlier tests. Record the combined 8 KiB saving and final tilemap DMA/NMI budgets before assigning 4bpp assets.

## Milestone 7: 4bpp palettes and custom tilesets

### Objective

Import the `Desert` variants from `ALTTPRetiling` and replace the vanilla overworld palette and graphics loaders with generated 4bpp bundles. Keep the vanilla layout and use the completed flat Map16, independent collision, and 64x32 tilemap paths so this milestone changes only the asset source and residency system.

### Compiled retiling data and first asset target

- Implement the offline retiling-data builder described in the overall workflow. Parse the `ALTTPRetiling` JSON and referenced palette/tile definitions into versioned Rust source types, then emit one deterministic compact blob; the randomizer and patcher must not load the editable JSON tree directly.
- Use a verified vanilla reference ROM during this build to perform RGB-resolved 8x8 matching and replace every match with a vanilla tile reference, `$0-$7` to `$0-$F` color-index map, and flip flags. Reconstruct and pixel-validate all such references and emit the sanitization audit manifest alongside the blob.
- Validate area dimensions, component-screen positions, array sizes, palette/tile references, flips, priority, collision metadata, envelope/payload lengths, compiled indexes, stable IDs, the root `type_hash`, and source/content hashes.
- Treat the imported 8x8 placements and properties as the canonical source. Generate deduplicated Map16 graphics definitions, independent four-byte quadrant-property records, and flat logical maps from each 2x2 group without requiring artists to author Map16 or Map32 tiles.
- Preserve the vanilla physical layout and area footprints. Generate Light World/Dark World members consistently, but defer rearranged placement and edge metadata to milestone 8.
- Read the blob through the production reader, render every imported `Desert` area in software, and compare it to the checked-in `Desert.png` reference before patching a ROM.

### Asset compilation

The patcher converts compiled `ALTTPRetiling` records into SNES-specific seed data:

- Convert each 16-color palette to BGR555 CGRAM words and each tile to planar 4bpp character data.
- Resolve palette and tile IDs into seed-global CGRAM and VRAM slots. Emit final SNES tilemap words containing character index, BG palette number, priority, and horizontal/vertical flip bits.
- Deduplicate identical tiles, including legal flipped forms. Collision remains attached to the generated Map16 quadrant-property record and is never inferred from the assigned VRAM character.
- Reserve stable slots for engine-owned animated tiles and graphics used by dynamic overlays. Compile animated frames into the same bundle or reject conflicting authored slots.

Shared seed-generation and patcher validation must enforce the measured milestone 5 and 6 character, CGRAM, DMA, NMI, and ROM budgets, reporting the source asset and coordinate responsible for any failure.

### Palette allocation

Replace `OverworldPalettesScreenToSet`, `OverworldPaletteSet`, and `OverworldPalettesLoader` for custom areas with generated palette descriptors. Allocation must cover every source/destination band, BG1 overlay, animated color, and dynamic tile that can coexist during a scrolling transition.

The initial implementation uses seed-global palette and graphics slots: each asset has one runtime slot everywhere in the seed. Reject an assignment that exceeds capacity with useful per-asset diagnostics. Add transition-local remapping only if measurements prove that seed-global allocation cannot support the required assets.

Audit rain, fog, darkening, mirror/whirlpool transitions, damage flashes, palette cycling, and Light World/Dark World background colors. Custom bundles must reserve the locations those effects require or make the effects descriptor-aware. OBJ palettes remain outside BG allocation.

### Graphics loading and residency

The vanilla loader selects fixed sheet IDs and expands compressed 3bpp sheets into VRAM. The custom path instead uploads generated 4bpp blocks to the final character regions established by milestones 5 and 6.

The area/transition descriptor provides 24-bit source pointers, fixed VRAM destinations, and transfer lengths. The loader supports:

- A forced-blank full load when entering the overworld, using flute travel, returning from an interior, or taking a non-scrolling transition.
- Loading missing blocks needed by upcoming visible bands before a scrolling transition, without remapping an asset already on screen.
- A residency record so already-loaded blocks can be skipped and mirror, whirlpool, special-overworld, attract, credits, and world-map restoration paths cannot leave stale graphics.

Store flat 4bpp data unless ROM-size measurements require compression. Generate assertions for every reserved VRAM region and reject any bundle that exceeds the tilemap character-index range.

### Runtime patch points

The principal hooks are `LoadGraphicsAndScreenSize` and the overworld load/transition state machines in `bank_02.asm`, `InitializeTilesets` and the graphics/DMA routines in `bank_00.asm`, and `OverworldPalettesLoader` in `bank_0C.asm`. Unrelated dungeon, menu, attract-mode, and sprite loaders retain vanilla behavior.

Map16 generation and graphics allocation share one tile-word ABI. `Map16Definitions`, the edge stripe builders, dynamic `DrawMap16Anywhere` updates, and BG1 overlay construction consume the compiler's final character and palette slots; runtime code never interprets editor IDs.

### Validation and exit criteria

For every imported area and allowed vanilla-layout adjacency:

1. Compare a software rendering of the compiled SNES tile words against the editor image, including priority and flips.
2. Capture CGRAM and VRAM before, during, and after transitions in all four directions and verify every visible tile's character and palette.
3. Compare collision queries against the compiled quadrant-property records to prove that graphics allocation has no effect on gameplay properties.
4. Test animated water, grass, signs, houses, dungeon entrances, event overlays, rain/fog/parallax screens, mirror and whirlpool effects, flute travel, interior return, save-and-quit reload, and world-map exit.
5. Measure forced-blank load time, per-frame DMA volume, NMI time, and ROM size, and enforce those budgets during generation.

Milestone 7 is complete when the entire vanilla-layout `Desert` build uses generated direct-4bpp graphics and CGRAM data on the milestone 5 and 6 tilemaps, every character and palette has a stable seed-global slot, collision still comes only from independent quadrant properties, all restoration paths are descriptor-aware, and the ROM remains playable through the earlier checkpoint tests.


## Milestone 8: paired-world area topology and transitions

### Separate physical placement from area identity

The vanilla engine overloads the overworld screen ID as a grid coordinate, area-size selector, graphics/palette selector, sprite and secret list index, event identity, music/environment selector, and persistence key. Rearrangement needs at least two explicit concepts:

- **Slot ID:** the physical location used for world coordinates, camera bounds, and lookup of north/east/south/west neighbors.
- **Area ID:** the stable identity of the authored location used for its entrances, sprites, events, secrets, music, and persistent state.
- **Area-pair ID:** the Light World area and its corresponding Dark World area, which are placed and themed as one randomization unit.

Keep `$8A` as a compatibility value only after deciding which meaning minimizes patching; do not let new code depend implicitly on both meanings. Prefer a new generated descriptor lookup that starts from the current slot and exposes the area ID and all runtime metadata. Screen-specific vanilla branches can then be converted deliberately to either physical placement or logical identity.

The physical topology is fixed to the vanilla 8x8 overworld grid, and every area keeps its vanilla dimensions and footprint. The layout generator places an entire area pair into corresponding Light World/Dark World slots and assigns the same theme to both members. It does not independently shuffle the worlds, resize areas, create holes outside the vanilla footprint rules, or introduce non-rectangular areas.

The current `ALTTPRetiling` schema describes only background tiles: 67 areas have a 2x2 screen grid and 17 have a 4x4 grid, with each component screen containing 32x32 8x8 placements. It does not yet describe typed edges, gameplay objects, or map art. The first retiled-asset target in milestone 7 therefore uses the existing `Desert` variant on the vanilla layout. Edge-aware randomization waits for a versioned edge schema; its final representation remains intentionally undecided.

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

The milestone 8 seed generator reads the production compiled-retiling blob and writes the fully resolved paired-world topology to the JSON seed artifact. For each logical area it records stable area/pair IDs, Light World and Dark World physical slots, vanilla footprint, selected theme, selected edge variants, directed adjacency, counterpart mapping, and travel/map metadata known at this stage. It also records the RNG seed, settings, schema version, compiled-blob identities, and required engine ABI.

The JSON uses logical IDs and coordinates rather than ROM addresses or packed runtime structures. Generation performs topology, edge, capacity, and reachability validation before publishing the JSON and its seed-specific retiling subset. The shared Rust patcher—native in the CLI and WebAssembly in the browser—later consumes those artifacts and deterministically builds the slot descriptors, transition tables, graphics/palette assignments, and other binary structures in the IPS-reserved seed-data regions; it does not rerun layout randomization. For web generation, the service returns only the selected variants and their transitive retiling dependencies, not the full catalog.

### Validation and exit criteria

1. First generate descriptors for the vanilla placement and prove that slot lookup, area identity, paired-world lookup, internal large-area movement, and all four transition directions reproduce vanilla behavior.
2. Verify that every Light World/Dark World pair has matching placement, footprint, and selected theme, and that mirror/portal counterpart lookup reaches the corresponding relative coordinate.
3. Once edge metadata exists, generate shuffled layouts using only the vanilla 8x8 grid and vanilla-sized footprints. Reject overlaps, out-of-grid placements, unpaired worlds, and incompatible or missing edge records.
4. Exercise internal and external transitions for every small/large footprint combination and every connection type, including milestone 5 BG1 overlays, milestone 6 BG2 streaming, and milestone 7 graphics/palette residency.

Milestone 8 is complete when the engine no longer derives topology from vanilla screen-ID arithmetic, vanilla placement works entirely through generated descriptors, and—after the edge schema becomes available—paired areas can be shuffled with every directed adjacency validated before ROM generation.

## Milestone 9: gameplay-data relocation and validation

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

Flute destinations need generated physical landing slots and coordinates. Fixed-slot mode can retain landmark numbering while changing destinations; geographic mode must sort the selected landmarks by final placement. Milestone 10 renders the resulting numbers and icons on the map. Every landing must be checked for valid ground, adequate sprite clearance, and correct graphics/palette loading before control returns to the player.

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

Milestone 9 is complete when all relocated gameplay data follow logical areas rather than vanilla physical screen IDs, vanilla-relative objects have been validated on every selected tiling variant, persistence survives movement and save/reload, flute destinations match the JSON seed, the screen-ID audit has no unexplained cases, and representative JSON seeds can be patched and completed without engine or topology failures. Once item randomization is implemented, completion also requires a deterministic validated compiled logic blob, equivalence between its reachability evaluator and the source-JSON reference evaluator, and item placements in the JSON reproducing exactly in the patched ROM.

## Milestone 10: generated Mode 7 world map

Generate the Mode 7 tilemap, graphics, edge art, and Link, dungeon-reward, portal, objective, and flute markers from the final topology and milestone 9 gameplay data. Keep these assets separate from the playable-overworld tileset: bank 0A and bank 18 use a distinct presentation path and VRAM mode. Derive them from the authoritative JSON rather than creating a second representation of the world layout.

### Validation and exit criteria

1. Verify that the Light World and Dark World maps match the generated topology and selected edge art.
2. Verify every Link, reward, portal, objective, and flute marker against its generated world coordinate.
3. Test map zooming and panning, flute selection and landing, menu exit, and restoration of the playable-overworld graphics and palettes.
4. Run a final completion playthrough to catch map, flute, or restoration regressions.

Milestone 10 is complete when both Mode 7 maps accurately represent the seed, all markers and flute choices agree with the JSON data, returning from the map restores the playable overworld correctly, and the final ROM passes completion testing.

## Deferred design decisions

The initial direction is fixed above: playable vanilla-asset checkpoints for flat Map16, independent collision, static 64x32 BG1, and streaming 64x32 BG2; then seed-global graphics/palette assignments, paired Light/Dark areas on the vanilla grid, gameplay-data relocation, and finally Mode 7 map generation. The following details remain intentionally deferred:

- The schema for edge connection types, material variants, and map art that will eventually be added to `ALTTPRetiling`.
- Whether gameplay-object metadata belongs in `ALTTPRetiling` or in a project-owned companion layer. In either case it needs stable IDs and area-relative coordinates.
- Whether particular tiling variants may eventually override the initial vanilla-relative object positions.
- Whether asset measurements force a transition-local graphics/palette fallback, and what behavior to use when a seed-global assignment fails. The initial implementation rejects the seed rather than silently changing transition style.
- Whether logic strats remain monotonic predicates over acquired items and enabled tech or later model consumable resources, counters, dungeon keys, world state, glitches with parameters, or other state transitions. These require explicit schema semantics rather than being inferred by the blob compiler.
