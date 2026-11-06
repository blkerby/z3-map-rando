; Load the two generated Mode 7 maps, including their authored cloud backgrounds.
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0F400
!free_space_bank_a0_end = $A0F500
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

; Both worlds now upload a full tilemap in the preceding stage.
org $8ABA99
hook_load_dark_world_map:
    INC.w $0200
    RTL

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
