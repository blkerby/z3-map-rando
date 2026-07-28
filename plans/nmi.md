# NMI updates

NMI runs once per video frame. Most game code prepares data in WRAM during
the main loop, then sets one of the controls below so NMI can transfer that
data to the PPU during vertical blank.

This is primarily how the game updates VRAM while the display is active.
During loading states that call `EnableForceBlank`, main-loop code can instead
write or DMA directly to VRAM. `EraseTilemaps_*`, `InitializeTilesets`, and
`TransferFontToVRAM` are examples of direct forced-blank transfers.

The controls are separate because they serve different recurring workloads:
fixed HUD and palette buffers, compact tilemap stripe lists, one fixed-size
graphics chunk, a general DMA list, and specialized bulk-transfer handlers.

`DoNMIUpdates` processes the controls in this order:

1. Upload the normal per-frame graphics group unless `$0710` is nonzero.
2. Upload the HUD/BG3 buffer if `$16` is nonzero.
3. Upload CGRAM if `$15` is nonzero, then clear both `$15` and `$16`.
4. Upload OAM unless the one-shot mirror-transfer flag at `$0702` is set.
5. Process the stripe list selected by `$14`, then clear `$14`.
6. Perform the incremental upload selected by `$19`, then clear `$19`.
7. Process the arbitrary DMA list if `$18` is nonzero, then clear `$18` and
   `$0710`.
8. Clear `$17` and dispatch the NMI handler it selected.

These mechanisms are independent. For example, setting `$17` does not stop
the `$14` or `$18` lists from also being processed during the same NMI.

## Update controls

| Control | Meaning | Lifetime |
| --- | --- | --- |
| `$0710` | Suppress the normal per-frame graphics DMA group when nonzero | Cleared by the associated large transfer |
| `$0702` | Skip one OAM upload during a deliberately flagged mirror transfer | Cleared by NMI |
| `$16` | Upload the HUD/BG3 buffer | Cleared with `$15` by NMI |
| `$15` | Upload the complete CGRAM palette | Cleared with `$16` by NMI |
| `$14` | Process a selected stripe list | Cleared by NMI |
| `$19` | Perform one 512-byte incremental VRAM upload | Cleared by NMI |
| `$18` | Process the arbitrary DMA list at `$1100` | Cleared by NMI |
| `$17` | Dispatch one specialized NMI handler | Cleared before dispatch |

Except for the 16-bit `$0710`, these are 8-bit direct-page variables.

### `$0710`: per-frame graphics DMA suppression

At the start of `DoNMIUpdates`, zero permits a group of graphics uploads for:

- Link's head, body, hands, sword, shield, and active item
- followers, item-get graphics, push blocks, rupees, and the flute duck
- the current 1 KiB animated background-tile block

Large-transfer producers commonly store the same value in `$17` and `$0710`:

```asm
LDA.b #$04
STA.b $17       ; select NMI handler $04
STA.w $0710     ; suppress the normal per-frame graphics group
```

The value in `$0710` is not decoded. It acts as a boolean guard, while `$17`
selects the handler. The selected handler normally clears `$0710` when its
transfer finishes.

Overworld transitions, full tilemap uploads, and graphics-set changes use
this pairing to reserve NMI time for the larger transfer. Some `$1100/$18`
producers, such as the multi-frame shutter-door drawing path, also set
`$0710`. A producer must not set `$0710` without leaving a corresponding path
that clears it.

### `$16`: HUD/BG3 upload request

When `$16` is nonzero, NMI uploads `$014A` bytes from `$7EC700` to the
VRAM destination in `$0219`. It is a one-shot trigger, although regular
gameplay sets it every frame.

After the optional `$16` HUD and `$15` palette uploads, NMI enters 16-bit
accumulator mode and executes:

```asm
REP #$20
SEP #$10
STZ.b $15
```

The 16-bit store clears the word at `$15`, so both `$15` and `$16` become
zero. `HandleItemRefills` runs during the next gameplay frame, updates the
relevant parts of the HUD buffer, and increments `$16` from zero to one on
both of its exit paths. HUD rebuilds and transition paths also set it after
modifying or restoring the buffer.

The dungeon floor indicator is itself part of the HUD. After a floor change,
`HandleFloorIndicator` temporarily writes a large `1F` or `B1` into a
2-by-2-tile region at `$7EC7F2`, `$7EC7F4`, `$7EC832`, and `$7EC834`. The
same region and large-number tiles are reused by `HandleHUDTimer`. The floor
indicator is erased after about 192 frames. Its final `INC $16` requests that
NMI upload the modified HUD buffer.

### `$15`: palette upload request

When `$15` is nonzero, NMI uploads all 512 bytes at `$7EC500` to CGRAM, then
clears `$15` and `$16` together with the 16-bit store described above.
Producers generally use `INC $15`, because only zero versus nonzero matters.

Palette filters set `$15` after changing the WRAM palette during fades,
lighting changes, and boss effects. Room effects such as draining water and
presentation states such as file select also request the same full-palette
upload.

### `$14`: stripe-list selector

`$14` selects a list in the game's stripe format:

- `$14 = 1` processes the list beginning at `$1002`.
- `$14 = 2` processes the list beginning at `$1000`.
- Values `$03-$09` select fixed lists used by menus, logos, and dungeon maps.

Each stripe specifies a VRAM destination, horizontal or vertical traversal,
raw or repeated data, and a transfer length. An `$FF` byte terminates the
list. NMI processes the complete selected list and clears `$14`.

Each stripe record has this layout:

| Offset | Size | Field |
| ---: | ---: | --- |
| `+0` | 2 bytes | VRAM word address, high byte first |
| `+2` | 2 bytes | Direction, RLE flag, and 14-bit encoded length, high byte first |
| `+4` | Variable | Raw tile data, or one 2-byte value for an RLE stripe |

The control word is:

```text
DRLLLLLL LLLLLLLL

D: 0 = horizontal, 1 = vertical
R: 0 = raw data, 1 = repeat one 2-byte value
L: 14-bit encoded length
```

The encoded length is biased differently for the two forms:

| Form | Source payload | VRAM output | Total record size |
| --- | ---: | ---: | ---: |
| Raw | `L + 1` bytes | `L + 1` bytes | `L + 5` bytes |
| RLE | 2 bytes | `L + 2` bytes | 6 bytes |

For example, a raw control word with `L=3` carries four bytes of tile data,
so its record occupies eight bytes: four bytes of address/control followed
by four bytes of data. A record does not include the terminator; the `$FF` or
`$FFFF` sentinel follows the last record in the list.

For the appendable `$14=1` list, `$1000-$1001` is a 16-bit byte offset from
the start at `$1002`. A producer loads that offset, overwrites the existing
terminator with one or more stripes, advances the offset by the number of
bytes appended, and stores the new offset back to `$1000`.

For example, `DrawMap16Anywhere` follows this pattern:

```asm
LDY.w $1000          ; byte offset of the current terminator
                     ; write stripe data at $1002,Y
LDA.w #$FFFF
STA.w $1012,Y        ; write a new terminator after the data
TYA
CLC
ADC.w #$0010         ; 16 bytes were appended
STA.w $1000          ; cursor now points at the new terminator
```

The exact offsets vary with the stripe data. Some builders append several
records before writing `$FFFF`; others, including `DrawMap16Anywhere`, leave
the list terminated after every append. A producer sets `$14=1` only after
the final terminator is in place. One-shot builders can use a local index
instead of `$1000`, and `$14=2` cannot use this convention because its list
data begins at `$1000`.

NMI does not read `$1000` and does not receive a total list length. Starting
at `$1002`, `HandleStripes14` obtains each record's size from its header,
advances to the next record, and stops when the first byte there has bit 7
set. Producers normally use the word `$FFFF`, although the first `$FF` byte
is sufficient for the parser. After processing a `$14=1` list, NMI clears
both bytes of the cursor at `$1000-$1001`, then clears `$14`. The next frame
therefore starts appending at `$1002` again.

The `$1002/$14=1` form is used for small gameplay tilemap edits, including
push blocks falling into pits, cracked-floor changes, chests, and other room
objects. Fixed lists draw screens such as the intro logo, file-select menus,
name-entry screen, and dungeon map. This mechanism is useful when tile data
can be expressed compactly as horizontal or vertical stripes.

### `$19`: incremental upload request

`$19` supplies the high byte of the VRAM destination. `$0118-$0119` supplies
the source address in WRAM bank `$7F`. NMI transfers `$0200` bytes and clears
`$19`.

This mechanism divides a larger graphics load into fixed 512-byte pieces
across multiple frames.

During an overworld area transition, `RunScrollOverworldTransition` calls
`IncrementalVRAMUpload` once per frame. Sixteen calls copy the 8 KiB buffer at
`$7F0000-$7F1FFF` into OBJ character VRAM `$5000-$5FFF`. This lets the next
area's enemy and object graphics load while the camera scrolls instead of
attempting one 8 KiB active-display transfer.

### `$18`: arbitrary DMA-list request

When `$18` is nonzero, NMI processes the list at `$1100`. Each entry contains:

```text
2 bytes: VRAM destination
1 byte:  VMAIN configuration
1 byte:  transfer length in bytes
N bytes: tile data
```

A `$FFFF` word terminates the list. NMI consumes the complete list, then
clears `$18` and `$0710`. The value of `$18` itself is otherwise ignored.

There is no persistent cursor for this list. Dungeon producers use the
direct-page word `$0C-$0D` as a temporary byte offset:

1. Clear `$0C` when beginning a batch.
2. Write an entry at `$1100 + $0C`.
3. Add the entry size, `4 + transfer length`, to `$0C`.
4. Write `$FFFF` at `$1100 + $0C` and set `$18`.

`Underworld_PrepNextTilemapUpdateDMA` and
`FinalizeTranslucentOverlayStripes` normally create eight-byte transfers, so
each complete entry occupies 12 bytes. NMI does not use `$0C`; it starts at
offset zero and advances by each entry's encoded length.

This is an ownership convention, not a shared append service. Vanilla expects
one logical producer to own `$1100` for a frame, although that producer may
append several entries. Nothing arbitrates between independent producers. A
later producer that clears or rewrites the list can replace an earlier
producer's work; `$0C` is general scratch memory and cannot preserve a cursor
between them.

The VMAIN byte is written directly to the SNES `$2115` register before that
entry's DMA. It controls how the PPU advances and optionally remaps the VRAM
word address while data is written through `$2118/$2119`:

```text
v... mmii

v: increment after writing 0 = $2118, 1 = $2119
m: address-remapping mode
i: address increment in VRAM words
```

| Bits | Values |
| --- | --- |
| `i` | `00` = 1 word, `01` = 32 words, `10` or `11` = 128 words |
| `m` | `00` = none, `01` = 8-bit, `10` = 9-bit, `11` = 10-bit remapping |
| `v` | `0` = increment after `$2118`; `1` = increment after `$2119` |

The `$18` handler uses DMA mode 1, which alternates writes between `$2118`
and `$2119`: the low and high bytes of each tilemap word. Setting `v=1`
therefore keeps both bytes at the same VRAM address, then advances after the
complete word.

The common values for tilemap transfers are:

| VMAIN | Effect |
| ---: | --- |
| `$80` | Advance by 1 word after each tile: a horizontal row |
| `$81` | Advance by 32 words after each tile: a vertical column in a 32-tile-wide screen block |

The remapping modes rearrange low VRAM address bits:

```text
m=01: aaaaaaaabbbccccc -> aaaaaaaacccccbbb
m=10: aaaaaaabbbcccccc  -> aaaaaaaccccccbbb
m=11: aaaaaabbbccccccc   -> aaaaaacccccccbbb
```

They support graphics layouts where sequential source data should be
distributed across tile-shaped VRAM addresses. Normal row and column
tilemap entries use `m=00`.

All vanilla `$18` producers are in the underworld module (`$10 = $07`):

| Producer | When it owns `$1100` |
| --- | --- |
| `RoomTag_WaterOff` | Submodule `$00`, when room tag `$18` starts draining a water room. |
| `DontOpenDoor` | Submodule `$00`, when an eye-watch door changes state. |
| `SlashSwordAgainstVinesAndDoors` | Submodule `$00`, when Link's sword reaches vines or a door; curtain doors can change state. |
| `ClearAwayExplodingWall` | Submodule `$00`, on each drawing step of a bombable-wall animation while `$0454` is nonzero. |
| `OpenBigChest` | Submodule `$00`, on the frame Link opens a big chest. |
| `LightTorch` / `ExtinguishTorch` | Submodule `$00`, when a dungeon torch changes state. Expired torch timers are checked by `ProcessTorchesAndDoors`. |
| `ApplyRoomHolesOverlay` | Submodule `$03`, while applying a room overlay change. |
| `UnlockKeyDoor` | Submodule `$04`, on the drawing steps of the key-door animation. |
| `OperateShutterDoors` | Submodule `$05`, on the drawing steps of a shutter animation. It is also called from submodule `$01`, sub-submodule `$02`, while resetting shutters during an intraroom transition. |
| `OpenCrackedDoor` | Submodule `$09`, on the drawing steps of the cracked-door animation. |
| `MakeNearbyWallsHighPriority_Entering` / `MakeNearbyWallsLowPriority` | Submodule `$0E`, at the beginning and end of a spiral-stairs transition. Both finish through `RoomDraw_CloseStripes`. |
| `OpenTriforceDoor` | Submodule `$1A`, every fourth frame while the Triforce door animation advances. |

The module/submodule dispatcher and event sequencing normally keep these
owners separate. This is not an enforced guarantee: for example,
`ProcessTorchesAndDoors` can call `ExtinguishTorch` more than once if several
timers expire together, and each call rebuilds the list. New code must
therefore arrange exclusive ownership rather than treating `$0C` as a global
append cursor.

No vanilla overworld routine sets `$18`. Vanilla overworld streaming instead
uses `$1100` with the incompatible `$17 = $03` handler. The milestone 1 BG2
patch disables those overworld producers before the new streamer takes
ownership of `$1100`; it does not remove the shared NMI handler itself.

### `$17`: specialized handler selector

NMI doubles `$17` and uses it as an index into a table of 16-bit handler
addresses. It clears `$17` before jumping to the selected handler.

| `$17` | Handler | Typical context |
| ---: | --- | --- |
| `$00` | No tile update | Normal frame with no specialized request |
| `$01` | Upload tilemap | HUD, item menu, and bottle-menu redraws |
| `$02` | Upload BG3 text | Message and text screens |
| `$03` | Update overworld scrolling tilemap data | Vanilla overworld area transitions |
| `$04` | Update subscreen overlay | Large overlay or tilemap replacement |
| `$05` | Update BG1 wall | Dungeon wall changes |
| `$06` | Unused in vanilla | BG3 row streaming in this project |
| `$07` | Load light-world map graphics | World-map screen |
| `$08` | Update left half of BG2 | Large BG2 replacement |
| `$09` | Update background character sets 3 and 4 | Graphics-set transition |
| `$0A` | Update background character sets 5 and 6 | Graphics-set transition |
| `$0B` | Update half of a background character set | Smaller graphics replacement |
| `$0C` | Upload the latter half of the subscreen overlay | Split overlay load |
| `$0D` | Upload the former half of the subscreen overlay | Split overlay load |
| `$0E-$11` | Update background character sets 0 through 3 | Background graphics replacement |
| `$12` | Update object character set 0 | Sprite graphics replacement |
| `$13` | Update object character set 2 | Sprite graphics replacement |
| `$14` | Update object character set 3 | Sprite graphics replacement |
| `$15` | Upload dark-world map | World-map screen |
| `$16` | Upload game-over text | Game-over screen |
| `$17` | Update peg tiles | Hammered peg animation |
| `$18` | Update star tiles | Dungeon star-switch animation |

Most handlers which perform a large transfer clear `$0710` before returning.
Unlike `$14` and `$18`, `$17` does not describe a general list format. Each
mode has fixed source, destination, size, or parameter conventions implemented
by its handler.

## Related NMI state

`$12` is the main-loop/NMI synchronization handshake. NMI performs the normal
updates only when `$12` is clear, then increments it. This prevents NMI from
consuming buffers while the main loop is still preparing them and lets the
main loop wait for the prepared frame to be consumed.

`$13` is not an update request. It is the queued value for `INIDISP`, which
controls screen brightness and forced blank. NMI temporarily writes `$80` to
`INIDISP` while preparing PPU writes, then restores the value in `$13` near
the end.

The scroll, window, color-math, screen-selection, and other PPU queue
variables are also copied during NMI, but they are current register values
rather than queued transfer requests.
