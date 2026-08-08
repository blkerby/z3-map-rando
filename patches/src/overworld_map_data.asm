; Load flat Map16 overworld screen and overlay maps.
lorom

!OverworldScreenSize = $82F5F1
!FlatMap16ScreenPointers = $BFE000
!BG1FlatMap16ScreenPointers = $BFE200
!Area80BG1FlatMap16Pointers = $BFE400
!GeneratedBG1Enabled = $BFE406
!LostWoodsClearBG1FlatMap16Pointers = $BFE410
!RainContexts = $BFE430
!RainFlatMap16Pointers = $BFE4D0

!free_space_bank_a0_start = $A08700
!free_space_bank_a0_end = $A08900

org $82F2AE
hook_OverworldLoadAllMapQuadrants:
    JSL Overworld_LoadAllFlatMap16
    RTS
assert pc() == $82F2B3

org $82F52F
hook_LoadSubOverlayMap:
    JSL LoadSubOverlayFlatMap16
    RTS
assert pc() == $82F534

org !free_space_bank_a0_start

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
    RTL

; A: screen ID
; Y: destination address in WRAM bank $7E
;
; Direct-page scratch:
;   $00: screen ID while calculating its pointer-table offset
;   $02-$05: executable MVN/RTS thunk
;   $0D: source bank
;   $0E: source rows remaining
Overworld_LoadOneFlatMap16:
    ; X <- A * 3
    STA.b $00
    ASL A
    CLC
    ADC.b $00
    TAX

    SEP #$20
    LDA.l !FlatMap16ScreenPointers+2,X
    STA.b $0D
    REP #$20

    ; Each screen has a little-endian 24-bit pointer to its flat map.
    LDA.l !FlatMap16ScreenPointers,X
    TAX
    BRA Overworld_CopyOneFlatMap16

; A: screen ID; Y: destination address in WRAM bank $7E.
; Returns carry set when the generated BG1 pointer exists.
Overworld_LoadOneBG1FlatMap16:
    STA.b $00
    ASL A
    CLC
    ADC.b $00
    TAX

    LDA.l $7EC213
    AND.w #$00FF
    BNE .ordinary
    LDA.l $7EF300
    AND.w #$0040
    BEQ .ordinary

    SEP #$20
    LDA.l !LostWoodsClearBG1FlatMap16Pointers+2,X
    BEQ .missing
    STA.b $0D
    REP #$20

    LDA.l !LostWoodsClearBG1FlatMap16Pointers,X
    TAX
    BRA .copy

.ordinary
    SEP #$20
    LDA.l !BG1FlatMap16ScreenPointers+2,X
    BEQ .missing
    STA.b $0D
    REP #$20

    LDA.l !BG1FlatMap16ScreenPointers,X
    TAX

.copy
    JSR Overworld_CopyOneFlatMap16
    SEC
    RTS

.missing
    REP #$20
    CLC
    RTS

; X: byte offset of the area $80 variant pointer; Y: WRAM destination.
Overworld_LoadArea80BG1FlatMap16:
    SEP #$20
    LDA.l !Area80BG1FlatMap16Pointers+2,X
    STA.b $0D
    REP #$20

    LDA.l !Area80BG1FlatMap16Pointers,X
    TAX

; X: source address; Y: destination address in WRAM bank $7E; $0D: source bank.
Overworld_CopyOneFlatMap16:
    ; To perform MVN using a dynamic source bank, write MVN + RTS at $02.
    LDA.w #$7E54              ; MVN opcode, destination bank $7E
    STA.b $02
    LDA.w #$6000              ; source-bank placeholder, RTS opcode
    STA.b $04
    SEP #$20
    LDA.b $0D
    STA.b $04
    REP #$20

    LDA.w #$0020
    STA.b $0E

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

; Load authored BG1 or generated rain for playable overworld modules.
; Presentation modules retain their vanilla flat overlay.
LoadSubOverlayFlatMap16:
    PHB

    SEP #$20
    LDA.b $10
    CMP.b #$15
    BEQ .generated
    CMP.b #$08
    BCC .vanilla
    CMP.b #$0C
    BCS .vanilla
.generated
    LDA.l !GeneratedBG1Enabled
    BEQ .vanilla
    REP #$20

    JSR LoadGeneratedBG1FlatMap16
    BRA .done

.vanilla
    REP #$20
    LDA.b $8A
    LDY.w #$4000
    JSR Overworld_LoadOneFlatMap16

.done
    PLB
    RTL

; Load the BG1 maps matching the current BG2 area dimensions. The real area ID
; was cached before vanilla replaced $8A with its synthetic overlay ID.
; Direct-page scratch: $06-$07 holds the real area ID.
LoadGeneratedBG1FlatMap16:
    LDA.l $7EC213
    AND.w #$00FF
    CMP.w #$0080
    BEQ .area_80
    STA.b $06

    LDY.w #$4000
    JSR Overworld_LoadOneBG1FlatMap16
    BCC .no_authored_bg1

    LDX.b $06
    LDA.l !OverworldScreenSize,X
    AND.w #$00FF
    BNE .done

    LDA.b $06
    INC A
    LDY.w #$4040
    JSR Overworld_LoadOneBG1FlatMap16

    LDA.b $06
    CLC
    ADC.w #$0008
    LDY.w #$5000
    JSR Overworld_LoadOneBG1FlatMap16

    LDA.b $06
    CLC
    ADC.w #$0009
    LDY.w #$5040
    JSR Overworld_LoadOneBG1FlatMap16

.done
    RTS

.area_80
    LDX.w #$0000              ; Grove Fog
    LDA.b $A0
    CMP.w #$0181
    BEQ .load_bridge

    LDA.l $7EF300
    AND.w #$0040
    BNE .no_authored_bg1
    BRA .load_area_80

.load_bridge
    LDX.w #$0003              ; Bridge Shadow

.load_area_80
    LDY.w #$4000
    JSR Overworld_LoadArea80BG1FlatMap16
    RTS

.no_authored_bg1
    LDA.b $8C
    AND.w #$00FF
    CMP.w #$009F
    BEQ .load_rain

    LDA.w #$0000
    STA.l $7E4000
    LDX.w #$4000
    LDY.w #$4002
    LDA.w #$1FFD
    MVN $7E,$7E

    SEP #$20
    STZ.b $1D
    REP #$20
    RTS

.load_rain
    LDA.l $7EC213
    AND.w #$00FF
    TAX
    SEP #$20
    LDA.l !RainContexts,X
    REP #$20
    AND.w #$00FF
    DEC A
    STA.b $00
    ASL A
    CLC
    ADC.b $00
    TAX

    SEP #$20
    LDA.l !RainFlatMap16Pointers+2,X
    STA.b $0D
    REP #$20

    LDA.l !RainFlatMap16Pointers,X
    TAX
    LDY.w #$4000
    JSR Overworld_CopyOneFlatMap16
    RTS

assert pc() <= !free_space_bank_a0_end
