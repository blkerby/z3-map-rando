; Set up the overworld VRAM layout, which expands the BG1/BG2 character set
; from 512 to 960 tiles. It owns the overworld and default register layouts,
; relocated HUD and tilemap destinations, layout switching, VRAM clearing, and
; rebasing of legacy uploads. Generated graphics and palette loading live in
; overworld_assets.asm.

lorom

incsrc "symbols.inc"

!free_space_bank_a0_start = $A0AC00
!free_space_bank_a0_end = $A0AF00
!free_space_bank_82_start = $82F5C0
!free_space_bank_82_end = $82F5CC

!DefaultHUD = $6040

; Module08_00_LoadProperties, modules $08/$0A submodule $00: common
; forced-blank entry for normal and special overworld loads. Select and clear
; the relocated layout before any animated or static background graphics are
; loaded, then reproduce the displaced color-math setup.
org $8282D1
    JSL hook_Module08LoadProperties
assert pc() == $8282D5

; Credits_LoadOverworldScene_PrepGFX, module $1A while loading an overworld
; credits scene: replace its vanilla tilemap clear with the relocated-layout
; credits clear before it loads background graphics.
org $828506
    JSL hook_CreditsOverworldTilemapClear
assert pc() == $82850A

; Module19_02_LoadMusicAndScreen, module $19 submodule $02 while loading the
; Triforce room: replace its vanilla tilemap clear with the relocated-layout
; clear before it loads the special overworld screen, and discard any pending
; HUD upload so the scene's BG3 remains blank.
org $829F4A
    JSL hook_TriforceRoomBeforeSpecialOverworldLoad
assert pc() == $829F4E

; Credits_InitializeTheActualCredits, module $1A submodule $20: replace its
; EraseTilemaps_bg3 call with the relocated-layout clear, retaining the
; distinct blank tiles used by the scrolling credits background.
org $8EE649
    JSL hook_CreditsOverworldTilemapClear
assert pc() == $8EE64D

; Credits_InitializeTheActualCredits initially selects vanilla's vertical BG2
; tilemap at $1000. Select its rebased static 32x64 map at $6000 instead.
org $8EE6F9
    LDA.b #$62
assert pc() == $8EE6FB

; Credits_FadeColorAndBeginAnimating later selects vanilla BG1/BG2 tilemaps
; together. Point both layers at the rebased shared static tilemap.
org $8EE783
    LDY.w #$6263
assert pc() == $8EE786

; Credits_AddEndingSequenceText, module $1A while preparing an overworld or
; underworld vignette: derive its full BG3 fill destination from the active
; layout instead of always targeting vanilla VRAM $6000.
org $8EECE7
    JSL hook_CreditsBeforeEndingTextBlankFill
    NOP
    NOP
assert pc() == $8EECED

; Credits_AddEndingSequenceText, while copying each text-stripe header: rebase
; its vanilla BG3 destination only when the overworld layout is active.
org $8EED0E
    JSL hook_CreditsBeforeEndingTextStripeCopy
assert pc() == $8EED12

; SetTargetOverworldWarpToPyramid, module $15 during Agahnim's mirror-warp
; transition: switch layouts when it loads the overworld destination without
; passing through module $08.
org $829D5C
hook_Module15OverworldLoad:
    JSR LoadModule15Overworld
assert pc() == $829D5F

; InitializeTilesets, shared graphics-loading routine with no fixed module or
; submodule: normally begins its eight background sheets at $2000. Choose
; $0000 for the final overworld layout and retain $2000 elsewhere.
org $80E259
    JML hook_InitializeTilesetsBeforeBGSetup
    NOP
assert pc() == $80E25E


; AnimateMirrorWarp_LoadPyramidIfAga: hold its penultimate delay step until the
; fade reaches white, then stage the relocated HUD after the old layout's final
; animated-tile write. The following step can safely switch VRAM layouts.
org $80D8DC
hook_Module15MirrorWarpBeforeLayoutSwitch:
    JML PrepareModule15HUDBeforeLayoutSwitch
assert pc() == $80D8E0


; Intro_InitializeDefaultGFX, intro initialization after startup or
; save-and-quit: restore the default layout before the routine clears and
; rebuilds VRAM.
org $8CC208
    JSL hook_IntroDefaultGraphicsBeforeTilemapClear
assert pc() == $8CC20C

; LoadUnderworldEntrance, shared underworld-entry routine during forced blank:
; restore and clear the default layout before the room and HUD are uploaded.
org $82D61A
    JSL hook_LoadUnderworldEntranceBeforeEnvironmentFlag
assert pc() == $82D61E

; NMI_UploadTilemap, NMI request $17=$01: tilemap-upload index $22 is used by
; the item and bottle menus. Make its destination follow the active BG3
; tilemap; other indexes retain the vanilla table.
org $808CB0
    JML hook_NMITilemapUploadDestination
    NOP
    NOP
assert pc() == $808CB6

; First unused tilemap-upload selector: fixed destination for the relocated
; Module $15 BG3 page staged before $0219 switches to the overworld HUD.
org $8098AB
    db $3C

; NMI_UpdateBGChar3and4, NMI request $17=$09: rebase the first half of the
; overworld transition BG-character upload. It has a separate entry because
; it does not use the later common DMA setup.
org $808EE7
    JML hook_NMIUpdateBGChar3And4
    NOP
assert pc() == $808EEC

; NMI_UpdateBGChar5and6, NMI request $17=$0A: rebase the second half of the
; overworld transition BG-character upload. It has a separate entry because
; it does not use the later common DMA setup.
org $808F16
    JML hook_NMIUpdateBGChar5And6
    NOP
assert pc() == $808F1B

; NMI_RunTilemapUpdateDMA, shared by NMI requests $17=$0E-$11 and OBJ requests
; $13-$14: rebase destinations below $4000 for the overworld layout while
; leaving OBJ destinations unchanged.
org $808FC9
    JML hook_NMIBGCharacterDMASetup
    NOP
    NOP
assert pc() == $808FCF

; ParseText_SetWindowPosition, message rendering with no fixed game module:
; interpret its window position as an offset within the active BG3 tilemap
; instead of the fixed vanilla $6000 tilemap.
org $8EF0ED
    JSL hook_RenderTextPosition
    NOP
    NOP
assert pc() == $8EF0F3

; RenderText_SetDefaultWindowPosition, message rendering with no fixed game
; module: apply the same active-BG3 selection to the default window position.
org $8EFBDF
    JSL hook_RenderTextPosition
    NOP
    NOP
assert pc() == $8EFBE5

; RenderText_PostDeathSaveOptions, game-over module $12 submodule $08: replace
; its hard-coded BG3 position with the equivalent position in the active
; gameplay layout.
org $8EEE34
    JSL hook_RenderTextPostDeathSaveOptionsPosition
    BRA $04
assert pc() == $8EEE3A

org !free_space_bank_82_start

LoadModule15Overworld:
    JSL HideModule15Backgrounds
    JSR.w $E207                 ; Run hi-jacked instruction
    RTS

assert pc() <= !free_space_bank_82_end

org !free_space_bank_a0_start

hook_IntroDefaultGraphicsBeforeTilemapClear:
    JSL $80893D                 ; Run hi-jacked instruction
    JSR SelectDefaultVRAM
    RTL

hook_LoadUnderworldEntranceBeforeEnvironmentFlag:
    JSR SelectDefaultVRAM

    ; Remove overworld tilemap data before the underworld room rebuild.
    JSL $80834B                 ; EraseTilemaps_normal

    LDA.b #$01
    STA.b $16                   ; Upload the HUD into the restored BG3 block.

    ; Run hi-jacked instructions:
    LDA.b #$01
    STA.b $1B                   ; Mark the active environment as indoors.

    RTL

hook_Module08LoadProperties:
    JSR SelectAndClearOverworldVRAM

    ; Run hi-jacked instructions:
    LDA.b #$82
    STA.b $99

    RTL

hook_TriforceRoomBeforeSpecialOverworldLoad:
    STZ.b $16                   ; Prevent NMI from restoring the HUD after the clear.
    JSR SelectAndClearOverworldVRAM
    RTL

hook_CreditsOverworldTilemapClear:
    JSR SelectAndClearCreditsOverworldVRAM
    RTL

hook_CreditsBeforeEndingTextBlankFill:
    LDA.w $0219
    XBA
    AND.w #$00FF               ; Convert $3C40/$6040 to $003C/$0060.
    STA.w $1002                ; Run hi-jacked instruction
    RTL

hook_CreditsBeforeEndingTextStripeCopy:
    PHA
    LDA.w $0219
    CMP.w #!OverworldHUD
    BNE .default_layout

    PLA
    SEC
    SBC.w #$0024               ; Rebase encoded $62xx/$63xx to $3Exx/$3Fxx.
    BRA .store

.default_layout
    PLA

.store
    ; Run hi-jacked instructions:
    STA.w $1008,X
    INY

    RTL

; Select the final layout and clear only its three tilemap regions. Callers
; are forced blank; no active-display path may call this routine.
SelectAndClearOverworldVRAM:
    PHP
    REP #$30
    PHX
    PHY

    JSL SelectOverworldVRAM

    LDY.w #BlankBG12Tile
    LDA.w #$6000
    LDX.w #$1000
    JSR ClearVRAMRegion

    LDY.w #BlankBG3Tile
    LDA.w #$3C00
    LDX.w #$0400
    JSR ClearVRAMRegion

    PLY
    PLX
    PLP
    RTS

SelectAndClearCreditsOverworldVRAM:
    PHP
    REP #$30
    PHX
    PHY

    JSL SelectOverworldVRAM

    LDY.w #BlankCreditsBG12Tile
    LDA.w #$6000
    LDX.w #$1000
    JSR ClearVRAMRegion

    LDY.w #BlankCreditsBG3Tile
    LDA.w #$3C00
    LDX.w #$0400
    JSR ClearVRAMRegion

    PLY
    PLX
    PLP
    RTS

; Select the final registers and layout-dependent NMI destinations without
; clearing VRAM. The Agahnim transition uses this while its destination is
; hidden, then fully replaces each region.
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
    LDA.w #$1C00                ; Periodic animated BG characters.
    STA.w $0134
    LDA.w #!OverworldHUD
    STA.w $0219
    PLP
    RTL

; Queue the relocated BG3 tilemap immediately before the layout switch. Start
; with transparent tiles, then overlay the current HUD at its new $3C40
; destination so BG3 can remain visible across the switch.
PrepareModule15OverworldTilemaps:
    PHP
    REP #$30
    PHX

    LDA.w #$207F
    LDX.w #$07FE
.next_word
    STA.w $1000,X
    DEX
    DEX
    BPL .next_word

    LDX.w #$0148
.copy_hud
    LDA.l $7EC700,X
    STA.w $1080,X
    DEX
    DEX
    BPL .copy_hud

    SEP #$20
    LDA.b #$23
    STA.w $0116
    LDA.b #$01
    STA.b $17

    PLX
    PLP
    RTL

; Select the shared non-overworld layout. BG3 uses the reduced 32x32 tilemap
; while BG1/BG2 retain their vanilla underworld and presentation arrangement.
SelectDefaultVRAM:
    PHP
    SEP #$20
    LDA.b #$13                  ; BG1: $1000, 64x64
    STA.w $2107
    LDA.b #$03                  ; BG2: $0000, 64x64
    STA.w $2108
    LDA.b #$60                  ; BG3: $6000, 32x32
    STA.w $2109
    LDA.b #$22                  ; BG1/BG2 characters begin at $2000.
    STA.w $210B
    LDA.b #$00
    STA.l !GeneratedAssetMarker

    REP #$20
    LDA.w #!DefaultHUD
    STA.w $0219
    PLP
    RTS

; Clear X VRAM words beginning at A. Y points to a two-byte fill word in this
; bank. As in vanilla EraseTilemaps, separate fixed-source DMAs fill the low
; and high byte of every tilemap word.
ClearVRAMRegion:
    PHA

    STZ.w $2115
    STA.w $2116

    LDA.w #$1808
    STA.w $4310
    STY.w $4312
    STX.w $4315

    SEP #$20
    LDA.b #BlankBG12Tile>>16
    STA.w $4314
    LDA.b #$02
    STA.w $420B

    REP #$20
    PLA
    STA.w $2116
    LDA.w #$1908
    STA.w $4310
    INY
    STY.w $4312
    STX.w $4315

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
hook_InitializeTilesetsBeforeBGSetup:
    REP #$30                    ; Run hi-jacked instruction

    LDA.w $0219
    CMP.w #!OverworldHUD
    BNE .default

    LDA.w #$0000
    BRA .store

.default
    LDA.w #$2000                ; Run hi-jacked instruction

.store
    JML $80E25E

; Tail of NMI_UploadTilemap's displaced destination lookup.
hook_NMITilemapUploadDestination:
    LDX.w $0116                 ; Run hi-jacked instruction

    CPX.b #$22
    BNE .table

    LDA.w $021A                ; High byte of $3C40 or $6040.
    JML $808CB6

.table
    LDA.w $9888,X               ; Run hi-jacked instruction
    JML $808CB6

hook_NMIUpdateBGChar3And4:
    ; Run hi-jacked instructions:
    REP #$20
    LDA.w #$2C00

    JSR RebaseBGCharacterVRAM
    JML $808EEC

hook_NMIUpdateBGChar5And6:
    ; Run hi-jacked instructions:
    REP #$20
    LDA.w #$3400

    JSR RebaseBGCharacterVRAM
    JML $808F1B

hook_NMIBGCharacterDMASetup:
    JSR RebaseBGCharacterVRAM

    ; Run hi-jacked instructions:
    STA.w $2116
    LDA.w #$0000               ; Low word of source $7F0000.

    JML $808FCF

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
hook_RenderTextPosition:
    LDA.l $0EFD3E,X             ; Run hi-jacked instruction
    SEC
    SBC.w #!DefaultHUD
    STA.w $1CD2                 ; Run hi-jacked instruction

    LDA.w $0219
    CMP.w #!OverworldHUD
    BEQ .add_offset
    LDA.w #!DefaultHUD

.add_offset
    CLC
    ADC.w $1CD2
    STA.w $1CD2
    RTL

hook_RenderTextPostDeathSaveOptionsPosition:
    REP #$20
    LDA.w $0219                 ; Active BG3 tilemap plus the vanilla offset.
    CLC
    ADC.w #$01A8
    STA.w $1CD2
    SEP #$20
    RTL

PrepareModule15HUDBeforeLayoutSwitch:
    LDA.b $10
    CMP.b #$15
    BNE .return

    LDA.w $06BA
    CMP.b #$1F
    BNE .return

    LDA.l $7EC009
    BEQ .prepare

    DEC.w $06BA                 ; Retry this step after the next palette update.
    BRA .return

.prepare
    JSL PrepareModule15OverworldTilemaps

    REP #$20
    LDA.w #$1C00
    STA.w $0134
    LDA.w #!OverworldHUD
    STA.w $0219
    SEP #$20

.return
    STZ.w $0200                 ; Run hi-jacked instruction
    RTL

; Keep the old BG3 HUD visible while module $15 prepares the overworld layout.
; Commit the background mask, BG3 color-math exclusion, and white fixed color
; immediately because heavy frames can overrun NMI and skip its normal writes.
HideModule15Backgrounds:
    PHP
    REP #$20

    LDA.w #$0080
    STA.l !Module15LayerEnable ; Bit 7 marks the outer Module $15 load.

    LDA.b $1C
    AND.w #$0404               ; Keep only BG3; OBJ graphics are being replaced.
    STA.b $1C
    STA.w $212C

    SEP #$20
    LDA.b $9A
    AND.b #$FB                  ; White fixed-color addition would bleach BG3.
    STA.b $9A
    STA.w $2131

    LDA.b #$3F
    STA.b $9C
    STA.w $2132
    LDA.b #$5F
    STA.b $9D
    STA.w $2132
    LDA.b #$9F
    STA.b $9E
    STA.w $2132

    PLP
    RTL

assert pc() <= !free_space_bank_a0_end
