; Set up a rearranged VRAM layout which allows for a larger overworld BG1/BG2
; character set (960 compared to the vanilla 512). All overworld scenes use
; this layout; dungeons keep their existing layout. The active HUD destination
; doubles as the layout marker: $3C40 means overworld, while $6040 is the
; default gameplay destination.
;
; This requires installing a bunch of hooks to apply conditional logic
; for a VRAM destination depending on whether we are in overworld or not.
; This includes hooking the NMI handler, which is a bit undesirable but
; at this stage may be preferable to a wider overhaul. And the extra cost
; should be offset by the optimizations in nmi_optimize.asm to some extent.

lorom

!free_space_bank_any_start = $A7E38F
!free_space_bank_any_end = $A7FFFF
!free_space_bank_82_start = $82F5C0
!free_space_bank_82_end = $82F5E7

!OverworldHUD = $3C40
!DefaultHUD = $6040

; Module08_00_LoadProperties is the common forced-blank entry for normal and
; special overworld loads. Select and clear the relocated layout before any
; animated or static background graphics are loaded, then reproduce the
; displaced color-math setup.
org $8282D1
    JSL Module08SelectOverworldVRAM
assert pc() == $8282D5

; The Triforce room and each overworld credits scene bypass module $08.
; Replace their vanilla tilemap clears with the relocated-layout clear before
; they load background graphics.
org $828506
    JSL SelectAndClearOverworldVRAMLong
assert pc() == $82850A

org $829F4A
    JSL SelectAndClearOverworldVRAMLong
assert pc() == $829F4E

; The scrolling credits background is also overworld-based, but its clear uses
; the distinct blank tiles selected by EraseTilemaps_bg3.
org $8EE649
    JSL SelectAndClearCreditsOverworldVRAMLong
assert pc() == $8EE64D

; Agahnim's mirror warp builds its overworld destination without passing
; through module $08. Switch layouts once the effect reaches that load step.
org $829D5C
    JSR Module15LoadOverworld
assert pc() == $829D5F

; Both the ordinary world map and flute map pass through this forced-blank
; restoration step before reloading overworld graphics.
org $8ABBF9
    JSL WorldMapSelectOverworldVRAM
assert pc() == $8ABBFD

; InitializeTilesets normally begins its eight background sheets at $2000.
; Choose $0000 for the final overworld layout and retain $2000 elsewhere.
org $80E259
    JML InitializeTilesetsSelectBGVRAM
    NOP
assert pc() == $80E25E

; Animated overworld tiles follow the active overworld layout.
org $80D3FC
    JSL SelectAnimatedOverworldVRAM
    NOP
    NOP
assert pc() == $80D402

; NMI mode $01 uses tilemap-upload index $22 for the item and bottle menus.
; Its destination follows the active BG3 tilemap rather than a fixed table
; byte. Other indexes retain the vanilla table.
org $808CB0
    JML NMISelectTilemapUploadDestination
    NOP
    NOP
assert pc() == $808CB6

; Rebase overworld background-character transfer modes. These entry hooks are
; separate because modes $09 and $0A do not use the later common DMA entry.
org $808EE7
    JML NMIUpdateBGChar3And4SelectVRAM
    NOP
assert pc() == $808EEC

org $808F16
    JML NMIUpdateBGChar5And6SelectVRAM
    NOP
assert pc() == $808F1B

; Modes $0E-$11 and OBJ modes $13-$14 share this DMA entry. The helper only
; rebases destinations below $4000, so OBJ transfers remain unchanged.
org $808FC9
    JML NMISelectBGCharacterDestination
    NOP
    NOP
assert pc() == $808FCF

; Message-window positions are offsets within the active gameplay BG3
; tilemap. Replace both fixed $6000-based lookups with a shared selector.
org $8EF0ED
    JSL RenderTextSelectBG3Position
    NOP
    NOP
assert pc() == $8EF0F3

org $8EFBDF
    JSL RenderTextSelectBG3Position
    NOP
    NOP
assert pc() == $8EFBE5

org !free_space_bank_82_start
Module15LoadOverworld:
    JSL SelectOverworldVRAM
    JMP.w $E207                 ; Tail-call LoadOverworldFromUnderworld.
assert pc() <= !free_space_bank_82_end

org !free_space_bank_any_start

Module08SelectOverworldVRAM:
    JSR SelectAndClearOverworldVRAM

    ; Run hi-jacked instructions:
    LDA.b #$82
    STA.b $99
    RTL

WorldMapSelectOverworldVRAM:
    JSL $80893D                 ; EnableForceBlank
    JSR SelectAndClearOverworldVRAM
    RTL

SelectAndClearOverworldVRAMLong:
    JSR SelectAndClearOverworldVRAM
    RTL

SelectAndClearCreditsOverworldVRAMLong:
    JSR SelectAndClearCreditsOverworldVRAM
    RTL

; Select the final layout and clear only its three tilemap regions. Callers
; are forced blank; no active-display path may call this routine.
SelectAndClearOverworldVRAM:
    PHP
    REP #$30
    PHX
    LDX.w #$0000
    BRA SelectAndClearOverworldVRAMCommon

SelectAndClearCreditsOverworldVRAM:
    PHP
    REP #$30
    PHX
    LDX.w #$0001

SelectAndClearOverworldVRAMCommon:
    PHY
    LDA.b $00
    PHA
    LDA.b $02
    PHA
    LDA.b $04
    PHA
    STX.b $04

    JSR SelectOverworldVRAM

    LDA.b $04
    BNE .credits_bg12
    LDY.w #BlankBG12Tile
    BRA .clear_bg12

.credits_bg12
    LDY.w #BlankCreditsBG12Tile

.clear_bg12
    LDA.w #$6000
    LDX.w #$1000
    JSR ClearVRAMRegion

    LDA.b $04
    BNE .credits_bg3
    LDY.w #BlankBG3Tile
    BRA .clear_bg3

.credits_bg3
    LDY.w #BlankCreditsBG3Tile

.clear_bg3
    LDA.w #$3C00
    LDX.w #$0400
    JSR ClearVRAMRegion

    PLA
    STA.b $04
    PLA
    STA.b $02
    PLA
    STA.b $00
    PLY
    PLX

    PLP
    RTS

; Select the final registers without clearing VRAM. The Agahnim transition
; uses this while its destination is hidden, then fully replaces each region.
SelectOverworldVRAM:
    PHP
    SEP #$20
    LDA.b #$69                  ; BG1: $6800, 64x32
    STA.w $2107
    LDA.b #$61                  ; BG2: $6000, 64x32
    STA.w $2108
    LDA.b #$3C                  ; BG3: $3C00, 32x32
    STA.w $2109
    STZ.w $210B                 ; BG1/BG2 characters begin at $0000.

    REP #$20
    LDA.w #!OverworldHUD
    STA.w $0219
    PLP
    RTS

; Clear X VRAM words beginning at A. Y points to a two-byte fill word in this
; bank. As in vanilla EraseTilemaps, separate fixed-source DMAs fill the low
; and high byte of every tilemap word.
ClearVRAMRegion:
    STA.b $00
    STX.b $02

    STZ.w $2115
    STA.w $2116

    LDA.w #$1808
    STA.w $4310
    STY.w $4312
    LDA.b $02
    STA.w $4315

    SEP #$20
    LDA.b #!free_space_bank_any_start>>16
    STA.w $4314
    LDA.b #$02
    STA.w $420B

    REP #$20
    LDA.b $00
    STA.w $2116
    LDA.w #$1908
    STA.w $4310
    INY
    STY.w $4312
    LDA.b $02
    STA.w $4315

    SEP #$20
    LDA.b #$80
    STA.w $2115
    LDA.b #$02
    STA.w $420B

    REP #$20
    RTS

BlankBG12Tile:
    dw $01EC
BlankBG3Tile:
    dw $007F
BlankCreditsBG12Tile:
    dw $007F
BlankCreditsBG3Tile:
    dw $0188

; Tail of InitializeTilesets' displaced background-VRAM setup.
InitializeTilesetsSelectBGVRAM:
    REP #$30
    LDA.w $0219
    CMP.w #!OverworldHUD
    BNE .default

    LDA.w #$0000
    BRA .store

.default
    LDA.w #$2000

.store
    JML $80E25E

; Tail of NMI_UploadTilemap's displaced destination lookup.
NMISelectTilemapUploadDestination:
    LDX.w $0116
    CPX.b #$22
    BNE .table

    LDA.w $021A                ; High byte of $3C40 or $6040.
    JML $808CB6

.table
    LDA.w $9888,X              ; TilemapUpload_HighBytes
    JML $808CB6

NMIUpdateBGChar3And4SelectVRAM:
    REP #$20
    LDA.w #$2C00
    JSR RebaseBGCharacterVRAM
    JML $808EEC

NMIUpdateBGChar5And6SelectVRAM:
    REP #$20
    LDA.w #$3400
    JSR RebaseBGCharacterVRAM
    JML $808F1B

NMISelectBGCharacterDestination:
    JSR RebaseBGCharacterVRAM
    STA.w $2116
    LDA.w #$0000               ; Low word of source $7F0000.
    JML $808FCF

SelectAnimatedOverworldVRAM:
    LDA.w #$3C00
    JSR RebaseBGCharacterVRAM
    STA.w $0134
    RTL

; A.w: VRAM destination. Move shared BG character writes down by $2000 only
; while the final overworld layout is active. OBJ and BG3 destinations are at
; or above $4000 and remain unchanged.
RebaseBGCharacterVRAM:
    PHA
    LDA.w $0219
    CMP.w #!OverworldHUD
    BNE .unchanged

    PLA
    CMP.w #$4000
    BCS .done

    SEC
    SBC.w #$2000
.done
    RTS

.unchanged
    PLA
    RTS

; RenderText_TextPosition contains the two vanilla positions relative to the
; default $6040 HUD destination. Preserve those offsets and add the active
; gameplay destination, defaulting safely when no save has initialized $0219.
RenderTextSelectBG3Position:
    LDA.l $0EFD3E,X
    SEC
    SBC.w #!DefaultHUD
    STA.w $1CD2

    LDA.w $0219
    CMP.w #!OverworldHUD
    BEQ .add_offset
    LDA.w #!DefaultHUD

.add_offset
    CLC
    ADC.w $1CD2
    STA.w $1CD2
    RTL

assert pc() <= !free_space_bank_any_end
