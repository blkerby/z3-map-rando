# Overworld and underworld entrances

Vanilla does not store overworld and underworld entrances as one bidirectional
connection list. It uses separate parallel tables for finding an overworld
entrance, initializing the destination room, and returning to the overworld.

## Overworld doors

Ordinary overworld doors use 129 records split across three tables in bank
`$1B`:

| Address | Field | Size | Meaning and effect |
| --- | --- | --- | --- |
| `$1BB96F` | Overworld screen ID | 129 words | Compared with the current screen in `$040A`; it identifies the area containing the entrance. |
| `$1BBA71` | Map16 buffer offset | 129 words | Compared with the contacted tile's byte offset in the current Map16 buffer at `$7E2000`. Rows are `$80` bytes apart and each Map16 entry occupies two bytes. |
| `$1BBB73` | Entrance ID | 129 bytes | Written to `$010E` after a match and used as the index into `EntranceData`. Multiple overworld locations can select the same entrance ID. |

[`UseOverworldEntrance`](../jpdasm/bank_1B.asm#L12048) calculates the contacted
Map16 offset and checks it together with the current overworld screen against
the first two tables. A match supplies the entrance ID from the third table,
which is written to `$010E`.

For example, record `$00` describes Link's house:

```text
overworld screen $2C + Map16 offset $0796 -> entrance ID $01
```

The Map16 offset is an offset into the current overworld Map16 buffer at
`$7E2000`, not a Map16 tile ID.

## Pit entrances

Pits use separate tables beginning at [`$1BB800`](../jpdasm/bank_1B.asm#L11465):

| Address | Field | Size | Meaning and effect |
| --- | --- | --- | --- |
| `$1BB800` | Map16 buffer offset | 19 words | Byte offset of the pit tile in the current Map16 buffer at `$7E2000`. |
| `$1BB826` | Overworld screen ID | 19 words | Area containing the pit; checked together with the Map16 offset to distinguish identical local coordinates on different screens. |
| `$1BB84C` | Entrance ID | 20 bytes | Selects `EntranceData` through `$010E`. Entries `$00-$12` correspond to the 19 locations; entry `$13` is the unmatched-pit fallback for the Houlihan room. |

[`GetPitEntranceDestination`](../jpdasm/bank_1B.asm#L11532) matches the first 19
entries by offset and screen. If none matches, it uses the twentieth entrance
ID, `$82`, for the Houlihan room. The selected ID is also written to `$010E`.

## Underworld entrance data

The entrance ID indexes 133 records, `$00-$84`, in
[`EntranceData`](../jpdasm/bank_02.asm#L13092). This is another set of parallel
tables, beginning at `$02C577`, with these fields:

| Address | Field | Bytes per entrance | Meaning and effect |
| --- | --- | --- | --- |
| `$02C577` | Room ID | 2 | Becomes the current and initial underworld room in `$A0` and `$048E`. |
| `$02C681` | Camera scroll boundaries | 8 | Supplies the high bytes of four vertical camera clamps at `$0600-$0606` and four horizontal clamps at `$0608-$060E`. These stop the camera at the appropriate room or quadrant edges. |
| `$02CAA9` | Horizontal scroll position | 2 | Initial horizontal BG and camera position, copied to `$E0`, `$E2`, `$011E`, and `$0120`. |
| `$02CBB3` | Vertical scroll position | 2 | Initial vertical BG and camera position, copied to `$E6`, `$E8`, `$0122`, and `$0124`. |
| `$02CCBD` | Link Y coordinate | 2 | Initial world-space Y position in `$20`. The loader skips the coordinate fields during the initial pre-game state. |
| `$02CDC7` | Link X coordinate | 2 | Initial world-space X position in `$22`, subject to the same pre-game exception. |
| `$02CED1` | Camera trigger Y coordinate | 2 | Upper vertical threshold in `$0618`; the loader derives the paired lower threshold in `$061A` by adding two. Link crossing these thresholds makes the camera follow vertically. |
| `$02CFDB` | Camera trigger X coordinate | 2 | Left horizontal threshold in `$061C`; the loader derives the paired right threshold in `$061E` by adding two. |
| `$02D0E5` | Main graphics set | 1 | Written to `$0AA1` to select the room's primary underworld graphics set. |
| `$02D16A` | Floor | 1 | Initial signed dungeon-floor value in `$A4`, used by floor handling and the HUD. Rooms `$0100` and above override it to zero. |
| `$02D1EF` | Dungeon ID | 1 | Written to `$040C`; selects dungeon-specific state such as dungeon flags and inventory. `$FF` denotes a non-dungeon interior. |
| `$02D274` | Indoor door state | 1 | Written to `$6C`. A nonzero value marks Link as still occupying the entrance doorway, affecting movement, collision, and drawing until he clears it. |
| `$02D2F9` | Layer | 1 | Packed layer selectors: the upper nibble becomes Link's layer in `$EE`, and the lower nibble becomes the room/collision layer in `$0476`. |
| `$02D37E` | Camera scroll controller | 1 | Packed camera-boundary selectors: the upper nibble becomes horizontal selector `$A6`, and the lower nibble becomes vertical selector `$A7`. The camera code uses them as offsets into the clamp values above. |
| `$02D403` | Quadrant | 1 | Packed starting-quadrant selectors: the upper nibble becomes horizontal quadrant `$A9`, and the lower nibble becomes vertical quadrant `$AA`. They determine which half of a multi-quadrant room is active. |
| `$02D488` | Overworld door tilemap | 2 | Return-side door descriptor stored in `$0696`. Normally it is the overworld Map16 buffer offset to replace with an open doorway; zero suppresses the update, `$FFFF` marks the reverse-facing exit, and bit 15 selects the bombable-entrance replacement. |
| `$02D592` | Song | 1 | Requested room music in `$0132`. Special values request transfer or volume behavior; song `$03` changes to the cave theme after Zelda has been rescued. |

[`LoadUnderworldEntrance`](../jpdasm/bank_02.asm#L15443) reads the selected
values and initializes the indoor room. Continuing the Link's house example,
`EntranceData[$01]` selects room `$0104` and its starting position, camera,
graphics, and music.

## Returning to the overworld

Returning outside is partly table-driven and partly based on state cached when
Link entered.

[`UnderworldExitData`](../jpdasm/bank_02.asm#L16267) contains 79 parallel
records, `$00-$4E`, beginning at `$02DAEE`. Each record includes:

| Address | Field | Bytes per exit | Meaning and effect |
| --- | --- | --- | --- |
| `$02DAEE` | Underworld room ID | 2 | Lookup key compared with the current room in `$A0`. The matching index selects every other field in the exit record. |
| `$02DB8C` | Destination overworld screen ID | 1 | Written to `$8A` and `$040A` as the area to load. The ID also carries the normal Light World, Dark World, or special-overworld screen selection. |
| `$02DBDB` | Exit VRAM address | 2 | Packed overworld tilemap position stored in `$84`. The loader derives the internal tilemap X and Y positions in `$86` and `$88`, which tell the screen builder which part of the area surrounds the exit. |
| `$02DC79` | Vertical scroll position | 2 | Initial vertical overworld BG and camera position, copied to `$E6`, `$E8`, `$0122`, and `$0124`. |
| `$02DD17` | Horizontal scroll position | 2 | Initial horizontal overworld BG and camera position, copied to `$E0`, `$E2`, `$011E`, and `$0120`. |
| `$02DDB5` | Link Y coordinate | 2 | Link's world-space Y position immediately outside the exit, written to `$20`. |
| `$02DE53` | Link X coordinate | 2 | Link's world-space X position immediately outside the exit, written to `$22`. |
| `$02DEF1` | Camera trigger Y coordinate | 2 | One vertical camera-follow threshold in `$0618`; the paired threshold in `$061A` is this value minus two. |
| `$02DF8F` | Camera trigger X coordinate | 2 | One horizontal camera-follow threshold in `$061C`; the paired threshold in `$061E` is this value minus two. |
| `$02E02D` | Vertical camera scroll adjustment | 1 | Signed initial vertical camera delta in `$0624`; the loader sign-extends it and stores its negation in `$0626`. |
| `$02E07C` | Horizontal camera scroll adjustment | 1 | Signed initial horizontal camera delta in `$0628`; the loader sign-extends it and stores its negation in `$062A`. |
| `$02E0CB` | Door graphic | 2 | Door-opening descriptor stored in `$0696`. Zero means no ordinary door update, `$FFFF` marks the reverse-facing case, a normal value is a Map16 buffer offset for a wooden doorway, and bit 15 selects a bombable opening. |
| `$02E169` | Door graphic location | 2 | Map16 buffer offset stored in `$0698` for a 2x2 animated door, such as Sanctuary or Hyrule Castle. Bit 15 requests the longer exit walk and is cleared before the offset is used. |

[`LoadOverworldFromUnderworld`](../jpdasm/bank_02.asm#L17348) searches this data
by the current room ID for Link's house, rooms below `$0100`, and special
overworld rooms at or above `$0180`.

Other rooms from `$0100-$017F` return using the overworld properties cached by
`LoadUnderworldEntrance` when Link entered. This lets a single-entrance room
return to the actual overworld location used to reach it without requiring a
separate exit record.

The two directions therefore resolve independently:

```text
overworld screen + Map16 offset
    -> entrance ID
    -> EntranceData
    -> underworld room and starting state

underworld room
    -> UnderworldExitData or cached entry state
    -> overworld screen and outside state
```

Changing an overworld door's entrance ID does not automatically change where
the destination room exits.

## Generated overworlds

The generated-overworld patch replaces the vanilla screen-and-offset scans
with lists local to each generated area. The formats and area-record pointers
are documented in [`asset_loading.md`](asset_loading.md#rom-storage-and-lookup)
and implemented in
[`overworld_entrances.asm`](../patches/src/overworld_entrances.asm). These lists
still produce vanilla entrance IDs; `EntranceData` and the underworld exit
handling remain the next stages of the transition.
