# Overworld asset loading

Status: submilestones 9B.1-9B.4 implemented; gameplay validation pending.

## Goal

Load generated overworld background and area-dependent sprite graphics
directly from ROM to VRAM, and palette source data directly from ROM to
`$7EC300`. Every area load installs the complete applicable asset set; do not
track residency or compare bundles. Zero sprite overrides explicitly retain
their current VRAM slots.

The first checkpoint changes the shared `Module08_00_LoadProperties` path used
by module `$08` when returning from an interior and module `$0A` when restoring
the saved gameplay area after a special overworld. Both resolve the destination
screen ID in `$8A` before reaching the common graphics and palette work, so
they can use the same asset lookup. Other overworld transitions and dungeon
loading remain unchanged.

## Payloads

`engine_check` generates flat 4bpp graphics and BGR555 palette data matching
the vanilla result. Keep the payload uncompressed unless its measured ROM cost
requires reconsideration.

Use two fixed payload sizes:

- One palette row: 16 colors, or 32 bytes.
- One character row: 16 sequential 4bpp tiles, or 512 bytes.

Store each distinct payload once. Full-reload, scrolling-transition, and later
animation lists all reference this shared payload data. Align tile allocation
and VRAM destinations to 16-character rows; align palette allocation and
`$7EC300` destinations to 16-color rows. No payload may cross a 64 KiB
ROM-bank boundary.

## DMA batches

Every descriptor is four bytes:

```text
1 byte:  ROM payload source bank
2 bytes: ROM payload source address, low byte then high byte
1 byte: destination row
```

Palette destination row `n` means byte address `$7EC300 + n * $20`. VRAM
destination row `n` means word address `n * $100`, because 16 4bpp characters
occupy `$100` VRAM words.

A source bank of zero terminates a descriptor list. Payloads will live only in
FastROM banks `$AA-$B7`, so there is never ambiguity between the terminator
`$00` and an actual bank.

Each DMA batch contains two null-terminated lists:

```text
zero or more palette-source descriptors
1 byte: $00
zero or more VRAM descriptors
1 byte: $00
```

The main loop copies each palette payload from ROM to `$7EC300` until the first
zero. At that point its cursor already identifies the VRAM list. Forced-blank
paths process that list directly; display-on paths pass its address to the NMI
queue. An empty list is represented by its terminator alone.

Palette presentation remains separate. Existing loading and transition code
copies or transforms `$7EC300` into the displayed mirror at `$7EC500`, then
uses `$15` when CGRAM must change. Mirror and whirlpool effects instead leave
the displayed palette covered and derive it from the new source palette as
their filters unwind.

## Full-reload batch sequences

A full reload is a null-terminated sequence of batch pointers. Batch pointers
also put their nonzero metadata bank first:

```text
1 byte:  batch bank, or $00 to end the sequence
2 bytes: batch address, low byte then high byte
```

The extra indirection costs three bytes per batch but lets the same batch
bodies be reused and lets active-display code select a batch without scanning
the variable-length bodies before it.

Forced-blank loading processes all batches synchronously. For active-display
full reloads such as mirror and whirlpool, the main loop processes one batch's
palette list per frame and submits its VRAM list to NMI when nonempty.
`engine_check` chooses how many payloads to place in each batch. Batch
boundaries affect scheduling only; they do not duplicate payload data.

## Scrolling-transition schedules

Each directional transition pointer selects one schedule with three
concatenated sections:

```text
pre-scroll batch pointers, terminated by bank $00
scroll entries, terminated by frame $FF
post-scroll batch pointers, terminated by bank $00
```

The pre-scroll and post-scroll sections use the same bank-first batch pointers
as a full reload. Process one batch per frame, delaying scroll start or final
handoff until the corresponding section reaches its terminator.

Each scroll entry is:

```text
1 byte:  zero-based scrolling frame, $00-$1F
1 byte:  batch bank
2 bytes: batch address, low byte then high byte
```

Entries are sorted by frame and at most one entry may select a given frame. A
batch itself contains every payload scheduled for that frame. The `$FF` frame
value ends the scrolling section. Horizontal transitions have 32 scrolling
frames (`$00-$1F`), while vertical transitions have 28 (`$00-$1B`); generated
schedules must stay within the applicable range. The post-scroll pointer
sequence begins immediately after the terminator.

This representation lets the runtime consume every section sequentially with
one cursor. It does not need batch counts, offsets, or a scan over descriptor
bodies. `engine_check` chooses the batch boundaries and frame assignments.

The generated data must reject overlapping or out-of-range destinations and
verify that every descriptor is contained within its payload. Graphics may
only target the milestone 9A overworld BG character region or area-dependent
OBJ rows `$50-$5F`. Palette descriptors may only target palette ranges owned
by the overworld background loader.

## ROM storage and lookup

Banks `$A1-$A6` hold generated Map16 definitions and properties. Keep
asset metadata and payloads in separate remaining bank ranges:

- `$A78000-$A9FFFF`: pointer table, asset records, and descriptor lists.
- `$AA8000-$B7FFFF`: shared palette and character payloads.

Reserve `$A78000-$A782FF` for `OverworldAssetBundlePointers`, a table of 256
little-endian 24-bit pointers. Each pointer selects a compact list of asset
record variants:

```text
1 byte: inclusive maximum game state
3 bytes: asset-record pointer, low byte, high byte, bank
```

The runtime selects the first entry whose maximum is at least `$7EF3C5`.
Bounds are strictly increasing and a final `$FF` entry is mandatory, so the
scan needs neither a terminator nor fallback logic. Adjacent variants with
identical records are merged. Light World areas use maximums `$01`, `$02`, and
`$FF` where their sprite graphics differ; fixed areas need only `$FF`.

The selected record uses this 24-byte format:

```text
3 bytes: full-reload list pointer
3 bytes: enter-from-west list pointer
3 bytes: enter-from-east list pointer
3 bytes: enter-from-north list pointer
3 bytes: enter-from-south list pointer
3 bytes: animation-track-list pointer, or zero
3 bytes: ordinary overworld entrance-list pointer, or zero
3 bytes: overworld pit entrance-list pointer, or zero
```

The entry side names the edge of the destination area: entering from west
means moving east through its west edge, and similarly for the other sides.
Full-reload paths use the first pointer. A scrolling transition selects one of
the following four pointers from the destination `$8A` and entry side.

An ordinary entrance list begins with its record count. Each record contains
a 16-bit Map16 buffer offset, an entrance ID, and a flag byte whose bit 0
allows frog and dwarf followers. A pit list also begins with its count, then
stores a 16-bit Map16 buffer offset and entrance ID in each record. Separate
lists keep pit coordinates out of the ordinary per-frame scan.

Normal, Dark World, and special-overworld screen IDs use the same lookup
without runtime classification. Credits overworld scenes use keys `$A0-$AF`,
the initial OBJ seed uses `$FE`, and the cool credits background uses `$FF`.
IDs which share data may point to the same list or record. Every reachable key
must have a generated entry; unused keys point to one empty `$FF` variant.

Asset records, batch sequences, transition schedules, DMA batches, and
animation definitions begin at `$A78300`. Keep each structure within one
metadata bank. Store each generated graphics or palette payload once in banks
`$AA-$B7` and let multiple batches reference it. Neither region may spill into
the other. `engine_check` reports their independent usage and fails if either
range overflows.

Submilestone 9B.1 uses this fixed ABI directly: the table starts at `$A78000`,
metadata starts at `$A78300`, and payloads start at `$AA8000`. `engine_check`
writes the vanilla bundle. A generated manifest is unnecessary unless these
locations later become configurable.

## Modules `$08` and `$0A`

Both modules are forced blank before `Module08_00_LoadProperties` loads
graphics and palettes. The first implementation indexes
`OverworldAssetBundlePointers` by `$8A`, selects the full-reload list, and
processes four batches synchronously. The first batch contains all six owned
palette rows; each batch contains eight 512-byte character rows, for at most
4 KiB of graphics DMA per batch. Submilestone 9B.4 extends the sequence with
additional OBJ rows as needed while retaining that per-batch limit. Forced-
blank loading does not use NMI.

Submilestone 9B.1 keeps sprite and dungeon assets on their existing loaders.
Replace the shared path's background half of `InitializeTilesets`,
`OverworldLoadScreensPaletteSet`, and `OverworldPalettesLoader` when their
behavior has been represented in the generated bundle. Keep
`DecompressAnimatedOverworldTiles` temporarily for the
first checkpoint.

Palette descriptors populate `$7EC300`. Retain the normal and special
overworld cache setup which derives `$7EC500` and requests the appropriate
`$15` upload. This keeps fades, lighting, damage effects, and later palette
uploads consistent.

Reload the full descriptor list even when the destination area selects the
same assets as the previous area. This is intentionally redundant and avoids
residency state in milestone 9B.

## Generated checks

For every module `$08` or `$0A` destination, `engine_check` should:

1. Reproduce the final vanilla BG character data and overworld-owned palette
   ranges.
2. Apply the generated descriptors and the existing presentation step in
   software, then compare `$7EC300`, VRAM, and CGRAM with the reference.
3. Check descriptor bounds, ROM-bank boundaries, payload sizes and alignment,
   null termination, batch pointers, schedule ordering, VRAM ownership,
   palette ownership, and the complete reload requirement.
4. Report total ROM size and forced-blank DMA bytes per area.

Runtime validation should cover Light World, Dark World, special graphics
sets reachable through modules `$08` and `$0A`, save loading into an interior
followed by an overworld exit, and palette fades and effects.
DMA logging should show that background VRAM and `$7EC300` receive their
generated payloads directly from ROM.

## Submilestone 9B.1: forced-blank playable checkpoint

Implemented: `engine_check` uses the importer's resolved map graphics and
palette sets to generate deduplicated payloads, records, four-batch full-load
sequences, and the 256-entry `$8A` lookup table. The shared module `$08`/`$0A`
path loads those assets directly from ROM while retaining vanilla sprite, HUD,
animated-tile, palette-cache, and presentation work. Other transition paths
remain unchanged.

Gameplay validation must confirm vanilla VRAM and CGRAM for Light World, Dark
World, and special-overworld returns before this checkpoint is marked complete.

## Submilestone 9B.2: scrolling transitions and ROM-source NMI queue

Introduce a descriptor queue dispatched through the obsolete `$17=$03`
overworld-stripe handler slot. Reusing the existing vector adds no work to
other NMI modes. The queue receives a 24-bit pointer to a null-terminated VRAM
descriptor list. Palette-source descriptors are processed by the main loop and
are not part of this NMI handler.

Store the active VRAM-list pointer in the vanilla-free direct-page range
`$35-$37`:

```text
$35: address low
$36: address high
$37: bank
```

This runtime byte order permits direct `[$35],Y` reads in NMI. After processing
a batch's palette list, the main loop advances past its terminator, stores the
resulting VRAM-list address in `$35-$37`, and sets `$17=$03` only when that list
is nonempty. The `$12` main-loop/NMI handshake protects the pointer. Its stale
value needs no clearing after NMI consumes `$17`.

Store the persistent scrolling-schedule cursor at `$7EC906-$7EC908`. This is
immediately after the BG streamer's temporary `$7EC900-$7EC904` parameters and
remains valid between transition frames. The main loop copies it to `$35-$37`
only while parsing metadata; queuing a VRAM list then replaces that temporary
direct-page value for NMI.

For each scrolling transition, select the list from the destination `$8A` and
the edge through which it is entered. These lists use the same batch and
descriptor formats as full reloads but contain only the payload rows required
for that transition. They reference the same shared payload catalog as the
full-reload lists.

For the vanilla arrangement, `engine_check` derives the expected source area
from the destination and entry side. Applying the transition list to the
source area's complete asset state must produce the destination state required
at the end of the transition. Empty lists are valid where no asset row changes.

The existing transition submodules process one pre-scroll batch per frame,
submit frame-indexed batches before their scroll step, and hold finalization
until post-scroll batches finish. Palette descriptors update `$7EC300`, then
copy the complete source palette to `$7EC500` so retained sprite-palette changes
and generated BG rows become visible together through `$15`. A nonempty VRAM
list uses `$17=$03`. Pre-scroll and post-scroll batches set `$0710` and `$0702`;
scrolling batches set only `$0710`, retaining OAM updates while suppressing
dynamic graphics and HUD work. The BG streamer's `$18` list remains independent.

The generated vanilla schedule includes palette rows 2-4 unless the first
auxiliary palette selector is `$FF`, and rows 5-7 unless the second selector is
`$FF`. It then includes the character rows for each nonzero area-specific sheet
override. A graphics row costs 16 batching units and a palette row costs two.
All transition batches have a 128-unit limit. Omitted palette groups and zero
sheet overrides preserve the assets already resident from the source area.
Full reloads still resolve these neutral entries to defaults and load the
complete asset set. The scroll and post-scroll sections are empty, and all four
directional record fields may share the same schedule for now.

Implemented: ordinary scrolling transitions now use generated ROM-source
payloads and the new mode `$03` NMI list. Gameplay validation must still confirm
all four directions, Light and Dark World boundaries, Castle/Pyramid parallax,
sprite and animated-tile continuity, and the absence of NMI overruns.
The `engine_check --transition-asset-phase` option can place the generated
batches in the pre-scroll, consecutive scroll-frame, or post-scroll section to
exercise each runtime path.

## Submilestone 9B.3: remaining transitions

Apply the full-reload lists to mirror and return-portal travel, whirlpools,
mosaic and other non-scrolling area changes, flute travel, and restoration
from the world map, credits, and other overworld scenes.

Forced-blank paths process every batch synchronously. Display-on full reloads
process palette-source descriptors in the main loop and submit nonempty VRAM
lists through `$17=$03`; they do not require another list format. Mirror and
whirlpool loads update `$7EC300` while their cover effect remains active, then
let the existing filter derive `$7EC500` and CGRAM as the effect clears.
Continue reloading the complete applicable asset set for every destination and
do not add residency tracking.

Implemented: mirror/portal and whirlpool transitions consume the existing
full-reload sequence one batch per frame while their palette effects cover the
change. Mosaic recovery retains only its sprite-palette work. Flute travel,
world-map restoration, credits scenes, and the Triforce room load full bundles
synchronously under forced blank. The final scrolling-names landscape uses
asset key `$FF` and a static shared BG1/BG2 tilemap rebased to `$6000-$6FFF`;
its text remains on BG3. The Triforce room uses its final `$8A=$88` record and
palette set `$0E`. The dormant Zora-area Triforce trigger is not treated as a
reachable load path.

Gameplay validation must still exercise each of these paths and verify NMI
timing, sprite continuity, palette-filter recovery, and vanilla appearance.

## Submilestone 9B.4: area-dependent sprite graphics

Move the four area-dependent sprite slots at VRAM word addresses
`$5000-$5FFF` into the generated asset bundles. Each slot contains 64 8x8
characters, so it maps to four existing 16-character payload rows. Preserve
the fixed destinations assumed by OAM tile numbers.

The importer uses vanilla sprite-set IDs only while compiling the final 4bpp
rows, including the loader's upper- or lower-palette-half conversion. Runtime
records contain only destination rows and payload pointers. A zero sheet
override emits nothing and retains the current VRAM slot in both full and
scrolling loads.

Normal Light World records use the game-state variants described above. Dark
World and fixed special areas use one `$FF` variant; Triforce uses asset key
`$88`. Credits overworld scenes use their dedicated keys and scripted sprite
sets. File initialization writes sheet `$46` to all four slots through key
`$FE`, matching vanilla's initial resident state.

Put sprite and background rows in the same batches so `engine_check` schedules
their combined NMI cost and enforces the phase-specific weighted limits above.
Converted overworld paths no longer stage these four slots in WRAM or run the
vanilla `$19` upload. Common/effect sprites at `$4400-$47FF`, Link graphics,
followers, fixed auxiliary graphics, sprite palettes, and unrelated NMI work
remain on their existing paths. Dungeon and other non-generated callers retain
the vanilla `$0AA3` cache behavior.

Implemented: the shared runtime resolver selects records by game state for
synchronous, full-sequence, and directional loads. Full loads and every
converted overworld transition use generated OBJ rows, while zero overrides
retain the current slots. `engine_check` validates variant coverage, full
loads, ordinary adjacencies, credits keys, the initial seed, destinations, and
batch limits. Gameplay validation must still cover game-state changes,
dungeon/presentation boundaries, every transition type, and NMI timing.

## Submilestone 9B.5: animated tiles

Implemented: generated ROM frames replace the old overworld WRAM animation
cache and use the queue introduced in 9B.2.

An area may activate multiple animation tracks. Each generated track defines
one or more destination character rows, the shared row payloads for every
frame, its frame count, update period, and initial phase. Runtime state keeps
an independent frame index and countdown for each active track. When a
countdown expires, the main loop appends that track's next ROM-to-VRAM
descriptors to a dedicated WRAM list; several tracks may therefore update in
the same NMI while others update on different frames.

Build one batch in dedicated WRAM using the same null-terminated
palette-then-VRAM format, normally with an empty palette list. The `$12`
main-loop/NMI handshake protects it. Suspend ordinary animation scheduling
while a transition owns the asset queue, and do not reuse `$1100`, which
remains owned by BG1/BG2 streaming.

Remove only the animated-background portion of the normal per-frame graphics
DMA group; Link, item, follower, and OBJ updates remain unchanged. Generated
animation schedules must enforce the combined NMI byte and cycle budget. If
simultaneous tracks exceed it, change their generated phases or reject the
bundle rather than silently dropping an update.

This checkpoint is complete when vanilla animations match, at least two test
tracks can run with different periods or phases, and no animated overworld
graphics depend on `DecompressAnimatedOverworldTiles` or its WRAM frame cache.

An animation list contains bank-first track pointers followed by a zero bank.
Each track stores an 8-bit frame count, hold time, initial frame, initial
countdown, and one bank-first batch pointer per frame. Frame batches use the
existing empty-palette-plus-character descriptor format. `phase_offset` is an
offline game-tick offset used to derive the initial frame and countdown.

Milestone 9B is complete only after every overworld load and restoration path
uses the generated descriptors and no overworld background asset or
area-dependent sprite graphic depends on the old graphics or palette loaders.

## Deferred optimization: fine-grained CGRAM uploads

This optimization applies only to ordinary area-scrolling transitions, where
no transition palette filter is active. Initially these transitions may use
vanilla `$15` to upload all 512 bytes of `$7EC500` even when only one or two
background palette rows changed. Keep this simple path unless NMI measurements
show that it is too expensive.

If needed, extend the existing palette control:

```text
$15 = $00: no palette upload
$15 = $80: upload rows selected by an 8-bit BG-row mask
other nonzero values: upload all 512 bytes
```

Each dirty bit selects one of the eight 32-byte background palette rows. The
main loop loads the row into `$7EC300`, copies it unchanged to the matching
`$7EC500` row, and sets its bit. NMI transfers each selected row directly from
`$7EC300` to the corresponding CGRAM row, then clears the mask. Keeping
`$7EC500` synchronized ensures that a later vanilla full upload or palette
effect cannot restore stale colors.

A pending full upload supersedes the mask; if legacy code increments
`$15=$80`, the resulting `$81` also selects the full upload. The full-upload
path must clear the mask as well.

This palette submode remains independent of `$17=$03`, so fine-grained CGRAM
and ROM-to-VRAM queues may run in the same NMI when their combined measured
budget permits it.
