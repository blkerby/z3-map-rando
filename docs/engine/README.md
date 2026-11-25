# Engine modifications

The vanilla game's arrangement of palettes and graphics heavily constrains rethemed and rearranged areas, so the randomizer replaces much of the overworld engine with data generated at patching time. This page summarizes each change and links to its detailed documentation or patch. [ASM patches](patches.md) describes how patches are built and applied, and the [ROM map](../../patches/rom_map/) records the space they occupy. References on the original game are in [`docs/vanilla/`](../vanilla/).

## ROM and execution

The ROM is expanded and runs in FastROM mode ([`rom_size.asm`](../../patches/src/rom_size.asm), [`fastrom_base.asm`](../../patches/src/fastrom_base.asm), [`fastrom_extra.asm`](../../patches/src/fastrom_extra.asm)). [`nmi_optimize.asm`](../../patches/src/nmi_optimize.asm) reduces the CPU work done during NMI, including a rewritten arbitrary-DMA handler.

## Maps and collision

- **Flat Map16 maps.** Compressed Map32 maps are replaced by flat 32x32 Map16 screen and overlay maps, copied into the unchanged 64x64 WRAM Map16 maps at `$7E2000` and `$7E4000` without decompression ([`overworld_map_data.asm`](../../patches/src/overworld_map_data.asm)). Screen-specific overlays, bomb doors, and recovered persistent changes still modify the WRAM map after loading and before it is drawn.
- **Banked Map16 definitions.** Each of a definition's four 8x8 quadrant words lives in its own bank, indexed by `ID * 2`, which supports Map16 IDs `$0000-$3FFF` ([`overworld_map16_graphics.asm`](../../patches/src/overworld_map16_graphics.asm)). The patcher generates the definitions from ALTTPRetiling's 8x8 placements; artists never author Map16 or Map32 tiles.
- **Quadrant collision properties.** Each Map16 quadrant has its own property byte in four parallel tables, so collision and terrain actions use the property of the contacted quadrant, independently of graphics ([`overworld_map16_properties.asm`](../../patches/src/overworld_map16_properties.asm)). Properties come from ALTTPRetiling, and the vanilla coarse per-Map16 table is unused.
- **Dynamic tiles.** Terrain, secret, door, and grave replacements resolve generated graphics from generated Map16 tables, as described in [dynamic tiles](dynamic_tiles.md).

## Tilemaps and VRAM

- **BG3.** A 32x32 tilemap, shared by the overworld and dungeons, replaces vanilla's 64x64; the item menu streams rows through it as a vertical ring ([`bg3_tilemap.asm`](../../patches/src/bg3_tilemap.asm)).
- **BG1 and BG2.** Overworld BG1 and BG2 use 64x32 tilemaps that stream visible rows and columns from the logical Map16 maps ([`overworld_bg_tilemaps.asm`](../../patches/src/overworld_bg_tilemaps.asm)). [`mirror_bg1_hdma.asm`](../../patches/src/mirror_bg1_hdma.asm) keeps BG1's own horizontal scroll through the mirror's wave effect.
- **VRAM layout.** The freed tilemap space expands the shared BG1/BG2 character set from 512 to 960 characters in the overworld, while dungeons keep their vanilla layout apart from BG3 ([VRAM layout](vram.md), [`overworld_vram.asm`](../../patches/src/overworld_vram.asm)).

## Generated graphics and presentation

- **Assets.** Each area's background characters, palettes, and area-dependent sprite characters load directly from ROM through generated records, for every loading and transition path ([overworld asset loading](overworld_assets.md)). Generated animation tracks replace the vanilla animated tiles ([`overworld_animations.asm`](../../patches/src/overworld_animations.asm)), and [`overworld_bg_color.asm`](../../patches/src/overworld_bg_color.asm) selects generated backdrop colors.
- **Backgrounds.** BG1 layers, composition, and camera follow and drift come from ALTTPRetiling ([overworld backgrounds](overworld_backgrounds.md)).
- **Cutscenes.** Dungeon-entrance cutscenes are generated scripts run by a small interpreter ([cutscenes](cutscenes.md)).
- **World map.** The Mode 7 world maps are generated, and the map shows uncollected dungeon prizes according to their placement ([`overworld_world_map.asm`](../../patches/src/overworld_world_map.asm)).
- **Weather.** Rain music follows the global storm state ([rain music](rain_music.md)), and Dark Mountain lightning is disabled because its fixed palettes do not fit generated themes ([`overworld_lightning.asm`](../../patches/src/overworld_lightning.asm); see the [vanilla reference](../vanilla/lightning.md)).

## Area topology

Logical area IDs stay in `$8A`/`$040A`, while physical origins in the rearranged grid are resolved separately ([`overworld_topology.asm`](../../patches/src/overworld_topology.asm)). Transitions align the camera and preload source tiles before scrolling ([`overworld_camera_alignment.asm`](../../patches/src/overworld_camera_alignment.asm)). Entrances, pits, and special-overworld transitions are found from generated area-local lists rather than screen-specific scans ([entrances](entrances.md)). [`world_state.asm`](../../patches/src/world_state.asm) preserves the world context through death and save-and-quit.
