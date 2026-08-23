; Select generated overworld backdrop colors for every area-loading path.
lorom

incsrc "symbols.inc"

!free_space_bank_a0_start = $A0A400
!free_space_bank_a0_end = $A0A620
!ThemeBackgroundColors = $A09E00
!AreaBackgroundSettingsOffset = 24
!GeneratedBG1Enabled = $BFE406
!BackgroundSettings = $7ECC72

org $8CFF91
hook_SetOverworldBackgroundColor:
    JML SetThemeOverworldBackgroundColor
    NOP
assert pc() == $8CFF96
org $8CFF96
return_SetOverworldBackgroundColor:

org $8CFFC3
hook_SetOverworldBackgroundColorCacheOnly:
    JML SetThemeOverworldBackgroundColorCacheOnly
    NOP
assert pc() == $8CFFC8
org $8CFFC8
return_SetOverworldBackgroundColorCacheOnly:

org $8BFEB5
hook_SetOverworldFixedColorAndScrollBackdrop:
    JML SetThemeOverworldFixedColorAndScrollBackdrop
assert pc() == $8BFEB9

org $8BFF0B
return_SetOverworldFixedColorWithoutSubscreen:

org $80F2D7
hook_PreserveGeneratedBackdropDuringSpotlight:
    JML PreserveGeneratedBackdropDuringSpotlight
assert pc() == $80F2DB

org $829999
hook_FinishGeneratedSpotlight:
    JML FinishGeneratedSpotlight
    NOP
assert pc() == $82999E

org !free_space_bank_a0_start

SetThemeOverworldBackgroundColor:
    ; Run hi-jacked instructions:
    REP #$30
    LDX.w #$2669

    LDA.b $8A
    CMP.w #$00A0
    BCS .vanilla
    ASL A
    TAX
    LDA.l !ThemeBackgroundColors,X
    BMI .vanilla
    TAX
    JML $8CFF69

.vanilla
    JML return_SetOverworldBackgroundColor

SetThemeOverworldBackgroundColorCacheOnly:
    ; Run hi-jacked instructions:
    REP #$30
    LDX.w #$2669

    LDA.b $8A
    CMP.w #$00A0
    BCS .vanilla
    ASL A
    TAX
    LDA.l !ThemeBackgroundColors,X
    BMI .vanilla
    TAX
    JML $8CFF71

.vanilla
    JML return_SetOverworldBackgroundColorCacheOnly

PreserveGeneratedBackdropDuringSpotlight:
    ; Run hi-jacked instructions:
    LDA.b $1B
    BNE .indoors

    LDA.l !GeneratedBG1Enabled
    BEQ .black
    LDA.l !BackgroundSettings
    CMP.b #$02
    BNE .black

    LDA.b #$A2                ; Clip and disable color math outside the iris.
    STA.b $99
    JML $80F2E7

.indoors
    JML $80F2E7

.black
    JML $80F2DB

FinishGeneratedSpotlight:
    LDA.b $1B
    BNE .vanilla
    LDA.l !GeneratedBG1Enabled
    BEQ .vanilla
    LDA.l !BackgroundSettings
    CMP.b #$FF
    BEQ .vanilla

    CMP.b #$02
    BNE .generated
    LDA.b #$82                ; Restore ordinary overworld color math.
    STA.b $99

.generated
    JML $8299CB

.vanilla
    ; Run hi-jacked instructions:
    REP #$30
    LDX.w #$4C26

    JML $82999E

SetThemeOverworldFixedColorAndScrollBackdrop:
    TXA                         ; Run hi-jacked instruction
    PHA
    LDA.b $8A
    CMP.w #$00A0
    BCC +
    JMP .vanilla
    +
    ASL A
    TAX
    LDA.l !ThemeBackgroundColors,X
    BPL +
    JMP .vanilla
    +
    PHA
    PHY
    LDA.b $00
    PHA
    LDA.b $02
    PHA
    LDA.b $04
    PHA
    LDA.b $06
    PHA

    SEP #$20
    LDA.b $8A
    JSR.w !ResolveOverworldAreaRecord
    LDY.w #!AreaBackgroundSettingsOffset
    LDA.b [$00],Y
    REP #$20
    AND.w #$00FF
    TAX
    PLA
    STA.b $06
    PLA
    STA.b $04
    PLA
    STA.b $02
    PLA
    STA.b $00
    CPX.w #$0002
    BEQ .backdrop
    CPX.w #$0001             ; Carry set selects the half-add subscreen.

    PLY
    PLA
    PLX
    BRA .store

.backdrop
    PLY
    PLA
    PLX
    TAX

    AND.w #$001F
    ORA.w #$0020
    SEP #$20
    STA.b $9C
    REP #$20

    TXA
    LSR A
    LSR A
    LSR A
    LSR A
    LSR A
    AND.w #$001F
    ORA.w #$0040
    SEP #$20
    STA.b $9D
    REP #$20

    TXA
    XBA
    LSR A
    LSR A
    AND.w #$001F
    ORA.w #$0080
    SEP #$20
    STA.b $9E
    REP #$20

    LDA.w #$0000
    STA.l $7EC500
    STA.l $7EC300
    STA.l $7EC540
    STA.l $7EC340
    JML $8BFF9D

.vanilla
    PLA
    TAX
    TXA
    STA.l $7EC500
    JML $8BFEBA

.store
    STA.l $7EC500
    STA.l $7EC300
    STA.l $7EC540
    STA.l $7EC340

    LDA.w #$4020
    STA.b $9C
    LDA.w #$8040
    STA.b $9D

    BCS .enable_subscreen
    JML return_SetOverworldFixedColorWithoutSubscreen

.enable_subscreen
    JML $8BFF9D

assert pc() <= !free_space_bank_a0_end
assert !ThemeBackgroundColors+$0140 <= !free_space_bank_a0_start
