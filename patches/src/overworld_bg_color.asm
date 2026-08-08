; Select generated overworld backdrop colors for every area-loading path.
lorom

incsrc "symbols.inc"

!free_space_bank_a0_start = $A0A400
!free_space_bank_a0_end = $A0A620
!ThemeBackgroundColors = $A09E00
!AreaBackgroundSettingsOffset = 24

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
org $8BFEC6
return_SetOverworldFixedColorAndScrollBackdrop:

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
    PLY
    CPX.w #$0002
    BEQ .backdrop

    PLA
    PLX
    BRA .store

.backdrop
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

.store
    STA.l $7EC500
    STA.l $7EC300
    STA.l $7EC540
    STA.l $7EC340
    JML return_SetOverworldFixedColorAndScrollBackdrop

assert pc() <= !free_space_bank_a0_end
assert !ThemeBackgroundColors+$0140 <= !free_space_bank_a0_start
