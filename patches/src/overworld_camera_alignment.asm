; Align the source camera before destination setup replaces its Map16 data.
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0F580
!free_space_bank_a0_end = $A0F900
!free_space_bank_82_start = $82F720
!free_space_bank_82_end = $82F724

; HandleOverworldTransitions has 8-bit A/X/Y, X=direction bit and Y=direction
; table offset. $00 holds the same direction bit; $02-$05 are scratch.
org $82A932
hook_prepare_overworld_transition:
    JML PrepareAlignedOverworldTransition

; Ordinary transitions skip phase $05; use it to hold source-side alignment.
; Mosaic phase $13 still dispatches to vanilla StartOverworldScrollTransition.
org $82A30E
    dw hook_align_overworld_transition

; OverworldScrollTransition has already saved DB and selected bank $82.
; Preserve X/Y and widths, then run its displaced frame counter/direction load.
org $82BF3C
hook_begin_overworld_transition_scroll:
    JSL SnapAlignedOverworldCamera
    NOP
    NOP

org !free_space_bank_82_start
hook_align_overworld_transition:
    JML AlignOverworldTransitionCamera
assert pc() <= !free_space_bank_82_end

org !free_space_bank_a0_start
PrepareAlignedOverworldTransition:
    REP #$30
    PHX
    PHY
    LDA.w #$0000
    STA.l !TransitionCameraSnap

    LDA.b $8A
    AND.w #$003F
    BEQ .ready                  ; Keep mosaic transitions unchanged.

    TYX
    LDA.w $0700
    CLC
    ADC.l $82A72B,X
    LSR A
    TAX
    LDA.l !SlotToArea,X
    AND.w #$00FF
    BEQ .ready
    TAX
    LDA.l $82A73B,X             ; Destination large/small flag.
    AND.w #$00FF
    PHA
    TXA
    JSL !GetPhysicalAreaAnchor
    ASL A
    TAX

    CPY.w #$0004
    BCS .vertical_transition
    LDA.l $82A7BB,X
    STA.b $04
    PLA
    BEQ .small_y
    LDA.w #$031E
    BRA .have_y_span
.small_y
    LDA.w #$011E
.have_y_span
    LDX.w #$0006
    BRA .check_bounds

.vertical_transition
    LDA.l $82A83B,X
    STA.b $04
    PLA
    BEQ .small_x
    LDA.w #$0300
    BRA .have_x_span
.small_x
    LDA.w #$0100
.have_x_span
    LDX.w #$0000

.check_bounds
    CLC
    ADC.b $04
    CMP.b $E2,X
    BCC .align
    LDA.b $04
    CMP.b $E2,X
    BCC .ready
    BEQ .ready

.align
    STA.l !TransitionCameraTarget
    PLY
    PLX
    STY.w $0418                ; Retain the original direction table offset.
    STX.w $0410
    SEP #$30
    LDA.b #$05
    STA.b $11
    JML AlignOverworldTransitionCamera

.ready
    PLY
    PLX
    SEP #$30

    ; Run hi-jacked instructions:
    STY.b $02
    STZ.b $03

    JML $82A936

AlignOverworldTransitionCamera:
    REP #$30
    LDX.w #$0000
    LDA.w $0418
    CMP.w #$0004
    BCS .have_axis
    LDX.w #$0006
.have_axis
    ; Finish this frame's player-driven camera movement before aligning.
    CPX.w #$0006
    BEQ .check_delta_y
    LDA.l !BackgroundDeltaX
    BRA .check_delta
.check_delta_y
    LDA.l !BackgroundDeltaY
.check_delta
    AND.w #$00FF
    BNE .wait

    LDA.l !TransitionCameraTarget
    SEC
    SBC.b $E2,X
    BPL .positive
    CMP.w #$FFF8
    BCS .ready
    LDA.w #$FFF8
    BRA .move
.positive
    CMP.w #$0009
    BCC .ready
    LDA.w #$0008
.move
    STA.b $04
    CLC
    ADC.b $E2,X
    STA.b $E2,X
    LDA.b $E0,X
    CLC
    ADC.b $04
    STA.b $E0,X

    CPX.w #$0006
    BEQ .delta_y
    LDX.w #$0001
    BRA .store_delta
.delta_y
    LDX.w #$0000
.store_delta
    SEP #$20
    LDA.b $04
    STA.l !BackgroundDeltaY,X
    STA.w $069E,X
.wait
    SEP #$30
    JML $82A929                 ; Return through the transition handler's RTS.

.ready
    STA.l !TransitionCameraSnap
    LDA.b $18
    AND.w #$00FF
    BNE .wait                  ; Finish any queued source stripe before preloading.
    LDA.l !TransitionCameraSnap
    BEQ .begin_setup
    JSL !PreloadAlignedTransitionEdge
.begin_setup
    REP #$30
    LDA.w $0418
    TAY
    LDA.w $0410
    STA.b $00
    TAX
    SEP #$30
    STZ.b $11
    STY.b $02
    STZ.b $03
    JML $82A936                 ; Begin vanilla setup with the source still active.

SnapAlignedOverworldCamera:
    PHP
    REP #$30
    PHX
    LDA.w $0126
    AND.w #$00FF
    BNE .done
    LDA.b $11
    AND.w #$00FF
    CMP.w #$0006
    BNE .done
    LDA.l !TransitionCameraSnap
    BEQ .done

    LDX.w #$0000
    LDA.w $0418
    AND.w #$00FF
    CMP.w #$0002
    BCC .have_axis
    LDX.w #$0006
.have_axis
    LDA.b $E2,X
    CLC
    ADC.l !TransitionCameraSnap
    STA.b $E2,X

    ; The source strip was preloaded before setup; only stream the main axis.
    CPX.w #$0006
    BEQ .finalize_y
    CLC
    ADC.w $011A
    STA.w $011E
    BRA .finalized
.finalize_y
    CLC
    ADC.w $011C
    STA.w $0122
.finalized
    LDA.b $E0,X
    CLC
    ADC.l !TransitionCameraSnap
    STA.b $E0,X

    CPX.w #$0006
    BEQ .delta_y
    LDX.w #$0001
    BRA .store_delta
.delta_y
    LDX.w #$0000
.store_delta
    SEP #$20
    LDA.l !TransitionCameraSnap
    STA.l !BackgroundDeltaY,X
    STA.w $069E,X
.done
    PLX
    PLP

    ; Run hi-jacked instructions:
    INC.w $0126
    LDA.w $0418

    RTL
assert pc() <= !free_space_bank_a0_end
