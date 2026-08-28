; Select generated overworld backdrop colors for every area-loading path.
lorom

incsrc "symbols.inc"

; Free-space range used by this patch.
!free_space_bank_a0_start = $A0A400
!free_space_bank_a0_end = $A0A620

; Generated theme data and active background state.
!ThemeBackgroundColors = $A09E00
!AreaBackgroundSettingsOffset = 24
!GeneratedBG1Enabled = $BFE406
!BackgroundSettings = $7ECC72

; Replace the full background-color selector, retaining its vanilla tail for
; areas without a generated color.
org $8CFF91
hook_SetOverworldBackgroundColor:
    JML SetThemeOverworldBackgroundColor
    NOP
assert pc() == $8CFF96
org $8CFF96
return_SetOverworldBackgroundColor:

; Apply the same selection to the cache-only area-loading path.
org $8CFFC3
hook_SetOverworldBackgroundColorCacheOnly:
    JML SetThemeOverworldBackgroundColorCacheOnly
    NOP
assert pc() == $8CFFC8
org $8CFFC8
return_SetOverworldBackgroundColorCacheOnly:

; Replace the backdrop portion of Overworld_SetFixedColAndScroll.
org $8BFEB5
hook_SetOverworldFixedColorAndScrollBackdrop:
    JML SetThemeOverworldFixedColorAndScrollBackdrop
assert pc() == $8BFEB9

; This vanilla tail finishes without enabling BG1 on the subscreen.
org $8BFF0B
return_SetOverworldFixedColorWithoutSubscreen:

; Preserve generated backdrop presentation through closing and opening irises.
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

; Use the area's generated BGR555 backdrop when available. Entries with bit 15
; set, and areas outside $00-$9F, retain vanilla's area-based selection.
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

; Select the same generated color for the source-palette cache without
; immediately presenting it.
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

; Fixed-color backdrop mode $02 must retain its generated color while the
; closing iris clips color math. Indoor and other overworld modes stay vanilla.
PreserveGeneratedBackdropDuringSpotlight:
    LDA.b $1B                     ; Run hi-jacked instruction
    BNE .indoors

    LDA.l !GeneratedBG1Enabled
    BEQ .black
    LDA.l !BackgroundSettings
    CMP.b #$02
    BNE .black

    LDA.b #$A2                    ; Clip and disable color math outside the iris.
    STA.b $99
    JML $80F2E7

.indoors
    JML $80F2E7

.black
    JML $80F2DB

; Generated overworld backgrounds already selected their fixed color. Skip
; vanilla's area table and restore normal color math for backdrop mode $02.
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
    LDA.b #$82                    ; Ordinary overworld color math.
    STA.b $99

.generated
    JML $8299CB

.vanilla
    ; Run hi-jacked instructions:
    REP #$30
    LDX.w #$4C26

    JML $82999E

; Select the generated backdrop and BG1 presentation used by
; Overworld_SetFixedColAndScroll. Area-record background modes are:
;   $00: solid CGRAM backdrop, normally without BG1
;   $01: solid CGRAM backdrop with half-add BG1
;   $02: generated fixed-color backdrop behind BG1
;
; ResolveOverworldAreaRecord uses $00-$07 and X/Y, so preserve them while
; reading the background mode. The hook enters with 16-bit A/X/Y.
SetThemeOverworldFixedColorAndScrollBackdrop:
    TXA                           ; Run hi-jacked instruction
    PHA                           ; Preserve vanilla's selected color.

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

    ; Preserve the generated color, caller Y, and resolver scratch.
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

    ; Restore resolver scratch in reverse order.
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
    CPX.w #$0001                  ; Carry set selects the half-add subscreen.

    PLY
    PLA
    PLX
    BRA .store

.backdrop
    PLY
    PLA
    PLX
    TAX                           ; Keep the generated BGR555 color in X.

    ; Convert BGR555 red to a $2132 component byte.
    AND.w #$001F
    ORA.w #$0020
    SEP #$20
    STA.b $9C
    REP #$20

    ; Convert BGR555 green to a $2132 component byte.
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

    ; Convert BGR555 blue to a $2132 component byte.
    TXA
    XBA
    LSR A
    LSR A
    AND.w #$001F
    ORA.w #$0080
    SEP #$20
    STA.b $9E
    REP #$20

    ; Fixed-color backdrop mode uses black in the matching CGRAM entries.
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
    STA.l $7EC500                ; Run hi-jacked instruction
    JML $8BFEBA

.store
    ; Keep source and displayed backdrop entries synchronized.
    STA.l $7EC500
    STA.l $7EC300
    STA.l $7EC540
    STA.l $7EC340

    ; Other modes use black fixed color.
    LDA.w #$4020
    STA.b $9C
    LDA.w #$8040
    STA.b $9D

    BCS .enable_subscreen

    ; $8C is the active overworld overlay; $9F selects the rain overlay.
    LDA.b $8C
    AND.w #$00FF
    CMP.w #$009F
    BEQ .enable_subscreen

    JML return_SetOverworldFixedColorWithoutSubscreen

.enable_subscreen
    JML $8BFF9D

; Keep generated color data below this patch's independently assigned code.
assert pc() <= !free_space_bank_a0_end
assert !ThemeBackgroundColors+$0140 <= !free_space_bank_a0_start
