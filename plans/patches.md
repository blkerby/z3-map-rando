# ASM patches

Each `.asm` file is assembled into an independent IPS patch. Keeping the patches
independent preserves conflict detection when they are applied on the Rust side.
`fastrom_base.ips` is exceptional as a base transformation which is applied
first; other patches may overwrite its changes without triggering conflicts.

`symbols.inc` defines common symbols and interfaces shared by otherwise
independent patches.

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
  banks and makes map construction, stripe generation, dynamic changes, special
  triggers, and animated doors use them.
- `overworld_map16_properties.asm` gives each Map16 quadrant an independent
  property used by collision, terrain, hammer, and liftable-tile behavior.
- `overworld_entrances.asm` resolves ordinary entrances and pits from the
  current area's generated lists while preserving vanilla follower and Houlihan
  behavior. Door-opening animations remain separate.

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

### Generated overworld assets

- `overworld_assets.asm` resolves area records and owns palette and
  graphics loading, forced-blank batches, active-display queues, NMI character
  lists, generated palette ownership, area-dependent sprites, directional
  transition schedules, special-effect asset phases, and sprite-cache seeding.
- `overworld_animations.asm` activates generated animation tracks and manages
  their frames, phases, hold times, descriptors, and NMI scheduling. Vanilla
  dungeon animation remains unchanged.
