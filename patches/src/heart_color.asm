; Draw the HUD hearts with BG3 palette 6, whose color 3 the patcher sets to the
; customized heart color. Vanilla draws them with palette 1, whose red is also
; used by the "LIFE" label.
;
; Palette 6 is the text palette: text glyphs use its dark navy color 1 and white
; color 2, leaving color 3 free. The heart tiles use 1 for red, 2 for white, and
; 3 for black, so their indices 1 and 3 are swapped when the HUD graphics are
; loaded. Black then shows as the (darkened) navy.
lorom

incsrc "symbols.inc"

; Free-space range used by this patch.
!free_space_bank_a0_start = $A0811C
!free_space_bank_a0_end = $A08180

; LoadDefaultGraphics decompresses each common UI sheet here, then copies it to
; the current VRAM address. Sheet $6B holds BG3 tiles $80-$FF, including the
; heart tiles $A0-$A4.
!SheetBuffer = $7F4000
!HeartSheet = $6B
!HeartTilesOffset = $0200
!HeartTilesSize = $0050
!SheetSize = $0800

; RTS-terminated sprite-sheet decompressor, and an RTL in the same bank.
!DecompressSpriteLow = $80E7A6
!LoadDefaultGraphicsRtl = $80E37A

; Empty, current-health, and refill-animation heart tilemap words.
org $8DFA1D
    dw $38A2, $38A2, $38A2
    dw $38A2, $38A1, $38A0
    dw $38A3, $38A4, $38A3, $38A0

; Darken palette 6's navy from #000070 to #000040, closer to the black
; it replaces in empty hearts.
org $9BD692
    dw $2000

; LoadDefaultGraphics: replace the upload of sheet $6B.
org $80E36F
hook_upload_ui_sheet:
    JSL UploadHeartSheet
    NOP
assert pc() == $80E374

org !free_space_bank_a0_start

; Entry is A8/X8/Y8 with DB = $80, as for DecompressAndCopyManually.
UploadHeartSheet:
    ; Run hi-jacked instruction
    LDY.b #!HeartSheet

    ; The decompressor returns with RTS, so enter it with a stacked return to
    ; LoadDefaultGraphics's RTL, which returns here.
    PHK
    PEA.w .decompressed-1
    PEA.w !LoadDefaultGraphicsRtl-1
    JML !DecompressSpriteLow

.decompressed
    REP #$30

    ; XOR each heart row's plane 0 into plane 1, swapping pixel indices 1 and 3.
    LDX.w #!HeartTilesSize-2
.next_row
    LDA.l !SheetBuffer+!HeartTilesOffset,X
    AND.w #$00FF
    XBA
    EOR.l !SheetBuffer+!HeartTilesOffset,X
    STA.l !SheetBuffer+!HeartTilesOffset,X
    DEX
    DEX
    BPL .next_row

    ; Copy the sheet to VRAM, as DecompressAndCopyManually does.
    LDX.w #$0000
.next_word
    LDA.l !SheetBuffer,X
    STA.w $2118
    INX
    INX
    CPX.w #!SheetSize
    BNE .next_word

    SEP #$30
    RTL

assert pc() <= !free_space_bank_a0_end
