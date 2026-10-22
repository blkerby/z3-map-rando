; Reuse the vanilla item slot ($24/$34) for dynamic pickups and the vanilla
; heart-piece slot ($E0/$F0) for heart-piece carriers. Keeping those independent
; lets the Spectacle Rock pickup coexist with the falling Ether Tablet item.
; No equipment/follower slots are borrowed. Native receipt IDs and carrier
; subtypes stay unchanged; only their displayed artwork is selected here.
;
; $7FF180-$7FF47F lies in documented free WRAM. The heart-piece buffer holds
; four 4bpp tiles; a second buffer preserves the shared item image while loading
; a heart-piece preview. Three cached rupee frames avoid decompressing their
; sheets repeatedly during animation. Narrow images have transparent right tiles.

!PickupReceipt = $7FF180
!LoadedPickupGfx = $7FF181
!PickupGfxValid = $7FF182
!HeartPickupGfx = $7FF183
!HeartPickupValid = $7FF184
!DrawHeartPickup = $7FF185
!DrawAssignedMilestone = $7FF186
!RupeeGraphicsValid = $7FF187
!LoadedPickupReceipt = $7FF188
!PickupPreviewValid = $7FF189
!HeartPickupReceipt = $7FF18A
!HeartPickupBuffer = $7FF200
!SavedItemBuffer = $7FF280
!RupeeGraphicsBuffer = $7FF300
ItemGraphicsCodeStart:

; ClearOAMBuffer's loop returns to its second instruction, so keep that routine
; intact. Reset frame metadata after clearing and before the module draws sprites.
org $808056
hook_begin_pickup_frame:
    JSL BeginPickupFrame

org $80E310
hook_initialize_pickup_graphics:
    JML InitializePickupGraphics
    NOP

; Both the decompression loader and the receipt's rupee animation enter this
; converter. Recording every write makes previews reload after native receipts.
org $80D54F
hook_convert_item_graphics:
    JML RecordItemGraphics
    NOP #2

; Run after the vanilla item upload. Entry is A8/X16/Y16 with DB=$00.
org $808B22
hook_upload_heart_pickup:
    JSL UploadHeartPickup
    NOP #2

org $85EE78
hook_draw_mushroom_item:
    JSL DrawMushroomItem
org $85EF79
hook_draw_heart_container_item:
    JSL DrawHeartContainerItem
org $85F030
hook_draw_heart_piece_item:
    JSL DrawHeartPieceItem
org $85FC9E
hook_draw_book_item:
    JSL DrawBookItem
org $9DDF81
hook_draw_water_item:
    JSL DrawWaterItem

; These callers use JSR. Their helpers return through the original bank's RTS.
org $85F67B
hook_draw_powder_item:
    JML DrawPowderItem
org $858DA8
hook_draw_pedestal_item:
    JML DrawPedestalItem

; The Grove carrier already uses the shared item image and one 16x16 OBJ.
org $88D045
hook_draw_grove_item:
    JSL DrawGroveItem
org $88D04A
hook_color_grove_item:
    JSL ColorGroveItem

; Select a visual receipt ID for tablets without changing their dispatch ID.
; All other uses of this native drawer, including prizes/fairies, stay native.
org $88C6BA
hook_select_milestone_graphics:
    JSL SelectMilestoneGraphics
org $88C6C8
hook_color_milestone_top:
    JSL ReadMilestonePalette
org $88C6F9
hook_color_milestone_bottom:
    JSL ReadMilestonePalette

org ItemGraphicsCodeStart

BeginPickupFrame:
    PHP
    PHA
    LDA.b #$00
    STA.l !DrawHeartPickup
    PLA
    PLP
    JML $8080B5 ; Run hi-jacked instruction: dispatch the current module.

InitializePickupGraphics:
    LDA.b #$00
    STA.l !PickupGfxValid
    STA.l !HeartPickupValid
    STA.l !DrawHeartPickup
    STA.l !RupeeGraphicsValid
    STA.l !PickupPreviewValid
    ; Run hi-jacked instructions. PHK must select the original table bank.
    PHB
    LDA.b #$80
    PHA
    PLB
    LDA.b #$80
    JML $80E315

RecordItemGraphics:
    PLA ; Run hi-jacked instruction
    STA.l !LoadedPickupGfx
    PHA
    LDA.b #$01
    STA.l !PickupGfxValid
    LDA.b #$00
    STA.l !PickupPreviewValid
    PLA
    ; Run hi-jacked instructions:
    REP #$21
    AND.w #$00FF
    JML $80D555

UploadHeartPickup:
    STA.w $420B ; Run hi-jacked instruction: finish the native item DMA first.
    PHP
    PHX
    PHY
    LDA.l !DrawHeartPickup
    BEQ .done
    LDY.w #$1801
    STY.w $4300
    LDA.b #$80
    STA.w $2115
    LDA.b #$7F
    STA.w $4304
    LDY.w #!HeartPickupBuffer
    STY.w $4302
    LDY.w #$0040
    STY.w $4305
    LDY.w #$4E00
    STY.w $2116
    LDA.b #$01
    STA.w $420B
    LDY.w #$0040
    STY.w $4305
    LDY.w #$4F00
    STY.w $2116
    LDA.b #$01
    STA.w $420B
    LDA.b #$7E
    STA.w $4304 ; Subsequent vanilla DMA uses WRAM bank $7E.
.done
    PLY
    PLX
    PLP
    LDA.w $0AF4 ; Run hi-jacked instruction
    RTL

; Resolve the next tier before comparing cached artwork. Previewing never grants.
SelectPickupReceipt:
    JSL ResolveProgressiveItem
    CMP.b #$50
    BNE +
    LDA.b #$01
+   CMP.b #$49
    BNE +
    LDA.b #$00 ; Use the Fighter Sword preview's common palette and color remap.
+   CMP.b #$4E
    BNE +
    LDA.b #$45
+   STA.l !PickupReceipt
    RTL

ReadPickupGraphics:
    LDA.l !PickupReceipt
    TAX
    LDA.l $8983FE,X
    CMP.b #$24
    BNE .done
    LDA.b $1A
    LSR A
    LSR A
    LSR A
    AND.b #$03
    TAX
    LDA.l PickupRupeeFrames,X
.done
    RTS

PickupRupeeFrames:
    db $24,$25,$26,$25

; Native equipment palettes depend on owned equipment. Previews use the common
; red/blue/gold item palettes instead, leaving Link's equipment colors intact.
ReadPickupPalette:
    PHX
    LDA.l !PickupReceipt
    TAX
    CMP.b #$07
    BCS .native
    LDA.l PickupEquipmentPalettes,X
    BRA .done
.native
    LDA.l $898496,X
.done
    PLX
    RTL

PickupEquipmentPalettes:
    db $02,$02,$01,$04,$02,$01,$04

; Reindex native sword/shield pixels into their chosen common palette. Native
; palette 5 dedicates colors 1-3 to swords and 4-7 to shields; the common palettes
; arrange those colors differently. Keep transparency and outline colors intact.
PickupEquipmentColors:
    db $00,$01,$06,$03,$04,$05,$06,$07 ; Fighter Sword
    db $00,$03,$04,$07,$04,$05,$06,$07 ; Master Sword
    db $00,$03,$04,$07,$04,$05,$06,$07 ; Tempered Sword
    db $00,$06,$01,$03,$04,$05,$06,$07 ; Golden Sword
    db $00,$01,$02,$03,$01,$05,$04,$03 ; Fighter Shield
    db $00,$01,$02,$03,$04,$05,$02,$03 ; Fire Shield
    db $00,$01,$02,$03,$01,$05,$06,$07 ; Mirror Shield

; The loader changes A/X/Y/DB and direct-page $00-$0F/$BD-$BE/$C8-$CD. Preserve the
; caller's coordinates, OAM pointers, and scratch; decompression runs only when
; the requested image differs from the current cache (including rupee frames).
PreparePickupGraphics:
    PHP
    PHB
    PHX
    PHY
    REP #$20
    LDA.b $00 : PHA
    LDA.b $02 : PHA
    LDA.b $04 : PHA
    LDA.b $06 : PHA
    LDA.b $08 : PHA
    LDA.b $0A : PHA
    LDA.b $0C : PHA
    LDA.b $0E : PHA
    LDA.b $BD : PHA
    LDA.b $C8 : PHA
    LDA.b $CA : PHA
    LDA.b $CC : PHA
    SEP #$20
    LDA.l !PickupGfxValid
    BEQ .load
    LDA.l !PickupPreviewValid
    BEQ .load
    LDA.l !PickupReceipt
    CMP.l !LoadedPickupReceipt
    BNE .load
    JSR ReadPickupGraphics
    CMP.l !LoadedPickupGfx
    BEQ .pad
.load
    JSR ReadPickupGraphics
    CMP.b #$24
    BCC .single_image
    CMP.b #$27
    BCS .single_image
    JSR PrepareRupeeGraphics
    BRA .prepared
.single_image
    JSL $80D52D
.prepared
    JSR RemapEquipmentGraphics
    LDA.l !PickupReceipt
    STA.l !LoadedPickupReceipt
    LDA.b #$01
    STA.l !PickupPreviewValid
.pad
    LDA.l !PickupReceipt
    TAX
    LDA.l $89844A,X
    BNE .done
    LDX.b #$1F
    LDA.b #$00
.clear_right
    STA.l $7EBD60,X
    STA.l $7EBDA0,X
    DEX
    BPL .clear_right
.done
    REP #$20
    PLA : STA.b $CC
    PLA : STA.b $CA
    PLA : STA.b $C8
    PLA : STA.b $BD
    PLA : STA.b $0E
    PLA : STA.b $0C
    PLA : STA.b $0A
    PLA : STA.b $08
    PLA : STA.b $06
    PLA : STA.b $04
    PLA : STA.b $02
    PLA : STA.b $00
    SEP #$20
    PLY
    PLX
    PLB
    PLP
    RTL

; Process the three color planes of each row. Plane 3's nontransparent-pixel
; mask stays unchanged. Scratch $00-$09/X/Y belongs to the enclosing loader.
RemapEquipmentGraphics:
    LDA.l !PickupReceipt
    CMP.b #$07
    BCC .remap
    RTS
.remap
    REP #$20
    AND.w #$00FF
    ASL A
    ASL A
    ASL A
    CLC
    ADC.w #PickupEquipmentColors
    STA.b $06
    SEP #$20
    LDA.b #PickupEquipmentColors>>16
    STA.b $08
    REP #$10
    LDX.w #$0000
.row
    LDA.l $7EBD40,X : STA.b $00
    LDA.l $7EBD41,X : STA.b $01
    LDA.l $7EBD50,X : STA.b $02
    STZ.b $03
    STZ.b $04
    STZ.b $05
    LDA.b #$08
    STA.b $09
.pixel
    PHX
    LDA.b #$00
    ASL.b $02 : ROL A
    ASL.b $01 : ROL A
    ASL.b $00 : ROL A
    REP #$20
    AND.w #$00FF
    TAY
    SEP #$20
    LDA.b [$06],Y
    LSR A : ROL.b $03
    LSR A : ROL.b $04
    LSR A : ROL.b $05
    PLX
    DEC.b $09
    BNE .pixel
    LDA.b $03 : STA.l $7EBD40,X
    LDA.b $04 : STA.l $7EBD41,X
    LDA.b $05 : STA.l $7EBD50,X
    INX
    INX
    REP #$20
    TXA
    AND.w #$001F
    CMP.w #$0010
    BNE .next_row
    TXA
    CLC
    ADC.w #$0010
    TAX
.next_row
    SEP #$20
    CPX.w #$0080
    BCC .row
    SEP #$10
.done
    RTS

; Prepare all three frames together while their decompressed sheets are resident.
; $06-$08/X/Y are scratch here, protected by PreparePickupGraphics's stack frame.
PrepareRupeeGraphics:
    LDA.l !RupeeGraphicsValid
    BNE .select_frame
    LDA.b #$24
    JSL $80D52D
.save_frame
    LDA.l !LoadedPickupGfx
    SEC
    SBC.b #$24
    REP #$20
    AND.w #$00FF
    XBA
    LSR A
    CLC
    ADC.w #!RupeeGraphicsBuffer
    STA.b $06
    SEP #$20
    LDA.b #$7F
    STA.b $08
    REP #$10
    LDX.w #$007F
.copy_frame
    LDA.l $7EBD40,X
    TXY
    STA.b [$06],Y
    DEX
    BPL .copy_frame
    SEP #$10
    LDA.l !LoadedPickupGfx
    CMP.b #$26
    BEQ .ready
    INC A
    JSL $80D51B
    BRA .save_frame
.ready
    LDA.b #$01
    STA.l !RupeeGraphicsValid
.select_frame
    JSR ReadPickupGraphics
    STA.l !LoadedPickupGfx
    SEC
    SBC.b #$24
    REP #$20
    AND.w #$00FF
    XBA
    LSR A
    CLC
    ADC.w #!RupeeGraphicsBuffer
    STA.b $06
    SEP #$20
    LDA.b #$7F
    STA.b $08
    REP #$10
    LDX.w #$007F
.load_frame
    TXY
    LDA.b [$06],Y
    STA.l $7EBD40,X
    DEX
    BPL .load_frame
    SEP #$10
    LDA.b #$01
    STA.l !PickupGfxValid
    RTS

PrepareHeartPickupGraphics:
    PHP
    PHX
    PHY
    LDA.l !HeartPickupValid
    BEQ .load
    LDA.l !PickupReceipt
    CMP.l !HeartPickupReceipt
    BNE .load
    JSR ReadPickupGraphics
    CMP.l !HeartPickupGfx
    BEQ .done
.load
    LDA.l !LoadedPickupGfx : PHA
    LDA.l !PickupGfxValid : PHA
    LDA.l !LoadedPickupReceipt : PHA
    LDA.l !PickupPreviewValid : PHA
    REP #$10
    LDX.w #$007F
.save_shared
    LDA.l $7EBD40,X
    STA.l !SavedItemBuffer,X
    DEX
    BPL .save_shared
    SEP #$10
    JSL PreparePickupGraphics
    REP #$10
    LDX.w #$007F
.copy_graphics
    LDA.l $7EBD40,X
    STA.l !HeartPickupBuffer,X
    LDA.l !SavedItemBuffer,X
    STA.l $7EBD40,X
    DEX
    BPL .copy_graphics
    SEP #$10
    PLA : STA.l !PickupPreviewValid
    PLA : STA.l !LoadedPickupReceipt
    PLA : STA.l !PickupGfxValid
    PLA : STA.l !LoadedPickupGfx
    JSR ReadPickupGraphics
    STA.l !HeartPickupGfx
    LDA.l !PickupReceipt
    STA.l !HeartPickupReceipt
    LDA.b #$01
    STA.l !HeartPickupValid
.done
    PLY
    PLX
    PLP
    RTL

; Replace only the body OBJ; the vanilla drawer still computes collision
; coordinates, offscreen flags, physics-related scratch, and any shadow.
WritePickupBody:
    PHP
    PHX
    PHY
    LDY.b #$02
    STA.b ($90),Y
    INY
    LDA.b ($90),Y
    AND.b #$30
    PHA
    JSL ReadPickupPalette
    ASL A
    ORA $01,s
    STA.b ($90),Y
    PLA
    LDA.l !PickupReceipt
    TAX
    LDA.l $89844A,X
    BNE .done
    ; Center an 8x16 image in the carrier's existing 16x16 footprint.
    LDY.b #$00
    LDA.b ($90),Y
    CLC
    ADC.b #$04
    STA.b ($90),Y
    LDA.b ($92)
    AND.b #$01
    ADC.b #$00
    AND.b #$01
    ORA.b #$02
    STA.b ($92)
.done
    PLY
    PLX
    PLP
    RTS

DrawSharedPickup:
    JSL SelectPickupReceipt
    JSL PreparePickupGraphics
    LDA.b #$24
    JSR WritePickupBody
    RTL

macro draw_item(name, item)
Draw<name>Item:
    JSL $86DBF8
    LDY.b #$01
    LDA.b ($90),Y
    CMP.b #$F0
    BEQ .done
    LDA.l <item>
    JSL DrawSharedPickup
.done
    RTL
endmacro
%draw_item(Mushroom, !MushroomItem)
%draw_item(Book, !BookItem)

DrawHeartContainerItem:
    JSL $86DBF8
    LDY.b #$01
    LDA.b ($90),Y
    CMP.b #$F0
    BEQ .done
    JSL ReadHeartItem
    JSL DrawSharedPickup
.done
    RTL

DrawHeartPieceItem:
    JSL $86DBF8
    LDY.b #$01
    LDA.b ($90),Y
    CMP.b #$F0
    BEQ .done
    JSL ReadHeartItem
    JSL SelectPickupReceipt
    JSL PrepareHeartPickupGraphics
    LDA.b #$01
    STA.l !DrawHeartPickup
    LDA.b #$E0
    JSR WritePickupBody
.done
    RTL

DrawWaterItem:
    JSL $86DBF8
    LDY.b #$01
    LDA.b ($90),Y
    CMP.b #$F0
    BEQ .done
    LDA.w $0D90,X
    CMP.b #$1E
    BNE .catfish
    LDA.l !KingZoraItem
    BRA .draw
.catfish
    LDA.l !CatfishItem
.draw
    JSL DrawSharedPickup
.done
    RTL

DrawPowderItem:
    JSL DrawPickupWithoutShadow
    LDY.b #$01
    LDA.b ($90),Y
    CMP.b #$F0
    BEQ .done
    LDA.l !PowderItem
    JSL DrawSharedPickup
.done
    JML $85F68D

DrawPedestalItem:
    JSL DrawPickupWithoutShadow
    LDY.b #$01
    LDA.b ($90),Y
    CMP.b #$F0
    BEQ .done
    ; Retain the pedestal's bottom two 8x8 tiles, underneath the item image.
    LDY.b #$04
    LDA.b $00
    SEC
    SBC.b #$08
    STA.b ($90),Y
    INY
    LDA.b $02
    CLC
    ADC.b #$08
    STA.b ($90),Y
    INY
    LDA.b #$E0
    STA.b ($90),Y
    INY
    LDA.b $05
    STA.b ($90),Y
    INY
    LDA.b $00
    STA.b ($90),Y
    INY
    LDA.b $02
    CLC
    ADC.b #$08
    STA.b ($90),Y
    INY
    LDA.b #$F0
    STA.b ($90),Y
    INY
    LDA.b $05
    STA.b ($90),Y
    LDY.b #$02
    LDA.b $01
    AND.b #$01
    STA.b ($92),Y
    DEY
    LDA.b $00
    CMP.b #$08
    LDA.b $01
    SBC.b #$00
    AND.b #$01
    STA.b ($92),Y
    ; Position the replacement image where the upper four sword tiles were.
    LDY.b #$00
    LDA.b $00
    SEC
    SBC.b #$08
    STA.b ($90),Y
    LDA.b $01
    SBC.b #$00
    AND.b #$01
    ORA.b #$02
    STA.b ($92)
    INY
    LDA.b $02
    SEC
    SBC.b #$08
    STA.b ($90),Y
    LDA.l $8589B0
    JSL DrawSharedPickup
.done
    JML $858DD7

; These two vanilla carriers have custom drawers without a shadow. Temporarily
; suppress the generic drawer's shadow so it leaves the body coordinates intact
; and stays within their original OAM allocations. Restore the sprite flags.
DrawPickupWithoutShadow:
    LDA.w $0E60,X
    PHA
    AND.b #$EF
    STA.w $0E60,X
    JSL $86DBF8
    PLA
    STA.w $0E60,X
    RTL

DrawGroveItem:
    PHX
    PHY
    LDA.l !GroveItem
    JSL SelectPickupReceipt
    JSL PreparePickupGraphics
    LDA.l !PickupReceipt
    TAX
    LDA.l $89844A,X
    BNE .wide
    LDY.b #$00
    LDA.b ($90),Y
    CLC
    ADC.b #$04
    BCC .store_x
    ; This ancilla's native drawer has no ninth X bit. Keep a centered image
    ; beyond the right edge offscreen instead of wrapping it to the left edge.
    INY
    LDA.b #$F0
    STA.b ($90),Y
    BRA .wide
.store_x
    STA.b ($90),Y
.wide
    PLY
    PLX
    ; Run hi-jacked instructions:
    LDA.b #$24
    STA.b ($90),Y
    RTL

ColorGroveItem:
    JSL ReadPickupPalette
    ASL A
    ORA.b $65 ; Run hi-jacked instruction
    RTL

SelectMilestoneGraphics:
    LDA.b #$00
    STA.l !DrawAssignedMilestone
    LDA.b $1B
    BNE .native
    LDA.w $0C4A,X
    CMP.b #$29
    BNE .native
    LDA.w $0C5E,X
    CMP.b #$10
    BEQ .ether
    CMP.b #$0F
    BNE .native
    LDA.l !BombosItem
    BRA .assigned
.ether
    LDA.l !EtherItem
.assigned
    JSL SelectPickupReceipt
    JSL PreparePickupGraphics
    LDA.b #$01
    STA.l !DrawAssignedMilestone
    LDA.l !PickupReceipt
    TAX
    RTL
.native
    ; Run hi-jacked instructions:
    LDA.w $0C5E,X
    TAX
    RTL

ReadMilestonePalette:
    LDA.l !DrawAssignedMilestone
    BEQ .native
    JML ReadPickupPalette
.native
    LDA.l $898496,X ; Run hi-jacked instruction
    RTL
