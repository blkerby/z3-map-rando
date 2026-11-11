; Preload source tiles, prepare the destination, then align before main scrolling.
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

; Ordinary transitions skip phase $05; use it to wait for source-side preloading.
; Mosaic phase $13 still dispatches to vanilla StartOverworldScrollTransition.
org $82A30E
    dw hook_preload_overworld_transition

org $82AAD9
hook_run_overworld_transition_scroll:
    JML RunAlignedOverworldScroll

; OverworldScrollTransition has already saved DB and selected bank $82.
; Preserve X/Y and widths, then run its displaced frame counter/direction load.
org $82BF3C
hook_begin_overworld_transition_scroll:
    JSL SnapAlignedOverworldCamera
    NOP
    NOP

org !free_space_bank_82_start
hook_preload_overworld_transition:
    JML PreloadOverworldTransitionCamera
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
    BCC .prepare
    LDA.b $04
    CMP.b $E2,X
    BCS .prepare
    LDA.b $E2,X

.prepare
    STA.l !TransitionCameraTarget
    PLY
    PLX
    STY.w $0418                ; Retain the original direction table offset.
    STX.w $0410
    SEP #$30
    LDA.b #$05
    STA.b $11
    JML PreloadOverworldTransitionCamera

.ready
    PLY
    PLX
    SEP #$30

    ; Run hi-jacked instructions:
    STY.b $02
    STZ.b $03

    JML $82A936

PreloadOverworldTransitionCamera:
    REP #$30
    LDX.w #$0000
    LDA.w $0418
    CMP.w #$0004
    BCS .have_axis
    LDX.w #$0006
.have_axis
    ; Finish both source-camera axes before changing the active room identity.
    LDA.l !BackgroundDeltaY
    BNE .wait

    LDA.l !TransitionCameraTarget
    SEC
    SBC.b $E2,X
    STA.l !TransitionCameraSnap
    BEQ .begin_setup
    LDA.b $18
    AND.w #$00FF
    BNE .wait                  ; Finish any queued source stripe before preloading.
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

.wait
    SEP #$30
    JML $82A929                 ; Return through the transition handler's RTS.

RunAlignedOverworldScroll:
    JSL $87E68F                 ; Run hi-jacked instruction

    PHP
    REP #$30
    PHX
    LDA.b $11
    AND.w #$00FF
    CMP.w #$0006
    BNE .scroll
    LDA.w $0126
    AND.w #$00FF
    BNE .scroll
    LDA.l !TransitionCameraSnap
    BPL .positive
    CMP.w #$FFFE
    BCS .scroll
    LDA.w #$FFFE
    BRA .align
.positive
    CMP.w #$0003
    BCC .scroll
    LDA.w #$0002
.align
    STA.b $04
    LDA.l !TransitionCameraSnap
    SEC
    SBC.b $04
    STA.l !TransitionCameraSnap

    LDX.w #$0000
    LDA.w $0418
    AND.w #$00FF
    CMP.w #$0002
    BCC .have_axis
    LDX.w #$0006
.have_axis
    LDA.b $04
    JSR ApplyOverworldCameraAlignment
    PLX
    PLP
    JML $82A929                 ; Hold the main scroll counter and asset schedule.

.scroll
    PLX
    PLP
    JML $82AADD

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
    LDA.l !TransitionCameraSnap
    JSR ApplyOverworldCameraAlignment
.done
    PLX
    PLP

    ; Run hi-jacked instructions:
    INC.w $0126
    LDA.w $0418

    RTL

; A is the signed delta; X selects horizontal (0) or vertical (6). A/X/Y are
; 16-bit. Clobbers A/X and $04; preserves Y and register widths.
ApplyOverworldCameraAlignment:
    STA.b $04
    LDA.b $E2,X
    CLC
    ADC.b $04
    STA.b $E2,X

    ; The source strips are already resident; suppress destination-map streaming.
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
    REP #$20
    RTS
assert pc() <= !free_space_bank_a0_end
