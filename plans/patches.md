# ASM patches

Each patch `.asm` file is assembled into an independent IPS patch; `symbols.asm` is the separate symbol-export entry point. Keeping the patches independent preserves conflict detection when they are applied on the Rust side. `fastrom_base.ips` is exceptional as a base transformation which is applied first; other patches may overwrite its changes without triggering conflicts.

`symbols.inc` defines common symbols and interfaces shared by otherwise independent patches.

## Patch catalog

`catalog_builder patches` bundles the individual IPS patches into `build/patch_catalog.bin`, together with the matching symbol manifest and item/location patching data extracted directly from `z3-json-data`. `all` includes this operation alongside logic and retiling catalog builds. Individual patches remain separate inside the catalog to preserve conflict detection and support optional patches and the initial `fastrom_base` transformation.

Application order, phases, and optional patch selection belong in patcher code, not catalog records. The patch builder reads `rooms/` and the grouped item sections in `items.json` directly to extract stable `(room_id, item_id)` location mappings, optional ROM addresses, and item receipt IDs. Prize encodings are derived from prize identity using the lookup below, and prize addresses are derived from the room's `dungeonId`. It does not consume the logic catalog. Builder modules share source-deserialization types and file readers in `catalog_builder::z3_json_data`. Omitted receipt-ID addresses produce empty offset lists unless this project's patch supplies an address, as it does for dungeon prizes and the Catfish/Zora gifts. The fixed flute-activation event is not a placement location. Specific capacity-upgrade receipts, the progressive boomerang, and the Triforce receipt are defined locally because they are not individual shared-data records.

`PatchCatalog.patches` is a plain `PatchIps` struct, with one `Ips` field per patch named after its ASM filename stem. Each `Ips` stores its diagnostic filename and complete payload bytes. During building, Serde deserializes the named patches into this struct through an in-memory JSON value. Required fields and `deny_unknown_fields` reject missing or extra patches. Adding or renaming a patch requires updating the struct; optional application stays in patcher code.

The runtime `patch_catalog` crate provides streaming writer and reader APIs using the [catalog envelope](README.md#catalog-format) and Zstd-compressed bincode payload. Packaging writes the catalog directly after invalidating its cache fingerprint. IPS bytes and symbols are read under the assembly cache lock so the catalog receives a consistent snapshot. `theme_check --patch-catalog <PATH>` consumes the catalog, defaulting to `build/patch_catalog.bin`. The main CLI consumes this catalog for both saved-seed patching and combined generation/patching, using bundled bytes or `--patch-catalog`. IPS files are generated under `build/patches/`; neither IPS files nor separate Python builders are tracked in source control. The browser will obtain the artifact alongside the matching current WebAssembly patcher.

Always patch saved seeds with the latest patcher and its matching patch catalog, so bug fixes and new patch-time customization options apply to old seeds too. Saved seeds describe game content using stable identities, not patch addresses or symbols, and do not require a particular patch catalog version. Released seed readers will support older seed formats through defaults or migrations; prototype migrations are deferred.

### Dungeon prize encodings

`z3-json-data` prize records supply `name`, `itemReceiptId`, and optional notes. The prize identity determines its pendant/crystal category and crystal number; ownership masks, falling-object subtypes, and map graphics are implementation details maintained in this project. The labels in the tables below describe derived values, not fields in the shared data.

| Prize name | Inventory mask | Map properties | Map tile | Number tile | Falling-prize ID |
|---|---|---|---|---|---|
| `PendantOfCourage` | `$04` | `$38` | `$60` | — | `$01` |
| `PendantOfWisdom` | `$01` | `$32` | `$60` | — | `$03` |
| `PendantOfPower` | `$02` | `$34` | `$60` | — | `$02` |
| `Crystal1` | `$02` | `$34` | `$64` | `$7F` | `$06` |
| `Crystal2` | `$10` | `$34` | `$64` | `$79` | `$06` |
| `Crystal3` | `$40` | `$34` | `$64` | `$6C` | `$06` |
| `Crystal4` | `$20` | `$34` | `$64` | `$6D` | `$06` |
| `Crystal5` | `$04` | `$34` | `$64` | `$6E` | `$06` |
| `Crystal6` | `$01` | `$34` | `$64` | `$6F` | `$06` |
| `Crystal7` | `$08` | `$34` | `$64` | `$7C` | `$06` |

These are vanilla Japanese 1.0 values. Inventory masks identify ownership bits in saved RAM `$7EF374` for pendants or `$7EF37A` for crystals; their order is not numerical crystal order. Map properties are the packed OBJ attribute byte, including palette, priority, flips, and graphics page. The map tile starts a 16x16 icon using tiles `n`, `n + 1`, `n + $10`, and `n + $11`. Number tiles contain the numerals displayed while crystal icons blink; pendants have no number tile. Falling-prize IDs index the vanilla spawning tables at SNES `$098B7C` and are distinct from item receipt IDs: `$01` selects receipt `$37`, `$03` selects `$38`, `$02` selects `$39`, and `$06` selects the shared crystal receipt `$20`.

### Dungeon prize address derivation

`z3-json-data` supplies each prize dungeon's raw vanilla `dungeonId` and the prize identity. It does not prescribe ROM table addresses or custom ASM layouts. This project derives the address of each dungeon's entry from its dungeon identity. All ROM offsets below refer to an unheadered Japanese 1.0 ROM; SNES addresses identify the original vanilla tables, whose FastROM mirrors have the same file offsets.

The dungeon ID is the even value stored in `$040C`, distinct from the logical room ID in `z3-json-data`. Let `i = dungeonId / 2`.

| Dungeon ID | Index `i` | Vanilla dungeon |
|---|---|---|
| `$00` | `$00` | Sewers |
| `$02` | `$01` | Hyrule Castle |
| `$04` | `$02` | Eastern Palace |
| `$06` | `$03` | Desert Palace |
| `$08` | `$04` | Agahnim's Tower |
| `$0A` | `$05` | Swamp Palace |
| `$0C` | `$06` | Palace of Darkness |
| `$0E` | `$07` | Misery Mire |
| `$10` | `$08` | Skull Woods |
| `$12` | `$09` | Ice Palace |
| `$14` | `$0A` | Tower of Hera |
| `$16` | `$0B` | Thieves' Town |
| `$18` | `$0C` | Turtle Rock |
| `$1A` | `$0D` | Ganon's Tower |

`$FF` denotes a cave rather than another dungeon ID. The vanilla prize-mask table at SNES `$02A09B` and falling-prize table at `$01C6FC` each have 13 entries, ending at Turtle Rock; neither contains an entry for Ganon's Tower. The Sewers, Hyrule Castle, and Agahnim's Tower entries do not award pendants or crystals. The ten prize locations therefore use these offsets:

| Prize value | ROM offset |
|---|---|
| `inventoryMask` | `0x1209B + i` |
| `fallingPrizeId` | `0xC6FC + i` |

Map marker order differs from dungeon-ID order. Let `s` be the marker slot in this table, and let `p` be the vanilla marker phase in `$7EF3C7`: `3` for the original Light World dungeon markers or `7` for the original Dark World dungeon markers. These refer to the dungeon's vanilla marker, independently of its randomized prize category.

| Marker slot `s` | Light World dungeon (`p = 3`) | Dark World dungeon (`p = 7`) |
|---|---|---|
| 0 | Eastern Palace | Palace of Darkness |
| 1 | Tower of Hera | Skull Woods |
| 2 | Desert Palace | Turtle Rock |
| 3 | — | Thieves' Town |
| 4 | — | Misery Mire |
| 5 | — | Ice Palace |
| 6 | — | Swamp Palace |

The vanilla icon tables begin at SNES `$0ABEF2`, with nine pairs of properties/tile bytes per marker slot. Vanilla's crystal-number table is at `$0AC527-$0AC52D`, ordered in reverse of the Dark World marker slots.

| Prize value | ROM offset |
|---|---|
| `mapIconProperties` | `0x53EF2 + 0x12 * s + 2 * p` |
| `mapIconTile` | `0x53EF3 + 0x12 * s + 2 * p` |
| `mapNumberTile` for an original crystal dungeon | `0x5452D - s` |

There is no independent vanilla crystal-number entry for each Light World dungeon, and no vanilla per-dungeon prize-type table. Vanilla checks choose pendant RAM `$7EF374` or crystal RAM `$7EF37A` from the current world. Arbitrary prize shuffling therefore also needs ASM hooks for category-dependent inventory checks, map marker collection checks, and crystal-number lookup; writing the vanilla table entries alone is insufficient.

#### Historical ALTTPR/SMZ3 layout

The former six-address prize lists in `z3-json-data` used this order: inventory mask, map properties, map tile, custom prize type, custom map-number tile, and falling-prize ID. The first three and last entries use the vanilla formulas above. The two custom entries use these expanded-ROM offsets in the earlier ALTTPR/SMZ3 layout:

| Custom value | ROM offset | Encoding |
|---|---|---|
| Prize type | `0x180050 + i` | `$00` for pendant, `$40` for crystal |
| Map-number tile | `0x18007C - 2 * s + w` | Assigned crystal's numeral tile; `w = 0` for an original Light World dungeon and `w = 1` for an original Dark World dungeon |

That number table contains unused `$69` placeholders for pendants. Our existing `!DungeonPrizeTypes = $B08050` uses the historical category table; the map-number lookup has not yet been migrated from vanilla. These are patch implementation details, not required shared-data conventions. Newer ALTTPR ASM reorganizes the marker and number tables, so its layouts must not be mixed with these formulas.

The vanilla encoding lookup above uses green pendant tile `$60` and crystal properties `$34`. ALTTPR/SMZ3's green pendant tile `$62` requires replacement map artwork with an asterisk; vanilla `$62` is the Master Sword image. Their red Crystals 5 and 6 use properties `$32`. Adopting either visual change belongs in this project's patching code and assets.

Crystal cutscene backdrop setup also needs all ten prize boss rooms. Vanilla searches only the seven original crystal rooms without an exit for an unmatched room, so crystals in Eastern, Desert, or Hera otherwise freeze before the maiden appears. `item_placements.asm` replaces this lookup, retaining the seven vanilla backdrop offsets and adding Eastern `$1658`, Desert `$1618`, and Hera `$0EBA`. Hera's offset accounts for its prize falling at the center of the full room rather than a quadrant center. The separate maiden-dialogue hook selects messages by the assigned crystal's ownership bit.

### Symbol manifest

`patches/src/symbols.inc` defines the shared ASM interface. Use `%export_symbol(InternalName, exported_name, $Value)` for symbols consumed by Rust; this defines both `!InternalName` for ASM and the assigned label `export_exported_name` for Asar's symbol output. The exported name matches its Rust field directly, for example `%export_symbol(Map16TopLeft, map16_top_left, $A18000)`. Symbols shared only between ASM files retain ordinary `!Name = $Value` definitions. Routine labels and their address anchors need no extra export labels.

`patches/src/symbols.asm` includes the interface and is assembled independently with `--symbols=nocash`. It produces `build/patches/symbols.sym`, not an IPS patch. `catalog_builder patches` caches this file with the same dependency and assembler fingerprints as the patches, selects `export_` labels, and removes their prefix before importing them. Asar's symbol output retains 24 bits, so this interface is for addresses and small constants.

`PatchCatalog.symbols` is a plain `PatchSymbols` struct with Serde's `Deserialize` and schema-hash derives. The builder deserializes the unprefixed export map directly through an in-memory map adapter. Required fields reject missing exports, and `deny_unknown_fields` rejects unconsumed exports, establishing exact correspondence between the ASM exports and the serialized fields. Rust needs no separate name mappings or constructor macro. Exports cover Map16 graphical/property tables, flat-map data and pointer tables, background colors, area-record and event pointer tables, and asset storage addresses.

The builder packages the typed manifest alongside the assembled IPS bytes. The shared patcher uses these fields to place runtime data and construct pointers, and reads IPS payloads directly from `PatchIps`.


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
