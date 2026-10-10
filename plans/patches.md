# ASM patches

Each patch `.asm` file is assembled into an independent IPS patch; `symbols.asm` is the separate symbol-export entry point. Keeping the patches independent preserves conflict detection when they are applied on the Rust side. `fastrom_base.ips` is exceptional as a base transformation which is applied first; other patches may overwrite its changes without triggering conflicts.

`symbols.inc` defines common symbols and interfaces shared by otherwise independent patches.

## Patch catalog

`catalog_builder patches` bundles the individual IPS patches into `build/patch_catalog.bin`, together with the matching symbol manifest and item/location patching data extracted directly from `z3-json-data`. `all` includes this operation alongside logic and retiling catalog builds. Individual patches remain separate inside the catalog to preserve conflict detection and support optional patches and the initial `fastrom_base` transformation.

Application order, phases, and optional patch selection belong in patcher code, not catalog records. The patch builder reads `rooms/` and `items.json` directly to extract stable `(room_id, item_id)` location mappings, ROM addresses, item receipt IDs, and prize encodings. It does not consume the logic catalog. Builder modules share source-deserialization types and file readers in `catalog_builder::source`. Unknown addresses produce empty offset lists; dungeon-prize offsets and bytes retain their authored order. The fixed flute-activation event is not a placement location.

`PatchCatalog.patches` is a plain `PatchIps` struct, with one `Vec<u8>` field per patch named after its ASM filename stem. During building, Serde deserializes the filename/payload entries directly into this struct through in-memory map and sequence adapters. Required fields and `deny_unknown_fields` reject missing or extra patches. Adding or renaming a patch requires updating the struct; optional application stays in patcher code.

The runtime `patch_catalog` crate provides streaming writer and reader APIs using the [catalog envelope](README.md#catalog-format) and Zstd-compressed bincode payload. Packaging publishes the completed file through a temporary file. IPS bytes and symbols are read under the assembly cache lock so the catalog receives a consistent snapshot. CLI embedding and `--patch-catalog <PATH>` support remain planned; the browser will obtain the artifact alongside the matching current WebAssembly patcher.

Always patch saved seeds with the latest patcher and its matching patch catalog, so bug fixes and new patch-time customization options apply to old seeds too. Saved seeds describe game content using stable identities, not patch addresses or symbols, and do not require a particular patch catalog version. The current patcher supports older seed formats through defaults or migrations.

### Symbol manifest

`patches/src/symbols.inc` defines the shared ASM interface. Use `%export_symbol(InternalName, exported_name, $Value)` for symbols consumed by Rust; this defines both `!InternalName` for ASM and the assigned label `export_exported_name` for Asar's symbol output. The exported name matches its Rust field directly, for example `%export_symbol(Map16TopLeft, map16_top_left, $A18000)`. Symbols shared only between ASM files retain ordinary `!Name = $Value` definitions. Routine labels and their address anchors need no extra export labels.

`patches/src/symbols.asm` includes the interface and is assembled independently with `--symbols=nocash`. It produces `build/patches/symbols.sym`, not an IPS patch. `catalog_builder asm` caches this file with the same dependency and assembler fingerprints as the patches, selects `export_` labels, and removes their prefix before importing them. Asar's symbol output retains 24 bits, so this interface is for addresses and small constants.

`PatchCatalog.symbols` is a plain `PatchSymbols` struct with Serde's `Deserialize` derive alongside its bincode and schema-hash derives. The builder deserializes the unprefixed export map directly through an in-memory map adapter. Required fields reject missing exports, and `deny_unknown_fields` rejects unconsumed exports, establishing exact correspondence between the ASM exports and the serialized fields. Rust needs no separate name mappings or constructor macro. Initial exports cover Map16 graphical/property tables and dynamic-tile, cutscene, and overlay pointer tables.

The builder packages the typed manifest alongside the assembled IPS bytes. Migration of existing Rust address constants to these fields remains part of the later patcher integration.


## Patch overview

### ROM and execution

- `rom_size.asm` declares the expanded 2 MiB ROM in the header.
- `fastrom_base.asm` mechanically converts vanilla ROM accesses to their FastROM mirrors
- `fastrom_extra.asm` enables FastROM and contains the related startup, NMI, title-screen timing, processor-bank, processor-flag, and credits-loading fixes that are not mechanical address conversions.

### Overworld map and gameplay

- `overworld_map_data.asm` replaces Map32 with flat, four-quadrant Map16 screen and overlay maps in WRAM.
- `overworld_map16_graphics.asm` expands Map16 graphical definitions to four banks and makes map construction, stripe generation, dynamic changes, and animated doors use them.
- `overworld_map16_properties.asm` gives each Map16 quadrant an independent property used by collision, terrain, hammer, and liftable-tile behavior.
- `overworld_dynamic_tiles.asm` resolves terrain, secret, door, and grave replacements from generated Map16 tables and draws arbitrary footprints.
- `overworld_entrances.asm` resolves ordinary entrances, pits, and special-area transitions from the current area's generated coordinate lists while preserving vanilla follower and Houlihan behavior, and opens generated wooden doors at those entrances.

### Rendering and VRAM

- `bg3_tilemap.asm` uses a 32-by-32 gameplay BG3 tilemap and handles the file select exception, menu and HUD row streaming, and credits wrapping.
- `overworld_bg_tilemaps.asm` owns the 64-by-32 BG1 and BG2 tilemaps, including bulk rendering, scrolling edges, dynamic Map16 writes, overlays, rain, credits, and transition rendering. Mirror, whirlpool, and transition margins remain here because they are part of streamed tilemap ownership.
- `mirror_bg1_hdma.asm` supplies the mirror-warp BG1 HDMA table that preserves parallax through the wave and dewave phases.
- `nmi_optimize.asm` contains NMI PPU and joypad optimizations, arbitrary DMA, unsafe-HUD suppression, and one-shot OAM suppression.
- `overworld_vram.asm` owns VRAM layouts: overworld selection and clearing, BG register setup, HUD relocation, module-15 layout switching, legacy upload rebasing, interior restoration, and credits and Triforce tilemap setup.
- `overworld_bg_color.asm` selects generated overworld backdrop colors during normal loading, cached restoration, and scrolling transitions.
- `overworld_lightning.asm` disables vanilla Dark Mountain lightning, including its thunder and Ganon's Tower and Turtle Rock palette effects.

### Generated overworld assets

- `overworld_assets.asm` resolves area records and owns palette and graphics loading, forced-blank batches, active-display queues, NMI character lists, generated palette ownership, area-dependent sprites, directional transition schedules, special-effect asset phases, and sprite-cache seeding.
- `overworld_animations.asm` activates generated animation tracks and manages their frames, phases, hold times, descriptors, and NMI scheduling. Vanilla dungeon animation remains unchanged.
