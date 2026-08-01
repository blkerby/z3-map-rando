; Reduce the CPU work performed during NMI.
;
; The "arbitrary DMA" handler (NMI mode $18) is rewritten
; with a more efficient inner loop. In bg_streamer.asm,
; this mode is used during scrolling to draw newly revealed
; BG2 rows/columns; we also use it for bulk redraws during
; loading and mirror/portal transitions.
;
; We use a new flag $0702 to disable OAM transfer (which
; in vanilla runs unconditionally). This allows us to
; prevent the NMI overruns that otherwise occur during
; mirror/portal and whirlpool use (as happens in vanilla).
; The extra overhead introduced by checking this flag is
; outweighed by a few minor optimizations that we include,
; so the NMI handler as a whole should never take longer
; than it does in vanilla.

lorom

; PPU registers
VMAIN       = $2115 ; VRAM increment mode and address remapping
VMADDR      = $2116 ; 16-bit VRAM word address ($2116-$2117)
W12SEL      = $2123
WOBJSEL     = $2125
TM          = $212C
TMW         = $212E
CGWSEL      = $2130
COLDATA     = $2132

; Controller registers
JOYPAD      = $4016
JOY1L       = $4218
JOY1H       = $4219

; General DMA control
MDMAEN      = $420B ; Writing bit 1 starts DMA channel 1

; DMA channel 1 registers
DMA1MODE    = $4310 ; Transfer mode
DMA1PORT    = $4311 ; Destination PPU register ($18 means $2118/$2119)
DMA1ADDRL   = $4312 ; 16-bit source address ($4312 low, $4313 high)
DMA1ADDRB   = $4314 ; Source bank
DMA1SIZEL   = $4315 ; Transfer size low byte
DMA1SIZEH   = $4316 ; Transfer size high byte

!NMISkipOAM = $0702

!free_space_bank_any_start = $A08380
!free_space_bank_any_end = $A083A0

; NoIRQThread and SwitchThread both copy the same eight adjacent PPU queue
; bytes to adjacent PPU registers. Word writes preserve the byte order while
; halving the number of loads and stores.
org $80814C
    REP #$20

    LDA.b $96                   ; $96 -> W12SEL, $97 -> W34SEL
    STA.w W12SEL

    LDA.b $99                   ; $99 -> CGWSEL, $9A -> CGADSUB
    STA.w CGWSEL

    LDA.b $1C                   ; $1C -> TM, $1D -> TS
    STA.w TM

    LDA.b $1E                   ; $1E -> TMW, $1F -> TSW
    STA.w TMW

    SEP #$20

    LDA.b $98
    STA.w WOBJSEL

    LDA.b $9C
    STA.w COLDATA
    LDA.b $9D
    STA.w COLDATA
    LDA.b $9E
    STA.w COLDATA

    JMP.w $8188

assert pc() <= $808188

org $80823D
    REP #$20

    LDA.b $96                   ; $96 -> W12SEL, $97 -> W34SEL
    STA.w W12SEL

    LDA.b $99                   ; $99 -> CGWSEL, $9A -> CGADSUB
    STA.w CGWSEL

    LDA.b $1C                   ; $1C -> TM, $1D -> TS
    STA.w TM

    LDA.b $1E                   ; $1E -> TMW, $1F -> TSW
    STA.w TMW

    SEP #$20

    LDA.b $98
    STA.w WOBJSEL

    LDA.b $9C
    STA.w COLDATA
    LDA.b $9D
    STA.w COLDATA
    LDA.b $9E
    STA.w COLDATA

    JMP.w $8279

assert pc() <= $808279

; Process each stable auto-joypad result directly instead of copying both
; bytes through $00/$01 and then loading them again.
org $8083D1
ReadJoypad:
    STZ.w JOYPAD

    LDA.w JOY1L
    STA.b $F2
    TAY
    EOR.b $FA
    AND.b $F2
    STA.b $F6
    STY.b $FA

    LDA.w JOY1H
    STA.b $F0
    TAY
    EOR.b $F8
    AND.b $F0
    STA.b $F4
    STY.b $F8

    RTS

assert pc() <= $8083F8

; OAM normally uploads every frame. Only an explicit one-shot request in
; documented free WRAM may skip it; $0710 retains its separate vanilla role
; of suppressing the normal graphics DMA group.
org $808BCD
return_NMIAfterOAMUpload:

org $808BAA
    JML.l hook_NMIBeforeOAMUpload

org $808C22
HandleArbitraryDMA:
    LDA.b $18
    BEQ return_NMIAfterArbitraryDMA

    REP #$10                    ; 16-bit X/Y; leave A 8-bit.

    STZ.w DMA1ADDRB             ; List and data are in WRAM bank $00.

    LDY.w #$1801
    STY.w DMA1MODE              ; Set DMA1MODE and DMA1PORT

    STZ.w DMA1SIZEH             ; Every entry has an 8-bit length.
    LDX.w #$1100                ; X points at the current header.

    LDY.w $0000,X
    CPY.w #$FFFF
    BEQ .done

.next_chunk
    STY.w VMADDR

    LDA.w $0002,X
    STA.w VMAIN

    LDA.w $0003,X
    STA.w DMA1SIZEL

    INX                         ; Skip the four-byte header.
    INX
    INX
    INX
    STX.w DMA1ADDRL

    LDA.b #$02
    STA.w MDMAEN

    LDX.w DMA1ADDRL
    LDY.w $0000,X
    CPY.w #$FFFF
    BNE .next_chunk

.done
    SEP #$30
    STZ.b $18
    STZ.w $0710
    BRA return_NMIAfterArbitraryDMA

assert pc() <= $808C75
org $808C75
return_NMIAfterArbitraryDMA:

org !free_space_bank_any_start
hook_NMIBeforeOAMUpload:
    LDA.w !NMISkipOAM
    REP #$20
    SEP #$10
    BEQ .upload_oam

    STZ.w !NMISkipOAM           ; Consume the one-shot request.

    STZ.b $15                   ; Run hi-jacked instruction
    JML.l return_NMIAfterOAMUpload

.upload_oam
    JML.l $808BAE               ; Resume at the displaced STZ $15.

assert pc() <= !free_space_bank_any_end
