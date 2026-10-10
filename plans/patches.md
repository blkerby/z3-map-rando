# ASM patches

Each `.asm` file is assembled into an independent IPS patch. Keeping the patches
independent preserves conflict detection when they are applied on the Rust side.
`fastrom_base.ips` is exceptional as a base transformation which is applied
first; other patches may overwrite its changes without triggering conflicts.

`symbols.inc` defines common symbols and interfaces shared by otherwise
independent patches.

## Patch catalog

A planned build step bundles the individual IPS patches into
`build/patch_catalog.bin`, together with the matching symbol manifest and
item/location patching data extracted directly from `z3-json-data`. Keep
individual patches separate inside the catalog
to preserve conflict detection and support optional patches and the initial
`fastrom_base` transformation.

Application order, phases, and optional patch selection belong in patcher code,
not catalog records. The patch builder reads the logic source data directly to
extract stable `(room_id, item_id)` location mappings, ROM addresses, item receipt
IDs, and prize encodings. It does not consume the logic catalog. Builder modules
can share source-deserialization types in `catalog_builder`.

Use the [catalog envelope](README.md#catalog-format) and bincode payload.
The CLI embeds this single artifact or accepts `--patch-catalog <PATH>`;
the browser obtains it alongside the matching current WebAssembly patcher.

Always patch saved seeds with the latest patcher and its matching patch catalog,
so bug fixes and new patch-time customization options apply to old seeds too.
Saved seeds describe game content using stable identities, not patch addresses
or symbols, and do not require a particular patch catalog version. The current
patcher supports older seed formats through defaults or migrations.

### Symbol manifest

The machine-readable symbol manifest is not implemented yet. Today
`patches/src/symbols.inc` defines the shared ASM interface, while Rust patching
code uses separately defined address constants. The current assembly script
produces IPS files only.

Add an export step for the symbols needed to write seed-specific data and
allocate runtime assets. Generate the manifest alongside the matching IPS
artifacts and include it in the patch catalog. The patcher should consume these
exported addresses rather than duplicate the corresponding constants in Rust.
The exact symbol-export mechanism remains to be selected during implementation.


## Patch overview

### ROM and execution

- `rom_size.asm` declares the expanded 2 MiB ROM in the header.
- `fastrom_base.asm` mechanically converts vanilla ROM accesses to their FastROM
  mirrors
- `fastrom_extra.asm` enables FastROM and contains the related startup, NMI,
  title-screen timing, processor-bank, processor-flag, and credits-loading fixes
  that are not mechanical address conversions.

### Overworld map and gameplay

- `overworld_map_data.asm` replaces Map32 with flat, four-quadrant Map16 screen
  and overlay maps in WRAM.
- `overworld_map16_graphics.asm` expands Map16 graphical definitions to four
  banks and makes map construction, stripe generation, dynamic changes, and
  animated doors use them.
- `overworld_map16_properties.asm` gives each Map16 quadrant an independent
  property used by collision, terrain, hammer, and liftable-tile behavior.
- `overworld_dynamic_tiles.asm` resolves terrain, secret, door, and grave
  replacements from generated Map16 tables and draws arbitrary footprints.
- `overworld_entrances.asm` resolves ordinary entrances, pits, and special-area
  transitions from the current area's generated coordinate lists while
  preserving vanilla follower and Houlihan behavior, and opens generated
  wooden doors at those entrances.

### Rendering and VRAM

- `bg3_tilemap.asm` uses a 32-by-32 gameplay BG3 tilemap and handles the file
  select exception, menu and HUD row streaming, and credits wrapping.
- `overworld_bg_tilemaps.asm` owns the 64-by-32 BG1 and BG2 tilemaps, including
  bulk rendering, scrolling edges, dynamic Map16 writes, overlays, rain,
  credits, and transition rendering. Mirror, whirlpool, and transition margins
  remain here because they are part of streamed tilemap ownership.
- `mirror_bg1_hdma.asm` supplies the mirror-warp BG1 HDMA table that preserves
  parallax through the wave and dewave phases.
- `nmi_optimize.asm` contains NMI PPU and joypad optimizations, arbitrary DMA,
  unsafe-HUD suppression, and one-shot OAM suppression.
- `overworld_vram.asm` owns VRAM layouts: overworld selection and clearing, BG
  register setup, HUD relocation, module-15 layout switching, legacy upload
  rebasing, interior restoration, and credits and Triforce tilemap setup.
- `overworld_bg_color.asm` selects generated overworld backdrop colors during
  normal loading, cached restoration, and scrolling transitions.
- `overworld_lightning.asm` disables vanilla Dark Mountain lightning, including
  its thunder and Ganon's Tower and Turtle Rock palette effects.

### Generated overworld assets

- `overworld_assets.asm` resolves area records and owns palette and
  graphics loading, forced-blank batches, active-display queues, NMI character
  lists, generated palette ownership, area-dependent sprites, directional
  transition schedules, special-effect asset phases, and sprite-cache seeding.
- `overworld_animations.asm` activates generated animation tracks and manages
  their frames, phases, hold times, descriptors, and NMI scheduling. Vanilla
  dungeon animation remains unchanged.
