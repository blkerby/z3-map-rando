# Overworld asset loading

The patcher generates each overworld area's background graphics, area-dependent sprite graphics, and palettes as flat data, and the engine loads them directly from ROM. The patcher writes this data in [`asset_bundle.rs`](../../crates/patcher/src/asset_bundle.rs), and the engine reads it in [`overworld_assets.asm`](../../patches/src/overworld_assets.asm). This document describes the format they share and the runtime rules that depend on it. Addresses of the tables and regions are in [`symbols.inc`](../../patches/src/symbols.inc) and the [ROM map](../../patches/rom_map/banks_A0-BF.txt).

## Principles

- Every area load installs the complete applicable asset set. The engine tracks no residency and does not compare bundles, so a load repeats rows that are already resident.
- A zero sprite-sheet override emits nothing and keeps the current VRAM slot.
- Palette descriptors write the palette source at `$7EC300`. Presentation stays separate: existing loading and transition code copies or transforms `$7EC300` into the displayed mirror at `$7EC500` and requests a CGRAM upload through `$15`. Mirror and whirlpool effects instead leave the display covered and derive the displayed palette from the new source as their filters unwind.

## Asset keys and lookup

`OverworldAreaRecordVariantPointers` holds 256 little-endian 24-bit pointers, indexed by asset key. Normal, Dark World, and special-overworld screen IDs are their own keys, without runtime classification; the Triforce room uses `$88`. Credits overworld scenes use keys `$A0-$AF`, the initial sprite-sheet seed uses `$FE`, and the cool credits background uses `$FF`. Keys sharing data may point to the same list or record. Every reachable key has a generated entry, and unused keys point to one empty `$FF` variant.

Each pointer selects a list of area-record variants:

```text
1 byte:  inclusive maximum game state
3 bytes: area-record pointer, low byte, high byte, bank
```

The runtime selects the first entry whose maximum is at least `$7EF3C5`. Maximums are strictly increasing and the last entry's is `$FF`, so the scan needs no terminator or fallback. Adjacent identical variants are merged. Light World areas use maximums `$01`, `$02`, and `$FF` where their sprite graphics differ; other areas need only `$FF`.

## Area record

An area record is 32 bytes:

| Offset | Size | Content |
| ---: | ---: | --- |
| 0 | 3 | Full-reload batch sequence |
| 3, 6, 9, 12 | 3 each | Directional transition schedules, selected by `$0418` (see `InitializeScrollingAssetSchedule`) |
| 15 | 3 | Animation-track list, or zero |
| 18 | 3 | Ordinary overworld entrance list, or zero |
| 21 | 3 | Overworld pit entrance list, or zero |
| 24 | 1 | Background composition: `0` none, `1` half-add, `2` backdrop, `$FF` vanilla |
| 25 | 1 | Horizontal camera follow, in signed eighths |
| 26 | 1 | Horizontal drift, in signed eighths of a pixel per frame |
| 27 | 1 | Vertical camera follow, in signed eighths |
| 28 | 1 | Vertical drift, in signed eighths of a pixel per frame |
| 29 | 3 | Special-overworld transition list, or zero |

Pointers are little-endian 24-bit. The patcher currently writes the same schedule to all four directional fields.

An ordinary entrance list begins with its record count; each record is a 16-bit Map16 buffer offset, an entrance ID, and a flag byte whose bit 0 allows frog and dwarf followers. A pit list begins with its count; each record is a 16-bit Map16 buffer offset and an entrance ID. Keeping pits separate keeps them out of the ordinary per-frame scan.

A special-overworld transition list begins with its record count. Each record is a 16-bit area-local Map16 offset, a 16-bit special-area selector, and the forced movement direction. A zero selector returns to the saved ordinary overworld.

## Payloads and descriptors

Payloads are uncompressed and come in two interned sizes: a palette row of 16 colors (32 bytes) and a character row of 16 4bpp characters (512 bytes). Each distinct payload is stored once and shared by full reloads, transition schedules, and animation frames. No payload crosses a ROM bank boundary, and character destinations are aligned to 16-character rows.

A palette descriptor is five bytes:

```text
3 bytes: source pointer, bank first, then address low and high bytes
1 byte:  destination color index n, meaning $7EC300 + n * 2
1 byte:  color count
```

The source pointer may point within a row to transfer any contiguous color range; a range crossing a row boundary is split. A VRAM descriptor is the same three-byte source pointer followed by a destination row `n`, meaning VRAM word address `n * $100`. A source bank of zero terminates a descriptor list; payloads never live in bank zero.

## Batches, sequences, and schedules

A batch is a palette descriptor list followed by a VRAM descriptor list, each terminated by a zero byte. An empty list is its terminator alone. The main loop copies the palette payloads to `$7EC300` until the first zero; its cursor then addresses the VRAM list.

A full-reload sequence is a list of batch pointers, each written bank first, terminated by a zero bank. Sharing batch bodies costs three bytes per pointer and lets active-display code select a batch without scanning variable-length bodies. Batch boundaries affect only scheduling; payloads are never duplicated.

A directional transition schedule has three concatenated sections, consumed with one cursor:

```text
pre-scroll batch pointers, bank first, terminated by bank $00
scroll entries: 1-byte frame, then a bank-first batch pointer; terminated by frame $FF
post-scroll batch pointers, bank first, terminated by bank $00
```

Scroll entries are sorted by frame, with at most one per frame. Horizontal transitions have 32 scrolling frames (`$00-$1F`) and vertical transitions 28 (`$00-$1B`). The CLI's `--transition-asset-phase` option places the generated batches in the pre-scroll, scroll, or post-scroll section.

## Runtime

Forced-blank loads process every batch of the full-reload sequence synchronously, without NMI. Active-display loads, such as mirror, portal, and whirlpool travel, process one batch per frame. Scrolling transitions process one pre-scroll batch per frame before scrolling, submit each scroll entry's batch on its frame, and hold finalization until the post-scroll batches finish. After their palette descriptors, they copy the complete source palette to `$7EC500` so retained sprite-palette changes and new background rows appear together through `$15`.

NMI mode `$03` (`$17=$03`) transfers a VRAM descriptor list directly from ROM. The main loop stores the list's address in direct page `$35-$37` (low, high, bank) and sets `$17=$03` only when the list is nonempty; the `$12` main-loop/NMI handshake protects the pointer. The persistent schedule cursor between transition frames is in WRAM (see the [RAM map](../../patches/rom_map/ram.txt)). The BG streamer's `$18` list remains independent.

Transition batches have a 128-unit cost limit, where a character row costs 16 units and each eight palette colors cost one. Pre-scroll and post-scroll batches set `$0710` and `$0702`; scrolling batches set only `$0710`, keeping OAM updates while suppressing dynamic graphics and HUD work.

The four area-dependent sprite slots at VRAM words `$5000-$5FFF` belong to the generated bundles, at four character rows per slot, at the fixed destinations OAM tile numbers assume. Common and effect sprites, Link, followers, fixed auxiliary graphics, sprite palettes, and dungeon loading keep their existing paths.

## Animated tiles

An area may activate several animation tracks. An animation list contains bank-first track pointers terminated by a zero bank. Each track stores an 8-bit frame count, hold time, initial frame, initial countdown, and one bank-first batch pointer per frame; frame batches use the batch format with an empty palette list. The patcher derives the initial frame and countdown from an offline phase offset.

Each active track keeps its own frame index and countdown. When a countdown expires, the main loop appends that track's next descriptors to a dedicated WRAM batch, so several tracks may update in the same NMI. Animation scheduling is suspended while a transition owns the asset queue. The patcher must keep simultaneous tracks within the NMI budget, by changing their phases or rejecting the bundle rather than dropping updates.
