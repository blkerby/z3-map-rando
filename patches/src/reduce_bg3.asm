; Reduce the shared overworld/dungeon BG3 tilemap from 64x64 to 32x32.
;
; Vanilla keeps the HUD at VRAM $6000 and the item menu at VRAM $6800, then
; reveals the menu by scrolling BG3 upward. With only one VRAM screen block,
; those two images alias each other, so this patch treats its 32 rows as a
; ring: menu rows replace VRAM rows leaving the top while opening, and
; HUD/blank rows replace VRAM rows entering at the bottom while closing.
;
; The ending credits also scroll BG3 vertically. Their existing stripe
; streamer is retained, but its VRAM destinations now wrap within one block.

lorom

!free_space_bank_00_start = $008878
!free_space_bank_00_end = $008888
!free_space_bank_any_start = $27E1E0
!free_space_bank_any_end = $27E400

; One byte of otherwise unused WRAM tells the NMI handler which source image
; supplies the next ring row.
!BG3StreamDirection = $7EC84A
!BG3StreamOpening = $00
!BG3StreamClosing = $01

; Intro_InitializeBackgroundSettings runs when the title-screen intro sets up
; its PPU backgrounds. Change BG3SC from $63 (VRAM $6000, 64x64) to $60
; (VRAM $6000, 32x32).
org $02C284
    db $60

; NMI_UploadTilemap uses WRAM $0116 as an index into
; TilemapUpload_HighBytes. ItemMenu_ClearTilemap and ItemMenu_Initialize select
; entry $22 for their full-buffer uploads; redirect that entry from the
; vanilla menu block at VRAM $6800 to the shared block at VRAM $6000.
org $0098AA
    db $60

; EraseTilemaps_normal enters the shared EraseTilemaps DMA body here during
; force-blank transitions such as loading a file, changing an attract/credits
; scene, or returning from the dungeon map. EraseTilemaps_bg3 and
; EraseTilemaps_dungeonmap also share this code. Vanilla's separate $0800-byte
; low- and high-byte fills clear $0800 tilemap words at VRAM $6000-$67FF:
; the top-left block and unused top-right block.
;
; Reduce each fill to $0400 bytes to clear only the remaining 32x32 block.
org $0083AA
    LDA.w #$0400

; During each vertical blank, NMI_HandleTileUpdates reads WRAM $17 as a
; tile-update mode number and calls the corresponding handler.
; Replace vanilla's unused mode $06 (NMI_TilemapNothing) with our new menu row
; streamer. The handler table holds 16-bit bank-00 addresses, so it points to
; a small bank-00 JSL trampoline.
org $008C8A
    dw BG3StreamNMITrampoline

; independent_tile_type.asm makes these final bytes of vanilla
; GetOverworldTileType unreachable, leaving room for the NMI trampoline.
org !free_space_bank_00_start
BG3StreamNMITrampoline:
    JSL BG3StreamNMI
    RTS
assert pc() <= !free_space_bank_00_end

; ItemMenu_ClearTilemap is state $00, called once immediately after entering
; the item-menu module and before the menu begins opening. It clears the
; menu buffer at WRAM $1000-$17FF, then vanilla uploads that blank menu
; to its separate VRAM block. We keep the WRAM clear but skip the VRAM upload,
; so the shared VRAM block continues to display the HUD.
org $0DDD9E
    BRA +
org $0DDDA7
+

; ItemMenu_Initialize is state $01, called next while the menu is about to open.
; It draws the menu into WRAM $1000-$17FF, then vanilla uploads it to VRAM
; before advancing to state $02 (ItemMenu_Open). We keep the writing
; to WRAM but skip the VRAM upload. ItemMenu_Open will reveal the completed WRAM
; buffer one VRAM row at a time.
org $0DDE4C
    BRA +
org $0DDE55
+

; ItemMenu_Open is state $02 and runs once per frame during the 29-frame
; opening animation. It scrolls 8 pixels per step, so we queue the menu row
; update to bring it into view.
org $0DDE63
    JSL ItemMenuScrollOpening
    NOP
assert pc() == $0DDE68

; UpdateHUD is state $05, called before ItemMenu_Close begins. We replace the
; call to RebuildHUD_update with a call to its subroutine UpdateHUDBuffer.
; This way, it rebuilds the current HUD contents in WRAM but skips requesting
; an immediate VRAM upload, which would overwrite the still-visible menu. 
; RestoreHUDRow consumes this WRAM buffer one VRAM row at a time.
org $0DDFAC
    JSR.w $FBB1 ; UpdateHUDBuffer

; ItemMenu_Close is state $06 and runs once per frame during the 29-frame
; closing animation. Replace its 8-pixel scroll step and queue the HUD or blank
; row that the step brings into view.
org $0DDFC2
    JSL ItemMenuScrollClosing

; NamePlayerTilemap is the static stripe list drawn when the file-selection
; name-entry screen is initialized. Its latter part describes the unused right
; half of vanilla BG3; end the list before it writes into freed VRAM
; $6400-$67FF.
org $0CE911
    db $FF

; Credits_InitializeTheActualCredits sets the first attribution's stripe
; destination before the credits begin scrolling. Start at VRAM row 0 ($6000)
; because there is no second vertical screen block, but skip the initial draw
; so that row is not replaced until the first 8-pixel scroll moves it offscreen.
org $0EE6D3
    dw $6000

org $0EE6E9
    NOP
    NOP
    NOP

; Credits_FadeColorAndBeginAnimating advances the vertical scroll every frame.
; Each 8-pixel boundary queues the next attribution; adjust its line index for
; the initialization-time draw removed above.
org $0EE7BA
    JSL CreditsAdvanceLine
    NOP
    NOP
assert pc() == $0EE7C0

; Credits_AddNextAttribution advances its stripe destination by one row.
; Vanilla alternates between two vertical screen blocks after each 32 rows;
; instead wrap every 32 rows to VRAM $6000.
org $0EE90C
    LDY.w #$6000
    BRA +
org $0EE915
+

org !free_space_bank_any_start

; Called with 16-bit A. Vanilla moves the BG3 scroll value in WRAM $00EA from
; 0 to -232 in 8-pixel steps. Queue the VRAM row at the new top of the ring,
; then reproduce vanilla's comparison flags so the caller advances its menu
; state at -232.
ItemMenuScrollOpening:
    SEP #$20

    LDA.b #!BG3StreamOpening
    STA.l !BG3StreamDirection

    LDA.b #$06
    STA.b $17

    REP #$20

    LDA.b $EA
    CMP.w #$FF18

    SEP #$20
    RTL

; Called with 16-bit A. Move the BG3 scroll value in WRAM $00EA back toward 0,
; queue the VRAM row entering at the bottom, and reproduce the new scroll
; value/flags expected by the caller.
ItemMenuScrollClosing:
    ; Run hi-jacked instructions:
    STA.b $EA

    SEP #$20
    LDA.b #!BG3StreamClosing
    STA.l !BG3StreamDirection

    LDA.b #$06
    STA.b $17

    LDA.b $EA

    RTL

; Convert the scroll position to a zero-based credits line index in WRAM $00CA.
; The vanilla initialization-time line was removed, so the first 8-pixel step
; is line 0.
CreditsAdvanceLine:
    TYA
    LSR A
    LSR A
    LSR A
    DEC A
    STA.b $CA

    RTL

; NMI tile-update mode $06. Convert the pixel scroll in WRAM $00EA to its
; physical BG3 VRAM tile row; masking to 31 performs the 32-row ring wrap for
; negative and positive values.
BG3StreamNMI:
    REP #$30

    LDA.b $EA
    LSR A
    LSR A
    LSR A
    AND.w #$001F
    STA.b $00

    SEP #$20

    LDA.l !BG3StreamDirection
    BNE .closing

    REP #$20

    LDA.b $00
    JMP.w UploadMenuRow

.closing
    REP #$20

    ; The PPU's effective BG vertical coordinate is one pixel ahead, so the
    ; bottom scanline comes from the row 28 rows after the new top row.
    LDA.b $00
    CLC
    ADC.w #$001C
    AND.w #$001F

; Closing path of BG3StreamNMI.
; A = physical row within the BG3 VRAM tilemap.
;
; First erase the entire VRAM row with the same transparent tile used by the
; HUD and menu. If the row intersects the compact HUD buffer in WRAM $7EC700,
; overlay its current contents afterward.
RestoreHUDRow:
    STA.b $00

    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    CLC
    ADC.w #$6000
    STA.b $02
    STA.w $2116

    ; Blank all 32 words in the selected VRAM tilemap row.
    LDA.w #BlankBG3Row
    STA.w $4302

    LDA.w #$1801
    STA.w $4300

    LDA.w #$0040
    STA.w $4305

    SEP #$20

    LDA.b #BlankBG3Row>>16
    STA.w $4304

    LDA.b #$80
    STA.w $2115

    LDA.b #$01
    STA.w $420B

    REP #$20

    LDA.b $00
    CMP.w #$0002
    BCC .done

    ; WRAM $7EC700-$7EC849 holds $14A bytes destined for VRAM $6040 onward:
    ; VRAM rows 2-6 are complete, and row 7 contains its final 10 bytes.
    CMP.w #$0008
    BCS .done

    CMP.w #$0007
    BEQ .partial_row

    SEC
    SBC.w #$0002
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    CLC
    ADC.w #$C700
    STA.w $4302

    LDA.w #$0040
    BRA .upload_hud

.partial_row
    LDA.w #$C840
    STA.w $4302

    LDA.w #$000A

.upload_hud
    STA.w $4305

    ; The blanking DMA advanced the VRAM address, so rewind it before
    ; overlaying the HUD data from WRAM.
    LDA.b $02
    STA.w $2116

    SEP #$20

    LDA.b #$7E
    STA.w $4304

    LDA.b #$01
    STA.w $420B

    REP #$20
.done

BG3StreamNMI_done:
    SEP #$30

    STZ.w $0710

    RTL

; Opening path of BG3StreamNMI.
; A = physical row within the BG3 VRAM tilemap.
;
; VRAM tilemap addresses count 16-bit words, so a 32-tile row advances by $20.
; The WRAM menu buffer is byte-addressed, so its 32 words advance by $40.
UploadMenuRow:
    STA.b $00

    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    CLC
    ADC.w #$6000
    STA.w $2116

    ; WRAM source = $00:1000 + row * $40.
    LDA.b $00
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    CLC
    ADC.w #$1000
    STA.w $4302

    ; DMA 32 consecutive tilemap words to the selected VRAM row.
    LDA.w #$1801
    STA.w $4300

    LDA.w #$0040
    STA.w $4305

    SEP #$20

    STZ.w $4304

    LDA.b #$80
    STA.w $2115

    LDA.b #$01
    STA.w $420B

    REP #$20

    JMP.w BG3StreamNMI_done

BlankBG3Row:
    ; Tilemap word $207F is the transparent space used by the HUD and menu.
    fillword $207F
    fill 32

ReduceBG3End:
assert pc() <= !free_space_bank_any_end
