; BG2 streamer. This reduces the overworld BG2 tilemap from 64x64 to 64x32,
; by implementing a new streaming renderer that applies for all areas, small
; and large. It differs from the vanilla streaming renderer (used by vanilla
; in large areas) in that it draws new rows/columns in units of 8x8 tiles
; rather than 16x16, and only draws them across the visible area:
; rows of 33 tiles and columns of 29 tiles, compared to vanilla
; which would draw a 64x2 block (128 tiles). However, unlike vanilla,
; when both a row and column update are needed on the same frame,
; we don't try to spread them out onto different frames. It's ok because the
; updates are so much smaller.
;
; The fact that we draw only the visible region with no margin creates
; some complication with the mirror warp, where the HDMA wave effects could
; bring tiles into view outside the drawn window. This is handled by drawing
; margin columns when using the mirror. We also fix some vanilla issues
; with the mirror: vanilla has NMI overruns causing flashes of black
; bands at the top of the screen while using the mirror. It also
; begins reloading graphics while the palette is not yet fully faded to white;
; it can get away with this because the Dark World and Light World tilesets
; are aligned with each other, something we don't assume for custom tilesets.
; So we delay all reloading to happen under the cover of the fully white
; screen. Because of the elimination of the Map32 system (with its slow
; decompression step) this doesn't result in an overall slower sequence;
; in fact it's a bit faster than vanilla.

lorom

!BG2BulkFreeStart = $1BB1E0
!BG2BulkFreeEnd = $1BB800

!NMISkipOAM = $0702

!Map16TopLeft = $288000
!Map16TopRight = $298000
!Map16BottomLeft = $2A8000
!Map16BottomRight = $2B8000

; Module09_LoadNewMapAndGFX
;
; Vanilla contexts: modules $09/$0B, submodules $03 and $11, while preparing
; the destination map for an area transition.
;
; Vanilla increments $0710 to suppress Link graphics DMA until the initial
; mode $03 BG2 stripe upload clears it. That upload is disabled below, so
; leave $0710 clear or Link's displayed graphics stop updating permanently.
org $02AACE
    NOP
    NOP
    NOP

; Module09_21
;
; Vanilla context: modules $09/$0B, submodule $21, after submodule $20
; restores the BG1 overlay on return from the world map.
;
; This forced-blank reload stage follows the BG1 overlay upload in submodule
; $20. Replace its obsolete small/large-area BG2 builder with the bulk
; renderer, then advance to Module09_22 to fade the screen back in.
org $02EA89
    JSL BG2BulkRender
    SEP #$20
    INC.b $11
    STZ.b $13
    RTS

; MirrorWarp_LoadSpritesAndColors
;
; Vanilla contexts: modules $09/$0B, submodules $23 and $2C, with $B0=$04
; while completing a mirror warp.
;
; The first half of this routine saves and adjusts the vanilla BG2 renderer
; cursors, calls BuildOverworldFromMap16, and restores the cursors. Skip
; directly to the sprite-palette work that follows.
org $02B283
    JMP.w $B2CE

; Module09_2E_Whirlpool
;
; Our bulk renderer makes whirlpool state 5 queue the destination BG2 window
; through $18. Vanilla state 5 also queues BG1 overlay half $0C through $17,
; which would make both transfers run in the same NMI. Split them across two
; solid-blue frames. The HUD remains enabled throughout.
;
; States 3, 4, and 6 also queue an overlay half. They are smaller than the
; combined state-5 transfer, but still benefit from deliberately omitting
; one OAM upload.
org $02B3C7
    JSL BG2WhirlpoolQueueOverlayLatter
    BRA BG2WhirlpoolBlueRemoval
    NOP
    NOP

org $02B3CF
    JSL BG2WhirlpoolQueueOverlayFormer
    RTS
    NOP

org $02B3D5
    JSL BG2WhirlpoolLoadDestination
    RTS
    NOP
    NOP
    NOP
    NOP
    NOP

; Branch target used after the state-3 replacement above.
org $02B426
BG2WhirlpoolBlueRemoval:

; MirrorWarp_Initialize
;
; Before changing $8A to the destination world, fill the three source-world
; columns that the horizontal-scroll HDMA can expose outside the normal
; 33-tile window.
org $02B158
    JSL BG2MirrorInitialize

; MirrorWarp_BuildWavingHDMATable
;
; Keep HDMA off on mirror frames that queue a large NMI transfer. The screen
; is white during these loading steps. While the destination tilemap is being
; staged, keep it off until the final margin repair has also been transferred.
org $00FE64
    JML BG2MirrorRunAnimation

; AnimateMirrorWarp_DrawDestinationScreen
;
; Unlike the ordinary full-load paths, this calls
; DrawOverworldQuadrantsAndOverlays as a subroutine and returns without
; falling through to OverworldBuildMapAndTrigger. Replace its final
; "INC $0710 : RTL" with the common bulk renderer, which sets $0710 itself.
org $00D8F7
    JML BG2MirrorBulkRender

; AnimateMirrorWarp_DoSpritesPalettes
;
; Step 8's original tail occupies the bytes reused below as the step-10
; trampoline. Move that tail to free space without changing its behavior.
org $00D8FF
    JML BG2MirrorTriggerOverlayA

; This four-byte tail is unreachable after the JML above. Reuse it as a
; bank-00 trampoline for the step-10 mirror margins.
org $00D903
    JML BG2MirrorRenderMargins

; AnimateMirrorWarp vector entry 10
;
; The common bulk renderer already installs the complete destination tilemap
; during step 7. Leave step 9 unchanged and use step 10 only to add margins
; after its normal animated-tile decompression.
org $00D880
    db $07, $03

; OverworldBuildMapAndTrigger
;
; This is the common full-overworld BG2 build path. Its vanilla entry paths
; are:
; - $08/$0A:$02:
;     Module08_02_LoadAndAdvance -> LoadAndBuildOverworldScreen
;     -> DrawOverworldQuadrantsAndOverlays -> this routine.
; - $09/$0B:$19:
;     Module09_19 -> Module08_02_LoadAndAdvance
;     -> LoadAndBuildOverworldScreen -> DrawOverworldQuadrantsAndOverlays
;     -> this routine.
; - $09/$0B:$21:
;     Module09_21 -> this routine directly.
; - $09/$0B:$28:
;     overworld submodule table -> LoadAndBuildOverworldScreen
;     -> DrawOverworldQuadrantsAndOverlays -> this routine.
; - $09/$0B:$2E with $B0=$05:
;     Module09_2E_05_LoadDestinationMap -> Overworld_LoadOverlayAndMap
;     -> LoadAndBuildOverworldScreen_long -> LoadAndBuildOverworldScreen
;     -> DrawOverworldQuadrantsAndOverlays -> this routine.
; - $0E:$0A with $0200=$08:
;     Module0E_0A_FluteMenu -> Overworld_LoadOverlayAndMap
;     -> the same LoadAndBuildOverworldScreen path.
; - Top-level $18 with $0200=$04:
;     Module18_04 -> Overworld_LoadOverlayAndMap
;     -> the same LoadAndBuildOverworldScreen path.
; - Top-level $19 with $B0=$04:
;     Module19_04_LoadAndSongAndAdvance -> Module08_02_LoadAndAdvance
;     -> the same LoadAndBuildOverworldScreen path.
; - Top-level $1A credits, with $11=$00,$04,$06,$08,$0A,$0C,$0E,$10,$12,
;   $18,$1A,$1C, or $1E and $B0=$02:
;     Credits_LoadNextScene_Overworld -> Credits_LoadOverworldScene
;     -> Credits_LoadOverworldScene_LoadMap -> LoadAndBuildOverworldScreen
;     -> DrawOverworldQuadrantsAndOverlays -> this routine.
;
; Except for $09/$0B:$21, these paths fall through from
; DrawOverworldQuadrantsAndOverlays; they do not call this label explicitly.
;
; Skip the BG2-only cursor setup and BuildOverworldFromMap16 call. Resume at
; the existing stack-restoration tail, then replace its BG2 mode $04 request
; with the new bulk renderer. LoadOverworldOverlay separately builds BG1 and
; retains its own request.
org $02EAF4
    BRA +
org $02EB04
+

org $02EB11
    JSL BG2BulkRender
    NOP
    NOP
    NOP

; LoadUnderworldEntrance
;
; Full underworld loads assume BG2SC still has vanilla value $03. The
; overworld bulk renderer changes it to $01, so restore the 64x64 dungeon
; layout at the shared underworld entrance loader. The replacement routine
; also performs the overwritten "LDA #$01 : STA $1B".
org $02D61A
    JSL BG2RestoreUnderworldLayout

; CreateInitialNewScreenMapToScroll
;
; Vanilla contexts: modules $09/$0B, submodules $03 and $11, after the
; destination area's logical Map16 data has been loaded.
;
; This routine and its direction-specific callees only initialize the
; vanilla BG2 streaming cursors and build the first mode $03 stripe batch.
; Replace it with the new streamer's destination-edge preload (only needed
; for eastward transitions).
org $02ED95
    JSL BG2SeedTransitionEdge
    RTS

; OverworldTransitionScrollAndLoadMap
;
; Vanilla contexts: modules $09/$0B during scrolling area transitions,
; including the final easing frames.
;
; The caller owns transition control and clears $0416 after this call. This
; routine's only contribution is another vanilla BG2 mode $03 stripe batch.
org $02EF72
    SEP #$30
    RTS

; OverworldHandleMapScroll
;
; Vanilla contexts: ordinary overworld camera movement and several scripted
; overworld movement sequences.
;
; This routine only consumes $0416 to build vanilla BG2 stripes and uses
; $0418 to serialize diagonal stripes across frames. Skip the whole consumer.
; Do not clear either variable here: $0418 also records the direction of
; transitions for movement code.
org $02EFD7
    RTS

; OverworldCameraBoundaryCheck
;
; Ordinary overworld camera movement records every crossed 16-pixel boundary
; in $0416 solely to request work from OverworldHandleMapScroll. The new
; streamer compares the finalized 8-pixel scroll positions instead, so skip
; this obsolete producer.
org $02BCE4
    BRA +
org $02BCED
+

; Module09_Overworld / Module0B_OverworldSpecial
;
; These modules add the current shake offset to BG2HOFS and BG2VOFS after
; their submodule has run. Each replacement routine compares and stores its
; own finalized axis. The horizontal routine starts the $1100 list and
; returns its cursor in Y; the vertical routine receives Y, appends its rows,
; and finishes the shared list.
;
; The first five replaced bytes are:
;   STA $E2
;   STA $011E
org $02A37D
    JSL BGStreamHorizontal
    NOP
;
; The second five replaced bytes are:
;   STA $E8
;   STA $0122
org $02A389
    JSL BGStreamVertical
    NOP

; DrawMap16Anywhere and AlterMap16Hardcore append immediate Map16 changes to
; the existing $1000/$14 stripe list. Keep those producers, but replace their
; duplicated vanilla 64x64 destination calculation with the 64x32 BG2 ring.
;
; AlterMap16Hardcore has already saved A, written the logical Map16 value, and
; pushed X. Its stripe emitter begins at $1BCA23 after this replacement.
org $1BC9E9
    STX.b $00
    JSR.w BG2FindMap16VRAMAddress
    BRA +
org $1BCA23
+

; Input: $00 = byte offset of the Map16 tile in the 64x64 WRAM map.
; Output: $02 = top-left BG2 VRAM word in the 64x32 tilemap.
;
; Each Map16 tile is two 8x8 tiles wide and high. Map16 column bit 4 selects
; the second 32x32 screen block; Map16 row bits 0-3 wrap through its 16
; physical Map16 rows.
org $1BCA69
BG2FindMap16VRAMAddress:
    LDA.b $00
    AND.w #$0020
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    STA.b $02                   ; $0000 or right-hand block $0400.

    LDA.b $00
    AND.w #$001F
    ORA.b $02
    STA.b $02                   ; Even 8x8 X within the screen block.

    LDA.b $00
    AND.w #$0780
    LSR A
    ORA.b $02
    STA.b $02                   ; Physical row * 32 words.
    RTS

assert pc() <= $1BCA9F

;---------------------------------------------------------------------------------------------------
; Build the complete visible BG2 window as an arbitrary DMA list at $1100.
;
; The logical window starts at the 8x8 tile containing the viewport's
; top-left pixel and is always 33 tiles wide by 29 tiles high. BG2 is a
; 64x32 tilemap based at VRAM word $0000.
;
; Each 33-tile row crosses exactly one discontinuity in the two horizontal
; 32x32 screen blocks. Emit two $18 entries per row, writing tile words
; directly into their final positions in $1100-$1963:
;
;   29 * (66 data bytes + 8 header bytes) + 2-byte terminator = 2148 bytes
;
; Direct-page scratch:
;   $00: logical X coordinate of the window's left edge (in units of 8x8 tiles from area left)
;   $02: logical Y coordinate of the row being emitted (in units of 8x8 tiles from area top)
;   $04: bulk rows remaining, or runtime entering-edge offset
;   $06: logical X coordinate of the tile being emitted
;   $08: tiles remaining in the current segment
;   $0A: address scratch, or previous finalized tile before comparison
;   $0C: segment scratch
;   $0E: column destination X, signed tile delta, or runtime rows remaining
;
; Y is the byte offset into the $1100 list.
;---------------------------------------------------------------------------------------------------

org !BG2BulkFreeStart
BG2BulkRender:
    PHP                         ; Preserve the caller's register-width flags.

    REP #$30                    ; Use 16-bit A, X, and Y for offsets and words.

    PHA                         ; The load-state tail does not expect us to
    PHX                         ; change any registers, so save all three.
    PHY

    ; Select a 64x32 BG2 tilemap at VRAM word $0000. The common full-load
    ; paths are still force-blanked when they reach this routine.
    SEP #$20                    ; PPU registers are byte-wide.

    LDA.b #$01                  ; base=$0000, width=64, height=32
    STA.w $2108                 ; BG2SC

    REP #$20                    ; Return to 16-bit coordinate arithmetic.

    JSR BG2CalculateLogicalWindowOrigin
    ; $00 = logical X tile,  $02 = logical Y tile

    LDA.w #$001D                ; Initialize remaining rows to 29
    STA.b $04

    JSR BG2BuildRowList

    PLY                         ; Restore registers in reverse push order.
    PLX
    PLA

    PLP                         ; Restore the caller's original widths.
    RTL

; Build and request a $1100/$18 list containing $04 rows beginning at $02.
BG2BuildRowList:
    LDY.w #$0000

.next_row
    JSR BG2EmitRow

    LDA.b $02
    INC A
    AND.w #$007F
    STA.b $02

    DEC.b $04
    BNE .next_row

    LDA.w #$FFFF
    STA.w $1100,Y

    ; Reserve NMI time for this tilemap transfer. The $18 handler clears both
    ; controls after consuming the complete list.
    LDA.w #$0001
    STA.w $0710

    SEP #$20
    LDA.b #$01
    STA.b $18
    REP #$20

    RTS

;---------------------------------------------------------------------------------------------------
; Preload the offscreen destination edge for east transitions.
;
; The resident window includes one extra column on the right and one extra
; row on the bottom. Loading an eastern destination invalidates the extra
; column without moving the camera, so refresh it before the first 8-pixel
; scroll step. South needs no seed: its $x11E->$x120 alignment correction
; crosses a tile boundary and invokes the normal vertical streamer.
;---------------------------------------------------------------------------------------------------

BG2SeedTransitionEdge:
    PHP
    REP #$30

    PHA
    PHX
    PHY

    LDA.w $0410                 ; Transition direction bit.
    AND.w #$00FF
    CMP.w #$0001                ; East.
    BNE .done                   ; Other directions expose or stream their edge.

    JSR BG2CalculateLogicalWindowOrigin

    LDA.b $00
    CLC
    ADC.w #$0020                ; Preload the current offscreen right column.
    AND.w #$007F
    STA.b $06
    STA.b $0E                   ; Source and destination X are the same.

    LDY.w #$0000
    JSR BG2EmitColumn

    LDA.w #$FFFF
    STA.w $1100,Y

    SEP #$20

    LDA.b #$01
    STA.b $18                   ; Transfer the seed edge before scrolling starts.

    REP #$20

.done
    PLY
    PLX
    PLA

    PLP
    RTL

;---------------------------------------------------------------------------------------------------
; Store finalized horizontal scroll and append an entering column.
;
; Input:
;   A: new BG2 horizontal scroll in pixels, including shake
;
; Output:
;   Y: byte offset after the optional column in the $1100 list
;
; The column uses the previous finalized vertical tile. If vertical movement
; exposed corner tiles, BGStreamVertical appends their rows afterward and
; overwrites those intersections with the same final data.
;---------------------------------------------------------------------------------------------------

BGStreamHorizontal:
    STA.b $E2                   ; Preserve the first overwritten vanilla store.

    PHP                         ; The caller has 16-bit A but 8-bit X/Y.
    REP #$30

    PHA                         ; Preserve registers other than the Y result.
    PHX

    LDY.w #$0000                ; Start this frame's gameplay list at $1100.

    LDA.w $011E
    LSR A                       ; Convert the old scroll to its 8x8 tile column.
    LSR A
    LSR A
    STA.b $0A

    LDA.b $E2
    STA.w $011E                 ; Preserve the second overwritten vanilla store.
    LSR A
    LSR A
    LSR A
    SEC
    SBC.b $0A                   ; Signed horizontal tile delta.
    STA.b $0E
    BEQ .done                   ; Most frames expose no horizontal edge.

    ; The bulk renderer may already own $1100 this frame.
    LDA.b $18
    AND.w #$00FF
    BNE .done

    LDA.b $0E
    CMP.w #$0001
    BEQ .moving_right

    CMP.w #$FFFF
    BNE .done                   ; No boundary crossed, or this was a larger jump.

    LDA.w #$0000                ; Moving left exposes the new left edge.
    BRA .save_column_offset

.moving_right
    LDA.w #$0020                ; Moving right exposes column left+32.

.save_column_offset
    STA.b $04

    JSR BG2CalculateLogicalWindowOrigin

    LDA.b $00
    CLC
    ADC.b $04
    AND.w #$007F
    STA.b $06                   ; Logical X of the entering column.
    STA.b $0E                   ; Source and destination X are the same.

    JSR BG2EmitColumn           ; Return its ending list offset in Y.

.done
    PLX
    PLA

    PLP
    RTL

;---------------------------------------------------------------------------------------------------
; Store finalized vertical scroll, append entering rows, and finish the list.
;
; Input:
;   A: new BG2 vertical scroll in pixels, including shake
;   Y: byte offset after any column emitted by BGStreamHorizontal
;---------------------------------------------------------------------------------------------------

BGStreamVertical:
    STA.b $E8                   ; Preserve the first overwritten vanilla store.

    PHP                         ; The caller has 16-bit A but 8-bit X/Y.
    REP #$30                    ; Use 16-bit coordinates, offsets, and tiles.

    PHA                         ; Preserve registers other than the Y cursor.
    PHX

    LDA.w $0122                 ; Convert the previous finalized Y scroll to its
    LSR A                       ; world-space 8x8 tile row.
    LSR A
    LSR A
    STA.b $0A

    LDA.b $E8
    STA.w $0122                 ; Preserve the second overwritten vanilla Y store.
    LSR A                       ; Convert the new scroll to its tile row.
    LSR A
    LSR A
    SEC
    SBC.b $0A                   ; Signed vertical tile delta.
    STA.b $0E
    BEQ .finish_list            ; Finish any horizontal work and return.

    ; The bulk renderer may already have prepared $1100 earlier this frame.
    ; Keep its complete-window upload instead of replacing it with edges.
    LDA.b $18
    AND.w #$00FF
    BNE .done

    ; Keep only the vertical deltas supported by gameplay. A normal frame
    ; exposes one row; the final north-transition frame can expose two above.
    LDA.b $0E
    CMP.w #$0001
    BEQ .vertical_delta_ready

    CMP.w #$FFFF
    BEQ .vertical_delta_ready

    CMP.w #$FFFE
    BEQ .vertical_delta_ready

    BRA .finish_list            ; Ignore larger jumps; bulk loading owns those.

.vertical_delta_ready
    JSR BG2CalculateLogicalWindowOrigin

    LDA.b $0E
    BMI .moving_up

    ; The supported downward delta is one row, exposing top+28.
    LDA.b $02
    CLC
    ADC.w #$001C
    AND.w #$007F
    STA.b $02
    BRA .next_row

.moving_up
    EOR.w #$FFFF                ; Absolute value of -1 or -2.
    INC A
    STA.b $0E                   ; Start at top and emit one or two rows.

.next_row
    JSR BG2EmitRow

    DEC.b $0E
    BEQ .finish_list

    LDA.b $02                   ; The second exposed row follows the first.
    INC A
    AND.w #$007F
    STA.b $02
    BRA .next_row

.finish_list
    CPY.w #$0000                ; Horizontal and vertical both had no work.
    BEQ .done

    LDA.w #$FFFF                ; Terminate after the last column or row entry.
    STA.w $1100,Y

    SEP #$20                    ; $18 is a byte flag.

    LDA.b #$01
    STA.b $18                   ; Ask NMI to transfer the completed edge list.

    REP #$20                    ; Restore 16-bit A for saved registers.

.done
    PLX
    PLA

    PLP                         ; Restore the caller's original register widths.
    RTL

;---------------------------------------------------------------------------------------------------
; Convert the finalized BG2 scroll to the logical tile at the window's
; top-left corner.
;
; Output:
;   $00: logical X coordinate, 0..127
;   $02: logical Y coordinate, 0..127
;
; The flat-map loader places screen $8A at logical Map16 coordinate (0,0),
; regardless of that screen's world position. Subtract its world-space
; origin before dividing the scroll coordinates by eight. Large areas use a
; top-left screen ID and naturally continue into the other three quadrants.
;---------------------------------------------------------------------------------------------------

BG2CalculateLogicalWindowOrigin:
    LDA.b $8A                   ; Current top-left overworld screen ID.
    AND.w #$0007                ; Screen-grid column, 0..7.
    XBA                         ; Multiply by $100.
    ASL A                       ; Multiply by $200 pixels per screen.
    STA.b $0A                   ; World-space X origin of loaded screen.

    LDA.w $011E                 ; Final BG2 horizontal scroll in pixels.
    SEC
    SBC.b $0A                   ; Make it relative to logical map column 0.
    LSR A                       ; Divide by 8 pixels per 8x8 tile.
    LSR A
    LSR A
    AND.w #$007F                ; Wrap within the 128-tile logical width.
    STA.b $00

    ; Each overworld screen-grid row is $200 pixels tall. Bits 3-5 of $8A
    ; select that row; world and special-area bits do not affect its origin.
    LDA.b $8A
    AND.w #$0038                ; Screen-grid row already multiplied by 8.
    ASL A                       ; Multiply by another $40:
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A                       ; row * 8 * $40 = row * $200 pixels.
    STA.b $0A                   ; World-space Y origin of loaded screen.

    LDA.w $0122                 ; Final BG2 vertical scroll in pixels.
    SEC
    SBC.b $0A                   ; Make it relative to logical map row 0.
    LSR A                       ; Divide by 8 pixels per 8x8 tile.
    LSR A
    LSR A
    AND.w #$007F                ; Wrap within the 128-tile logical height.
    STA.b $02

    RTS

;---------------------------------------------------------------------------------------------------
; Append one 33-tile horizontal row.
;
; $00: first logical X coordinate
; $02: logical Y coordinate
; Y:   current $1100 list offset
;
; A 33-tile row always crosses exactly one boundary between the two physical
; 32x32 screen blocks, so this appends two horizontal entries.
;---------------------------------------------------------------------------------------------------

BG2EmitRow:
    LDA.b $00                   ; Every row begins at the window's logical X.
    STA.b $06                   ; Initialize the tile X currently being emitted.

    ; If physical X within its 32-tile block is r, the second segment holds
    ; r+1 tiles and the first holds the remaining 33-(r+1).
    AND.w #$001F                ; r = X within the current screen block.
    INC A
    STA.b $0C                   ; Tiles in the second list entry.

    LDA.w #$0021                ; Entire row is 33 tiles.
    SEC
    SBC.b $0C
    STA.b $08                   ; Tiles through the first block boundary.

    JSR BG2EmitHorizontalSegment

    LDA.b $0C                   ; Continue at the X advanced by the first entry.
    STA.b $08

    JSR BG2EmitHorizontalSegment
    RTS

;---------------------------------------------------------------------------------------------------
; Emit one horizontal $18 entry.
;
; $06: first logical X coordinate
; $02: logical Y coordinate
; $08: number of tiles
; Y:   current $1100 list offset
;
; Advances $06 and Y past the emitted tiles.
;---------------------------------------------------------------------------------------------------

BG2EmitHorizontalSegment:
    LDX.b $06                   ; Horizontal source and destination X match.
    JSR BG2CalculateVRAMAddress
    STA.w $1100,Y               ; Entry bytes 0-1: VRAM word destination.

    INY                         ; Advance to the VMAIN/length fields.
    INY

    ; Low byte: VMAIN $80, advancing one VRAM word after each tile.
    ; High byte: transfer length in bytes.
    LDA.b $08                   ; Segment length in tile words.
    ASL A                       ; Convert words to DMA bytes.
    XBA                         ; Put byte count in entry byte 3.
    ORA.w #$0080                ; Put VMAIN $80 in entry byte 2.
    STA.w $1100,Y

    INY                         ; Advance to this entry's tile payload.
    INY

    ; Calculate the first Map16 byte offset once. Every pair advances to the
    ; next Map16 ID, avoiding a coordinate calculation per tile.
    LDA.b $02
    AND.w #$007E
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A                       ; Map16 row * $80 bytes.
    STA.b $0A

    LDA.b $06
    AND.w #$007E                ; Map16 column * 2 bytes.
    CLC
    ADC.b $0A
    TAX                         ; X is the current Map16 map offset.

    LDA.b $08
    STA.b $0A                   ; Preserve the segment length for final X advance.

    LDA.b $02                   ; Select top or bottom tables once per segment.
    AND.w #$0001
    BNE .bottom

    ; If the segment starts on an odd X, emit that initial top-right tile
    ; alone so every complete pair that follows starts on a left quadrant.
    LDA.b $06
    AND.w #$0001
    BEQ .top_pairs

    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16TopRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    INX
    INX                         ; Advance to the next Map16 ID.

    DEC.b $08
    BNE .top_pairs
    JMP .done

.top_pairs
    ; After an optional initial right tile, X is even. Save whether one
    ; trailing left tile remains, then count complete left/right pairs.
    LDA.b $08
    LSR A
    STA.b $08
    PHP

    LDA.b $08
    BEQ .top_pairs_done

.top_pair_loop
    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16TopLeft,X
    STA.w $1100,Y

    INY
    INY

    LDA.l !Map16TopRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    INX
    INX

    DEC.b $08
    BNE .top_pair_loop

.top_pairs_done
    PLP
    BCC .done                   ; Even remainder has no trailing tile.

    LDA.l $7E2000,X
    ASL A
    TAX

    LDA.l !Map16TopLeft,X
    STA.w $1100,Y

    INY
    INY
    BRA .done

.bottom
    ; Bottom rows follow the same right-then-pairs pattern using the bottom
    ; definition tables.
    LDA.b $06
    AND.w #$0001
    BEQ .bottom_pairs

    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16BottomRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    INX
    INX

    DEC.b $08
    BEQ .done

.bottom_pairs
    LDA.b $08
    LSR A
    STA.b $08
    PHP                         ; Carry records a trailing bottom-left tile.

    LDA.b $08
    BEQ .bottom_pairs_done

.bottom_pair_loop
    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16BottomLeft,X
    STA.w $1100,Y

    INY
    INY

    LDA.l !Map16BottomRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    INX
    INX

    DEC.b $08
    BNE .bottom_pair_loop

.bottom_pairs_done
    PLP
    BCC .done

    LDA.l $7E2000,X
    ASL A
    TAX

    LDA.l !Map16BottomLeft,X
    STA.w $1100,Y

    INY
    INY

.done
    LDA.b $06                   ; Advance logical X by the original length.
    CLC
    ADC.b $0A
    AND.w #$007F
    STA.b $06
    RTS

;---------------------------------------------------------------------------------------------------
; Append one 29-tile vertical column.
;
; $06: logical X coordinate
; $02: first logical Y coordinate
; Y:   current $1100 list offset
;
; VMAIN $81 advances by one physical 32-tile row. If the column crosses
; physical row 31, split it so the second entry wraps to row 0 in the same
; horizontal screen block. Restores $02 before returning.
;---------------------------------------------------------------------------------------------------

BG2EmitColumn:
    LDA.b $02
    PHA                         ; Preserve the window's logical top row.

    AND.w #$001F                ; Physical row of the column's first tile.
    STA.b $0A

    LDA.w #$0020
    SEC
    SBC.b $0A                   ; Tiles available through physical row 31.
    CMP.w #$001D
    BCC .save_first_length

    LDA.w #$001D                ; The whole column fits before the wrap.

.save_first_length
    STA.b $08

    LDA.w #$001D                ; Save any tiles remaining after the wrap.
    SEC
    SBC.b $08
    STA.b $0C

    JSR BG2EmitVerticalSegment

    LDA.b $0C
    BEQ .done

    STA.b $08
    JSR BG2EmitVerticalSegment

.done
    PLA
    STA.b $02                   ; Restore the logical top for row emission.
    RTS

;---------------------------------------------------------------------------------------------------
; Emit one vertical $18 entry.
;
; $06: logical source X coordinate
; $0E: physical destination X coordinate
; $02: first logical Y coordinate
; $08: number of tiles
; Y:   current $1100 list offset
;
; Advances $02 and Y past the emitted tiles.
;---------------------------------------------------------------------------------------------------

BG2EmitVerticalSegment:
    LDX.b $0E                   ; Column destination may differ from source X.
    JSR BG2CalculateVRAMAddress
    STA.w $1100,Y               ; Entry bytes 0-1: VRAM word destination.

    INY
    INY

    ; VMAIN $81 advances 32 VRAM words after each complete tile word.
    LDA.b $08
    ASL A                       ; Convert tile words to DMA bytes.
    XBA                         ; Put byte count in entry byte 3.
    ORA.w #$0081                ; Put VMAIN $81 in entry byte 2.
    STA.w $1100,Y

    INY
    INY                         ; Advance to this entry's tile payload.

    ; Calculate the first Map16 byte offset once. Every pair advances one
    ; Map16 row, avoiding a coordinate calculation and ID load per tile.
    LDA.b $02
    AND.w #$007E
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A                       ; Map16 row * $80 bytes.
    STA.b $0A

    LDA.b $06
    AND.w #$007E                ; Map16 column * 2 bytes.
    CLC
    ADC.b $0A
    TAX                         ; X is the current Map16 map offset.

    LDA.b $08
    STA.b $0A                   ; Preserve the segment length for final Y advance.

    LDA.b $06                   ; Select left or right tables once per segment.
    AND.w #$0001
    BNE .right

    ; If the segment starts on an odd Y, emit that initial bottom-left tile
    ; alone so every complete pair that follows starts on a top quadrant.
    LDA.b $02
    AND.w #$0001
    BEQ .left_pairs

    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16BottomLeft,X
    STA.w $1100,Y

    INY
    INY

    PLX
    TXA
    CLC
    ADC.w #$0080
    TAX                         ; Advance to the next Map16 row.

    DEC.b $08
    BNE .left_pairs
    JMP .done

.left_pairs
    ; Save whether one trailing top tile remains, then count complete
    ; top/bottom pairs.
    LDA.b $08
    LSR A
    STA.b $08
    PHP

    LDA.b $08
    BEQ .left_pairs_done

.left_pair_loop
    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16TopLeft,X
    STA.w $1100,Y

    INY
    INY

    LDA.l !Map16BottomLeft,X
    STA.w $1100,Y

    INY
    INY

    PLX
    TXA
    CLC
    ADC.w #$0080
    TAX

    DEC.b $08
    BNE .left_pair_loop

.left_pairs_done
    PLP
    BCC .done

    LDA.l $7E2000,X
    ASL A
    TAX

    LDA.l !Map16TopLeft,X
    STA.w $1100,Y

    INY
    INY
    BRA .done

.right
    ; Right columns follow the same bottom-then-pairs pattern using the right
    ; definition tables.
    LDA.b $02
    AND.w #$0001
    BEQ .right_pairs

    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16BottomRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    TXA
    CLC
    ADC.w #$0080
    TAX

    DEC.b $08
    BEQ .done

.right_pairs
    LDA.b $08
    LSR A
    STA.b $08
    PHP                         ; Carry records a trailing top-right tile.

    LDA.b $08
    BEQ .right_pairs_done

.right_pair_loop
    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16TopRight,X
    STA.w $1100,Y

    INY
    INY

    LDA.l !Map16BottomRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    TXA
    CLC
    ADC.w #$0080
    TAX

    DEC.b $08
    BNE .right_pair_loop

.right_pairs_done
    PLP
    BCC .done

    LDA.l $7E2000,X
    ASL A
    TAX

    LDA.l !Map16TopRight,X
    STA.w $1100,Y

    INY
    INY

.done
    LDA.b $02                   ; Advance logical Y by the original length.
    CLC
    ADC.b $0A
    AND.w #$007F
    STA.b $02
    RTS

;---------------------------------------------------------------------------------------------------
; Convert logical (X, $02) to its BG2 VRAM word address.
;
; BG2 is two horizontal 32x32 screen blocks based at VRAM word $0000.
; Returns the address in A and clobbers $0A.
;---------------------------------------------------------------------------------------------------

BG2CalculateVRAMAddress:
    LDA.b $02                   ; Logical Y may be anywhere in 0..127.
    AND.w #$001F                ; Physical tilemap row wraps every 32 rows.
    ASL A                       ; Multiply the physical row by 32 words.
    ASL A
    ASL A
    ASL A
    ASL A
    STA.b $0A                   ; Row offset within one 32x32 screen block.

    TXA
    AND.w #$001F                ; Column within its 32-tile screen block.
    CLC
    ADC.b $0A
    STA.b $0A

    TXA
    AND.w #$0020                ; Logical X bit 5 selects left or right block.
    BEQ .done

    LDA.b $0A
    CLC
    ADC.w #$0400                ; Skip one 32x32 block (1024 VRAM words).
    STA.b $0A

.done
    LDA.b $0A
    RTS

;---------------------------------------------------------------------------------------------------
; Prepare the source-world margins before MirrorWarp_Initialize changes $8A.
;---------------------------------------------------------------------------------------------------

BG2MirrorInitialize:
    STZ.w $420C                 ; HDMAEN: stop hardware before rewriting its table.
    JSL $00FDEE                 ; InitializeMirrorHDMA
    JSR BG2BuildMirrorMargins
    STZ.b $9B                   ; Do not combine the margin DMA with HDMA startup.
    RTL

BG2MirrorRunAnimation:
    ; Choose the next NMI's HDMA state before running an animation step.
    ; Several loading steps take most of a frame; setting $9B afterward lets
    ; NMI observe the temporary enabled value in the middle of that work.
    LDA.w $0200
    BEQ .enable_hdma
    CMP.b #$0E
    BCS .enable_hdma

    ; $0200 reaches step 1 before the fade-to-white has finished. Keep the
    ; wave active until filter mode 2 reaches zero; loading work does not
    ; advance while that filter is still active.
    LDA.l $7EC009
    BNE .enable_hdma

    STZ.w $420C
    STZ.b $9B
    BRA .hdma_ready

.enable_hdma
    LDA.b #$C0
    STA.b $9B                   ; Default: enable mirror channels after NMI.

.hdma_ready
    ; Step 1 starts replacing BG character graphics. Finish fading to white
    ; before it, then hold white through step 13 so every loading transfer and
    ; its HDMA-disabled frame remain hidden.
    LDA.w $0200
    BEQ .run_animation          ; Step 0 is only the normal startup delay.

    CMP.b #$0E
    BCS .run_animation          ; Loading is done; allow the fade to resume.

    LDA.l $7EC009
    BNE .advance_filter         ; Mode 2 has not reached full white yet.

    ; The palette and wave are both paused, so their alternating frame would
    ; otherwise do no work. Advance one loading step on every white frame.
    JSL $00D8A4                 ; AnimateMirrorWarp, without changing palette.

    LDA.b #$01
    STA.w $06BB                 ; Resume with a palette step after loading.

.keep_filter_white
    REP #$20
    LDA.w #$001E                ; Hold at the full-white boundary.
    STA.l $7EC007
    SEP #$20

    ; Loading is paused at the same logical palette time. Freeze the software
    ; oscillator too, so it has vanilla-relative history when fading resumes.
    JSR .prepare_nmi
    RTL                         ; Skip the vanilla wave-table update at $00FE68.

.advance_filter
    DEC.w $06BB                 ; Preserve vanilla's one-filter-step cadence.
    BNE .animation_done

    LDA.b #$02
    STA.w $06BB
    JSL $00EEEC                 ; PaletteFilter_BlindingWhite
    BRA .animation_done

.run_animation
    JSL $00EEE2                 ; MirrorWarp_RunAnimationSubmodules

.animation_done
    JSR .prepare_nmi
    JML $00FE68                 ; Build the wave table as vanilla does.

.prepare_nmi
    LDA.b $17                   ; Specialized NMI transfer queued?
    ORA.b $18                   ; Arbitrary DMA list queued?
    BEQ .keep_hdma

.disable_hdma
    LDA.b #$01
    STA.w !NMISkipOAM          ; This mirror transfer may also omit OAM once.
    STZ.w $420C                 ; Stop the active channels before table writes.
    STZ.b $9B                   ; Keep them off through the upcoming NMI.

.keep_hdma
    RTS

; Step 7 has finished rebuilding the destination's logical Map16 map.
BG2MirrorBulkRender:
    JSL BG2BulkRender
    RTL

; Reproduce the step-8 tail displaced by the bank-00 margin trampoline.
BG2MirrorTriggerOverlayA:
    LDA.b #$0C
    STA.b $17
    STA.w $0710
    RTL

; Step 10 retains its animated-tile work, then repairs the mirror margins.
BG2MirrorRenderMargins:
    JSL $00D915                 ; AnimateMirrorWarp_DecompressAnimatedTiles
    JSR BG2BuildMirrorMargins
    RTL

;---------------------------------------------------------------------------------------------------
; Build one mirror-margin $1100/$18 list.
;
; The HDMA oscillator reaches -9..+9 pixels. Across every possible subpixel
; camera position, the normal left..left+32 window therefore needs columns
; left-2, left-1, and left+33. At the far-right area edge, also repair the
; normal window's out-of-bounds left+32 column.
;---------------------------------------------------------------------------------------------------

BG2BuildMirrorMargins:
    PHP
    REP #$30

    PHX
    PHY

    JSR BG2CalculateLogicalWindowOrigin

    LDY.w #$0000

    ; VRAM is a ring, so the two destinations wrap at X=0. Logical map data
    ; does not wrap there: repeat source column 0 outside the world edge.
    LDA.b $00
    SEC
    SBC.w #$0002
    AND.w #$007F
    STA.b $0E

    LDA.b $00
    CMP.w #$0002
    BCS .left_two_inside
    LDA.w #$0000
    BRA .left_two_ready

.left_two_inside
    SEC
    SBC.w #$0002

.left_two_ready
    STA.b $06
    JSR BG2EmitColumn

    LDA.b $00
    DEC A
    AND.w #$007F
    STA.b $0E

    LDA.b $00
    BEQ .left_one_ready
    DEC A

.left_one_ready
    STA.b $06
    JSR BG2EmitColumn

    ; Clamp right-side sources to the last logical column: 63 for a small
    ; area, 127 for a large area.
    LDA.w $0712
    AND.w #$00FF
    BEQ .small_area
    LDA.w #$007F
    BRA .have_right_edge

.small_area
    LDA.w #$003F

.have_right_edge
    STA.b $04

    ; The normal 33-tile window already wrote left+32. At the far-right area
    ; boundary that destination wrapped around the VRAM ring, but its source
    ; was outside the loaded Map16 map. Replace it with the edge column.
    LDA.b $00
    CLC
    ADC.w #$0020
    CMP.b $04
    BCC .right_margin
    BEQ .right_margin

    AND.w #$007F
    STA.b $0E

    LDA.b $04
    STA.b $06
    JSR BG2EmitColumn

.right_margin
    ; left+33 is the additional column exposed by the mirror wave.
    LDA.b $00
    CLC
    ADC.w #$0021
    AND.w #$007F
    STA.b $0E

    LDA.b $00
    CLC
    ADC.w #$0021
    CMP.b $04
    BCC .right_ready
    BEQ .right_ready
    LDA.b $04

.right_ready
    STA.b $06
    JSR BG2EmitColumn

    LDA.w #$FFFF
    STA.w $1100,Y

    SEP #$20
    LDA.b #$01
    STA.b $18

    REP #$20
    PLY
    PLX

    PLP
    RTS

;---------------------------------------------------------------------------------------------------
; Restore the vanilla 64x64 BG2 layout before a full underworld load.
;---------------------------------------------------------------------------------------------------

BG2RestoreUnderworldLayout:
    PHP                         ; Preserve the caller's accumulator width.

    SEP #$20                    ; BG2SC and $1B are byte-wide.

    LDA.b #$03                  ; base=$0000, width=64, height=64
    STA.w $2108                 ; Restore the vanilla dungeon BG2SC value.

    LDA.b #$01                  ; Reproduce the two instructions replaced by
    STA.b $1B                   ; the JSL at LoadUnderworldEntrance+$03.

    PLP                         ; Restore the caller's accumulator width.
    RTL

;---------------------------------------------------------------------------------------------------
; Spread the whirlpool destination load across consecutive solid-blue frames.
;---------------------------------------------------------------------------------------------------

BG2WhirlpoolQueueOverlayLatter:
    LDA.b #$0C                  ; Upload the latter 4 KiB BG1 overlay half.
    STA.b $17
    STZ.b $15                   ; Preserve the displaced state-3 behavior.

    LDA.b #$01
    STA.w !NMISkipOAM           ; Make room for this known-heavy NMI.
    RTL

BG2WhirlpoolQueueOverlayFormer:
    LDA.b #$0D                  ; Upload the former 4 KiB BG1 overlay half.
    STA.b $17

    LDA.b #$01
    STA.w !NMISkipOAM

    INC.w $0710                 ; Reproduce AdvanceWhirlpooling.
    INC.b $B0
    RTL

BG2WhirlpoolLoadDestination:
    LDA.w $0200                 ; Overworld_LoadOverlayAndMap advances this
    BNE .upload_bg2             ; from 0 to 1 after preparing the BG2 list.

    JSL $0AB96C                 ; Overworld_LoadOverlayAndMap
    STZ.b $18                   ; Preserve $1100, but defer its NMI upload.

    LDA.b #$0C                  ; First upload the latter BG1 overlay half.
    STA.b $17
    LDA.b #$01
    STA.w !NMISkipOAM

    LDA.b #$0F                  ; Keep the solid-blue screen and HUD visible.
    STA.b $13
    RTL                         ; Stay in state 5 for one additional frame.

.upload_bg2
    STZ.w $0200                 ; Consume our existing one-bit phase state.
    LDA.b #$01
    STA.b $18                   ; Upload the preserved destination BG2 list.
    STA.w !NMISkipOAM

    LDA.b #$0F
    STA.b $13
    INC.b $B0                   ; Continue to state 6 on the next frame.
    RTL

assert pc() <= !BG2BulkFreeEnd
