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
!GeneratedAssetMarker = $FF
!OverworldAssetBundlePointers = $AE8000

; Module08_00_LoadProperties is the common forced-blank entry for normal and
; special overworld loads. Select and clear the relocated layout before any
; animated or static background graphics are loaded, then reproduce the
; displaced color-math setup.
org $8282D1
    JSL hook_Module08LoadProperties
assert pc() == $8282D5

; The Triforce room and each overworld credits scene bypass module $08.
; Replace their vanilla tilemap clears with the relocated-layout clear before
; they load background graphics.
org $828506
    JSL hook_OverworldSceneTilemapClear
assert pc() == $82850A

org $829F4A
    JSL hook_OverworldSceneTilemapClear
assert pc() == $829F4E

; The scrolling credits background is also overworld-based, but its clear uses
; the distinct blank tiles selected by EraseTilemaps_bg3.
org $8EE649
    JSL hook_CreditsOverworldTilemapClear
assert pc() == $8EE64D

; Agahnim's mirror warp builds its overworld destination without passing
; through module $08. Switch layouts once the effect reaches that load step.
org $829D5C
    JSR hook_Module15LoadOverworld
assert pc() == $829D5F

; Both the ordinary world map and flute map pass through this forced-blank
; restoration step before reloading overworld graphics.
org $8ABBF9
    JSL hook_WorldMapOverworldRestore
assert pc() == $8ABBFD

; InitializeTilesets normally begins its eight background sheets at $2000.
; Choose $0000 for the final overworld layout and retain $2000 elsewhere.
org $80E259
    JML hook_InitializeTilesetsBeforeBGSetup
    NOP
assert pc() == $80E25E

; Preserve InitializeTilesets' resolved BG sheet cache in $7EC2F8-$7EC2FB.
; Scrolling, mirror, whirlpool, and Agahnim-warp loaders still consume
; that cache until those transitions move to generated descriptors.
org $80E2C2
    JML hook_InitializeTilesetsAfterBGSheetResolution
    NOP
    NOP
assert pc() == $80E2C8

; Mark the shared forced-blank load before InitializeTilesets, then replace
; the generated background assets immediately before the existing background
; color and palette-presentation work.
org $828390
    JSL hook_Module08InitializeTilesets
assert pc() == $828394

org $8283A7
    JSL hook_Module08AfterBGPaletteSelection
assert pc() == $8283AB

; Keep the state, HUD, and sprite portions of the vanilla palette loaders,
; but omit their background writes while the generated full reload is active.
org $82C44A
    JSL hook_OverworldScreenPalettesAfterHUD
assert pc() == $82C44E

org $8CFF4F
    JSL hook_OverworldPalettesAfterSetSelection
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
assert pc() == $8CFF5B

; Animated overworld tiles follow the active overworld layout.
org $80D3FC
    JSL hook_DecompressAnimatedOverworldTilesAfterConversion
    NOP
    NOP
assert pc() == $80D402

; NMI mode $01 uses tilemap-upload index $22 for the item and bottle menus.
; Its destination follows the active BG3 tilemap rather than a fixed table
; byte. Other indexes retain the vanilla table.
org $808CB0
    JML hook_NMITilemapUploadDestination
    NOP
    NOP
assert pc() == $808CB6

; Rebase overworld background-character transfer modes. These entry hooks are
; separate because modes $09 and $0A do not use the later common DMA entry.
org $808EE7
    JML hook_NMIUpdateBGChar3And4
    NOP
assert pc() == $808EEC

org $808F16
    JML hook_NMIUpdateBGChar5And6
    NOP
assert pc() == $808F1B

; Modes $0E-$11 and OBJ modes $13-$14 share this DMA entry. The helper only
; rebases destinations below $4000, so OBJ transfers remain unchanged.
org $808FC9
    JML hook_NMIBGCharacterDMASetup
    NOP
    NOP
assert pc() == $808FCF

; Message-window positions are offsets within the active gameplay BG3
; tilemap. Replace both fixed $6000-based lookups with a shared selector.
org $8EF0ED
    JSL hook_RenderTextPosition
    NOP
    NOP
assert pc() == $8EF0F3

org $8EFBDF
    JSL hook_RenderTextPosition
    NOP
    NOP
assert pc() == $8EFBE5

org !free_space_bank_82_start
hook_Module15LoadOverworld:
    JSL SelectOverworldVRAM
    JMP.w $E207                 ; Run hi-jacked instruction
assert pc() <= !free_space_bank_82_end

org !free_space_bank_any_start

hook_Module08LoadProperties:
    JSR SelectAndClearOverworldVRAM

    ; Run hi-jacked instructions:
    LDA.b #$82
    STA.b $99

    RTL

hook_Module08InitializeTilesets:
    ; $0412 is normally the incremental character-upload cursor ($00-$10).
    ; $FF is therefore available as a transient marker for this forced-blank
    ; load and lets the shared palette routines distinguish this caller.
    LDA.b #!GeneratedAssetMarker
    STA.w $0412

    ; Keep InitializeTilesets' sprite loading and BG sheet-cache setup. The
    ; hook at $80E2C2 returns before it decompresses the static BG sheets.
    JSL $80E1DB                 ; Run hi-jacked instruction
    RTL

; Install generated source palettes and static BG characters after the
; vanilla palette selectors have established the area's palette state. Clear
; the marker before resuming the normal background-color/cache setup.
hook_Module08AfterBGPaletteSelection:
    JSR LoadGeneratedOverworldAssets
    STZ.w $0412

    JSL $8CFF91                 ; Run hi-jacked instruction
    RTL

; OverworldLoadScreensPaletteSet still loads sprite, equipment, Link, and HUD
; palettes. Suppress only its final main-background palette call while the
; generated loader owns BG palette rows 2-7.
hook_OverworldScreenPalettesAfterHUD:
    LDA.w $0412
    CMP.b #!GeneratedAssetMarker
    BEQ .skip

    JSL $9BEEC7                 ; Run hi-jacked instruction
.skip
    RTL

; OverworldPalettesLoader must still resolve palette IDs and load its sprite
; palettes. Suppress only the three auxiliary BG palette writes.
hook_OverworldPalettesAfterSetSelection:
    LDA.w $0412
    CMP.b #!GeneratedAssetMarker
    BEQ .skip

    ; Run hi-jacked instructions:
    JSL $9BEEE8                 ; PaletteLoad_OWBG1
    JSL $9BEF0C                 ; PaletteLoad_OWBG2
    JSL $9BEEA8                 ; PaletteLoad_OWBG3

.skip
    RTL

hook_WorldMapOverworldRestore:
    JSL $80893D                 ; Run hi-jacked instruction
    JSR SelectAndClearOverworldVRAM
    RTL

hook_OverworldSceneTilemapClear:
    JSR SelectAndClearOverworldVRAM
    RTL

hook_CreditsOverworldTilemapClear:
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

; This hook is reached after InitializeTilesets has loaded all sprite sheets
; and resolved the four variable BG sheet IDs into $7EC2F8-$7EC2FB.
hook_InitializeTilesetsAfterBGSheetResolution:
    SEP #$30
    LDA.w $0412
    CMP.b #!GeneratedAssetMarker
    BEQ .skip

    ; Run hi-jacked instructions:
    LDA.b #$07
    STA.b $0F

    JML $80E2C8

.skip
    ; We are replacing InitializeTilesets' tail, so balance its PHB and return
    ; directly to hook_Module08InitializeTilesets' JSL caller.
    PLB
    RTL

; Load the asset record selected by $8A and synchronously process every batch
; in its full-reload sequence. Metadata pointers are little-endian until the
; sequence; sequence and descriptor pointers put the bank first so zero can
; terminate each list.
;
; Direct-page scratch:
;   $00-$02: current asset-record or batch-sequence pointer
;   $03-$05: current batch pointer
;   $06-$07: 16-bit copy of the 8-bit screen ID
; DMA channel 1 is scratch during this forced-blank load. Flags, X, and Y are
; preserved for the surrounding module code.
LoadGeneratedOverworldAssets:
    PHP
    REP #$10
    PHX
    PHY

    SEP #$20
    LDA.b $8A
    STA.b $06
    STZ.b $07

    ; Each of the 256 screen entries is a packed 24-bit pointer, so X = 3*$8A.
    REP #$20
    LDA.b $06
    ASL A
    CLC
    ADC.b $06
    TAX

    ; The top-level table uses ordinary little-endian 24-bit pointers because
    ; every entry has a fixed size and therefore needs no terminator.
    SEP #$20
    LDA.l !OverworldAssetBundlePointers,X
    STA.b $00
    LDA.l !OverworldAssetBundlePointers+1,X
    STA.b $01
    LDA.l !OverworldAssetBundlePointers+2,X
    STA.b $02

    ; The first three bytes of the 18-byte asset record point to its full-load
    ; batch sequence. The remaining transition pointers are reserved for
    ; later 9B checkpoints.
    LDY.w #$0000
    LDA.b [$00],Y
    STA.b $03
    INY
    LDA.b [$00],Y
    STA.b $04
    INY
    LDA.b [$00],Y
    STA.b $05

    LDA.b $03
    STA.b $00
    LDA.b $04
    STA.b $01
    LDA.b $05
    STA.b $02
    BEQ .done                  ; A zero bank means there is no full-load list.

    ; A sequence is [bank, address low, address high] repeated, followed by a
    ; single zero bank. Keep Y as its cursor while each batch uses its own Y.
    LDY.w #$0000
.next_batch
    LDA.b [$00],Y
    BEQ .done
    STA.b $05
    INY
    LDA.b [$00],Y
    STA.b $03
    INY
    LDA.b [$00],Y
    STA.b $04
    INY

    ; ProcessGeneratedAssetBatch consumes Y, so preserve the sequence cursor.
    PHY
    JSR ProcessGeneratedAssetBatch
    PLY
    BRA .next_batch

.done
    PLY
    PLX
    PLP
    RTS

; Process one batch containing a null-terminated palette list followed by a
; null-terminated character list. Each descriptor is:
;   source bank, source address low, source address high, destination row
; Palette rows are 32 bytes; character rows are 16 4bpp tiles (512 bytes).
; Input: $03-$05 = batch pointer. Clobbers A, Y, and DMA channel 1.
ProcessGeneratedAssetBatch:
    LDY.w #$0000

.next_palette
    ; Load the ROM source directly into DMA channel 1's A-bus address.
    LDA.b [$03],Y
    BEQ .palette_done
    STA.w $4314
    INY
    LDA.b [$03],Y
    STA.w $4312
    INY
    LDA.b [$03],Y
    STA.w $4313
    INY
    LDA.b [$03],Y
    INY

    ; Convert destination row n to $7EC300 + n*$20. WMDATA makes the ROM-to-
    ; WRAM transfer direct; vanilla presentation code later derives $7EC500
    ; and schedules the CGRAM upload appropriate to this module.
    REP #$20
    AND.w #$00FF
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    CLC
    ADC.w #$C300
    STA.w $2181
    LDA.w #$8000              ; DMA mode 0 to WMDATA.
    STA.w $4310
    LDA.w #$0020              ; One complete 16-color palette row.
    STA.w $4315

    SEP #$20
    LDA.b #$7E
    STA.w $2183               ; Complete the $7EC300 WRAM destination.
    LDA.b #$02
    STA.w $420B               ; Run DMA channel 1 while display is blank.
    BRA .next_palette

.palette_done
    INY                       ; Skip the palette list's one-byte terminator.
    LDA.b #$80
    STA.w $2115               ; Increment VRAM after writes to $2119.

.next_character
    ; Parse the character descriptor's ROM source into the same DMA channel.
    LDA.b [$03],Y
    BEQ .done
    STA.w $4314
    INY
    LDA.b [$03],Y
    STA.w $4312
    INY
    LDA.b [$03],Y
    STA.w $4313
    INY
    LDA.b [$03],Y
    INY

    ; Character destination row n begins at VRAM word n*$100. The low byte is
    ; therefore zero and the descriptor row is the VMADDR high byte.
    STZ.w $2116
    STA.w $2117

    REP #$20
    LDA.w #$1801              ; DMA mode 1 to VMDATA.
    STA.w $4310
    LDA.w #$0200              ; Sixteen 32-byte 4bpp characters.
    STA.w $4315

    SEP #$20
    LDA.b #$02
    STA.w $420B               ; Run DMA channel 1 while display is blank.
    BRA .next_character

.done
    RTS

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

hook_DecompressAnimatedOverworldTilesAfterConversion:
    LDA.w #$3C00                ; Run hi-jacked instruction
    JSR RebaseBGCharacterVRAM
    STA.w $0134                 ; Run hi-jacked instruction
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

assert pc() <= !free_space_bank_any_end
