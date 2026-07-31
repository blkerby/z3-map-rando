; Preserve BG1's independent horizontal scroll during the mirror wave.
;
; Vanilla points HDMA channels 6 and 7 at the same horizontal-scroll table.
; Since that table is based on BG2 scroll $E2, channel 6 abruptly replaces
; BG1's $E0-based parallax when the mirror starts. Give channel 6 a parallel
; table containing the same wave displacement relative to $E0 instead:
;
;   BG2 scanline scroll = vanilla table value
;   BG1 scanline scroll = vanilla table value + ($E0 - $E2)

lorom

!MirrorBG1Table = $7EBE00

!HDMA6ADDRL = $4362
!HDMA6ADDRB = $4364
!HDMA6ITBLB = $4367

!free_space_bank_82_start = $82F3EE
!free_space_bank_82_end = $82F52F

; InitializeMirrorHDMA
;
; Vanilla has finished filling $1B00-$1CDF from $E2. Replace its final
; "SEP #$20 : LDA #$C0 : STA $9B : RTL" with channel-6 setup, the initial
; BG1 table build, and the same HDMA-enable request.
org $80FE57
    JML MirrorBG1Initialize

; MirrorWarp_BuildWavingHDMATable
;
; AnimateMirrorWarp runs before this parity check and may change $E0 while
; loading the destination. Refresh BG1 even on frames where vanilla leaves
; its wave table unchanged. Updating frames refresh it after finishing $1B00.
org $80FE68
    JML MirrorBG1CheckWavingFrame
    NOP

; The final value still needs to be written to $1B08 and $1B0C before the
; completed vanilla table is translated for BG1.
org $80FF26
    JML MirrorBG1FinishWavingFrame

; MirrorWarp_BuildDewavingHDMATable has the same alternating-frame behavior.
org $80FF33
    JML MirrorBG1CheckDewavingFrame
    NOP

; The dewave routine has multiple branches to this common return. Use the
; adjacent vanilla free space for a same-bank trampoline.
org $80FFB4
    BRA MirrorBG1FinishDewavingTrampoline
    NOP

org $80FFB7
MirrorBG1FinishDewavingTrampoline:
    JML MirrorBG1FinishDewavingFrame

; Obsolete Map32 expansion space in bank $02.
org !free_space_bank_82_start

; Configure channel 6 to read a separate indirect table, build its initial
; values, then reproduce InitializeMirrorHDMA's displaced tail.
MirrorBG1Initialize:
    PHP

    SEP #$20
    LDA.b #MirrorBG1HDMATable>>16
    STA.w !HDMA6ADDRB

    LDA.b #$7E
    STA.w !HDMA6ITBLB

    REP #$30
    LDA.w #MirrorBG1HDMATable
    STA.w !HDMA6ADDRL

    JSR MirrorBG1CopyTable

    PLP
    SEP #$20

    LDA.b #$C0
    STA.b $9B
    RTL

; Preserve vanilla's odd-frame early return, but first account for any BG1
; base-scroll change made by AnimateMirrorWarp.
MirrorBG1CheckWavingFrame:
    LDA.b $1A
    LSR A
    BCC .update

    REP #$30
    JSR MirrorBG1CopyTable
    SEP #$30
    RTL

.update
    JML $80FE6D

; Complete the four identical leading values written by vanilla, then build
; BG1 from the finished wave table.
MirrorBG1FinishWavingFrame:
    STA.w $1B08
    STA.w $1B0C

    JSR MirrorBG1CopyTable
    SEP #$30
    RTL

MirrorBG1CheckDewavingFrame:
    LDA.b $1A
    LSR A
    BCC .update

    REP #$30
    JSR MirrorBG1CopyTable
    SEP #$30
    RTL

.update
    JML $80FF38

MirrorBG1FinishDewavingFrame:
    REP #$30
    JSR MirrorBG1CopyTable
    SEP #$30
    RTL

; Copy all 240 scanline words, preserving the caller's X and direct-page
; scratch word. Sixteen-bit addition naturally wraps like the PPU scroll.
MirrorBG1CopyTable:
    PHX

    LDA.b $00
    PHA

    LDA.b $E0
    SEC
    SBC.b $E2
    STA.b $00

    LDX.w #$01DE

.next
    LDA.w $1B00,X
    CLC
    ADC.b $00
    STA.l !MirrorBG1Table,X

    DEX
    DEX
    BPL .next

    PLA
    STA.b $00

    PLX
    RTS

; Both channels retain vanilla's two 120-line indirect blocks. Channel 6
; reads these pointers from bank $02 and the scanline words from bank $7E.
MirrorBG1HDMATable:
    db $F8 : dw !MirrorBG1Table>>0
    db $F8 : dw (!MirrorBG1Table+$00F0)>>0
    db $00

assert pc() <= !free_space_bank_82_end
