lorom

!free_space_bank_any_start = $A0A400
!free_space_bank_any_end = $A0A620
!ThemeBackgroundColors = $A09E00

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

org !free_space_bank_any_start

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
    BCS .vanilla
    ASL A
    TAX
    LDA.l !ThemeBackgroundColors,X
    BMI .vanilla
    PLX
    BRA .store

.vanilla
    PLA
    TAX

.store
    STA.l $7EC500
    STA.l $7EC300
    STA.l $7EC540
    STA.l $7EC340
    JML return_SetOverworldFixedColorAndScrollBackdrop

assert pc() <= !free_space_bank_any_end
assert !ThemeBackgroundColors+$0140 <= !free_space_bank_any_start
