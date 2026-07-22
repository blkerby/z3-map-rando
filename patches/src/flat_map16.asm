lorom

!OverworldScreenSize = $02F5F1
!FlatMap16ScreenPointers = $27E000

org $02F2AE

; Build the 64x64 WRAM Map16 data (TMAPA) from 32x32 flat Map16 data.
; This replaces the slower, more complex decompression involving Map32.
Overworld_LoadAllFlatMap16:
    ; Store the data bank since it will be overwritten by MVN.
    PHB

    ; top-left: screen $8A -> $7E2000
    LDA.b $8A
    LDY.w #$2000
    JSR Overworld_LoadOneFlatMap16

    LDX.b $8A
    LDA.l !OverworldScreenSize,X
    AND.w #$00FF
    BNE .done  ; For a small area, stop after loading one quadrant.

    ; top-right: screen $8A+1 -> $7E2040
    LDA.b $8A
    INC A
    LDY.w #$2040
    JSR Overworld_LoadOneFlatMap16

    ; bottom-left: screen $8A+8 -> $7E3000
    LDA.b $8A
    CLC
    ADC.w #$0008
    LDY.w #$3000
    JSR Overworld_LoadOneFlatMap16

    ; bottom-right: screen $8A+9 -> $7E3040
    LDA.b $8A
    CLC
    ADC.w #$0009
    LDY.w #$3040
    JSR Overworld_LoadOneFlatMap16

.done
    PLB
    RTS

; A: screen ID
; Y: destination address in WRAM bank $7E
;
; Direct-page scratch:
;   $00: screen ID while calculating its pointer-table offset
;   $02-$05: executable MVN/RTS thunk
;   $0E: source rows remaining
Overworld_LoadOneFlatMap16:
    ; X <- A * 3
    STA.b $00
    ASL A
    CLC
    ADC.b $00
    TAX

    ; To perform MVN using a dynamic source bank, we write MVN + RTS
    ; instructions to WRAM, at $02. The same instruction will be reused
    ; for each row of Map16 tiles.
    LDA.w #$7E54  ; MVN opcode, destination bank $7E
    STA.b $02
    LDA.w #$6000  ; source-bank placeholder, RTS opcode
    STA.b $04

    ; Set the source bank (in the MVN instruction)
    SEP #$20
    LDA.l !FlatMap16ScreenPointers+2,X
    STA.b $04
    REP #$20

    LDA.w #$0020
    STA.b $0E

    ; Each screen has a little-endian 24-bit pointer to its flat map.
    LDA.l !FlatMap16ScreenPointers,X
    TAX

.next_row
    ; MVN copies A+1 bytes, so $003F copies one 64-byte Map16 row.
    LDA.w #$003F
    JSR.w $0002
    ; MVN advances X to the next source row.

    ; MVN also advances Y by $40. Skip the other quadrant's $40-byte row
    ; to reach the next row in the 128-byte-wide WRAM map.
    TYA
    CLC
    ADC.w #$0040
    TAY

    DEC.b $0E
    BNE .next_row

    RTS

assert pc() <= $02F39C

org $02F52F

; Load the 32x32 flat Map16 overlay into the top-left of TMAPB.
LoadSubOverlayFlatMap16:
    PHB

    LDA.b $8A
    LDY.w #$4000
    JSR Overworld_LoadOneFlatMap16

    PLB
    RTS

assert pc() <= $02F5E3
