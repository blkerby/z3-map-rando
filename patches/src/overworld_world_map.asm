; Load the generated Mode 7 maps and display their uncollected dungeon prizes.
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A09B80
!free_space_bank_a0_end = $A09CD0
!free_space_bank_ca_start = $CA8000
!free_space_bank_ca_end = $CB8000
!free_space_bank_cb_start = $CB8000
!free_space_bank_cb_end = $CBA000

; The original routine uses 8-bit A, 16-bit Y, and scratch $00-$02; X is unused.
; It returns with 8-bit A/X/Y. Keep the same widths and scratch ownership.
org $80E3D9
hook_load_world_map_characters:
    JML LoadWorldMapCharacters

org $808E57
hook_upload_world_map_tilemap:
    LDA.b #!WorldMapTilemaps>>16

; A is 16-bit and X/Y are 8-bit here. Only A and scratch $02/$04 change.
org $808E64
hook_initialize_world_map_tilemap_cursor:
    JSL InitializeWorldMapTilemapCursor

org $808E84
    ADC.w #!WorldMapTilemaps&$FFFF

org $8ABA28
hook_select_world_map_rain_color_math:
    JSL GetWorldMapRainContext
    NOP
    NOP
    BEQ return_world_map_color_math
assert pc() == $8ABA30

org $8ABA38
return_world_map_color_math:

; Both worlds now upload a full tilemap in the preceding stage.
org $8ABA99
hook_load_dark_world_map:
    INC.w $0200
    RTL

; Link and the portal are already drawn. The native exit restores their saved coordinates.
org $8AC012
hook_draw_world_map_prizes:
    JSL DrawWorldMapPrizes
    JMP.w return_restore_world_map_coordinates

; Call the native map projection and sprite routines from bank $8A.
ProjectWorldMapPrize:
    JSR $C59B
    JSR $C3B1
    RTL

DrawWorldMapPrize:
    LDA.b $0D
    CMP.b #$64
    BEQ .draw
    ; Pendants share the crystal icon phase.
    LDA.b $1A
    AND.b #$10
    BEQ .done
.draw
    JSR $C52E
.done
    RTL

org $8AC39C
return_restore_world_map_coordinates:

org $8AC53E
hook_read_world_map_crystal_number:
    JSL ReadWorldMapCrystalNumber

org !free_space_bank_a0_start
LoadWorldMapCharacters:
    LDA.b #!WorldMapGraphics>>16
    STA.b $02
    LDA.b #$80
    STA.w $2115
    STZ.w $2116
    STZ.w $2117
    REP #$10
    LDY.w #!WorldMapGraphics&$FFFF
    LDA.b $8A
    AND.b #$40
    BEQ .light
    LDY.w #(!WorldMapGraphics+$4000)&$FFFF
.light
    STY.b $00
    LDY.w #$0000
.next
    LDA.b [$00],Y
    STA.w $2119
    INY
    LDA.b [$00],Y
    STA.w $2119
    INY
    LDA.b [$00],Y
    STA.w $2119
    INY
    LDA.b [$00],Y
    STA.w $2119
    INY
    CPY.w #$4000
    BNE .next
    SEP #$10
    RTL

InitializeWorldMapTilemapCursor:
    STZ.b $04 ; Run hi-jacked instruction

    LDA.b $8A
    AND.w #$0040
    BEQ .light
    LDA.w #$1000
.light
    STA.b $02
    RTL

; Entry and return use 8-bit A/X/Y; preserve X/Y and scratch RAM.
GetWorldMapRainContext:
    REP #$30
    PHX
    LDX.w $040A
    JSL !GetRainContext
    SEP #$20
    CMP.b #$00
    BEQ .done

    PHA
    LDA.b $11
    CMP.b #$07                ; The shared map setup also runs for the flute menu.
    BNE .restore_context
    PLA
    PHA
    CMP.b #$02
    REP #$20
    LDA.w #!LightRainGroundColor
    BCC .tint
    LDA.w #!DarkRainGroundColor
.tint
    PHA
    LDX.w #$0182              ; Green pendant uses OBJ palette 4.
.next_color
    LDA.l $7EC300,X
    %tint_rain_sprite_color()
    INX
    INX
    CPX.w #$01A0
    BNE .next_color
    PLA
    SEP #$20
.restore_context
    PLA
.done
    PLX
    SEP #$30
    CMP.b #$00
    RTL

; Entry and exit are A8/X8/Y8. The native projection uses scratch $00/$0E/$0F
; and clobbers X. Y holds the OAM slot, and the native exit restores $7EC108-$7EC10B.
DrawWorldMapPrizes:
    LDY.b #$0E
    LDX.b #$00
    LDA.b $8A
    AND.b #$40
    BEQ .next
    LDX.b #$04
.next
    LDA.l .dungeons,X
    BMI .done
    PHX
    TAX
    LDA.l !DungeonPrizeTypes,X
    BEQ .pendant
    LDA.l $7EF37A
    BRA .check
.pendant
    LDA.l $7EF374
.check
    AND.l $82A09B,X
    PLX
    CMP.b #$00
    BNE .advance

    PHX
    LDA.l .position_offsets,X
    TAX
    REP #$20
    LDA.l $8ABDF6,X
    STA.l $7EC10A
    LDA.l $8ABE08,X
    STA.l $7EC108
    SEP #$20
    JSL ProjectWorldMapPrize
    PLX
    BCC .advance

    PHX
    LDA.l .icon_offsets,X
    TAX
    LDA.l $8ABEF2,X
    STA.b $0C
    LDA.l $8ABEF3,X
    STA.b $0D
    LDA.b #$02
    STA.b $0B
    TYX
    JSL DrawWorldMapPrize
    PLX
.advance
    DEY
    INX
    BRA .next
.done
    RTL

.dungeons
    db $02,$0A,$03,$FF                 ; Eastern, Hera, Desert
    db $06,$08,$0C,$0B,$07,$09,$05,$FF ; PoD, Skull, TR, Thieves, Mire, Ice, Swamp
.position_offsets
    ; Light and Dark prizes use entries 3 and 7 in each slot's coordinate tables.
    db 0*$24+6,1*$24+6,2*$24+6,0
    db 0*$24+14,1*$24+14,2*$24+14,3*$24+14,4*$24+14,5*$24+14,6*$24+14
.icon_offsets
    db 0*$12+6,1*$12+6,2*$12+6,0
    db 0*$12+14,1*$12+14,2*$12+14,3*$12+14,4*$12+14,5*$12+14,6*$12+14

ReadWorldMapCrystalNumber:
    ; OAM slots $08-$0E map to the interleaved Light/Dark number pairs at $B08070.
    PHX
    LDA.b $8A
    ASL
    ASL                      ; Carry selects the Dark World entry.
    TXA
    ROL
    TAX
    LDA.l $B08060,X
    PLX
    RTL
assert pc() <= !free_space_bank_a0_end

org !free_space_bank_ca_start
WorldMapGraphics:
assert WorldMapGraphics == !WorldMapGraphics
skip $8000
assert pc() <= !free_space_bank_ca_end

org !free_space_bank_cb_start
WorldMapTilemaps:
assert WorldMapTilemaps == !WorldMapTilemaps
skip $2000
assert pc() <= !free_space_bank_cb_end

org !WorldMapPalettes
WorldMapPalettes:
assert WorldMapPalettes == !WorldMapPalettes
