; Run generated overworld background animations (e.g. for water, clouds).
; Rust stores the following data in the generated asset bundle:
;
; Area record:
;   +$0F              3-byte animation-list pointer, or zero
;
; Animation list:
;   +$00 + 3*n        3-byte pointer to track n
;   after last track  1-byte zero bank terminator
;
; Track definition:
;   +$00              1-byte frame count
;   +$01              1-byte frame hold time
;   +$02              1-byte initial frame
;   +$03              1-byte initial countdown
;   +$04 + 3*n        3-byte pointer to frame n's descriptor list
;
; Frame descriptor list:
;   +$00              1-byte empty-palette terminator
;   +$01 + 4*n        source bank, address low, address high, destination row
;   after last row    1-byte zero bank terminator
;
; All 3-byte pointers are stored bank first, then address low and high.
;
; Activating an area record copies its tracks into compact WRAM state. The
; main loop advances each track independently and combines due character-row
; descriptors for one NMI upload. Dungeon animation retains the vanilla path.

lorom

incsrc "symbols.inc"

!free_space_bank_a0_start = !ActivateOverworldAnimationTracks
!free_space_bank_a0_end = $A0A0C1

!AnimationTrackCount = $7EC90C
; ponytail: Desert uses at most three tracks/rows; expand these ranges if a
; future theme exceeds 64 active tracks or 127 simultaneously updated rows.
!AnimationTrackState = $7EC910
!AnimationDescriptorList = $7ECA50

; DecompressAnimatedOverworldTiles: generated animation tracks load their
; frames directly from ROM, so the old overworld WRAM conversion is obsolete.
org $80D3D4
    RTL
assert pc() == $80D3D5

; PrepareOAMForTransfer, after dynamic OBJ sources are selected: replace only
; the overworld animated-background timer with the generated track scheduler.
org $8086DF
hook_PrepareOverworldAnimations:
    JML ScheduleOverworldAnimations
assert pc() == $8086E3

; DoNMIUpdates, after ordinary dynamic OBJ uploads: omit the old 1 KiB
; animated-background cache upload only while the overworld layout is active.
org $808B50
hook_NMIUploadOverworldAnimations:
    JML UploadOverworldAnimations
    NOP
    NOP
assert pc() == $808B56

org !free_space_bank_a0_start

; Copy the animation list at record offset 15 into compact runtime state.
; Each state entry is definition pointer low/high/bank, frame, countdown.
; Input: $00-$02 = selected area-record pointer.
ActivateOverworldAnimationTracks:
    PHP
    SEP #$20
    REP #$10
    PHA
    PHX
    PHY

    LDA.b #$00
    STA.l !AnimationTrackCount
    LDY.w #$000F
    LDA.b [$00],Y
    STA.b $03
    INY
    LDA.b [$00],Y
    STA.b $04
    INY
    LDA.b [$00],Y
    STA.b $05
    BEQ .done

    LDY.w #$0000
    LDX.w #$0000
.next_track
    LDA.b [$03],Y
    BEQ .done
    STA.l !AnimationTrackState+2,X
    STA.b $08
    INY
    LDA.b [$03],Y
    STA.l !AnimationTrackState,X
    STA.b $06
    INY
    LDA.b [$03],Y
    STA.l !AnimationTrackState+1,X
    STA.b $07
    INY

    PHY
    LDY.w #$0002
    LDA.b [$06],Y
    STA.l !AnimationTrackState+3,X
    INY
    LDA.b [$06],Y
    STA.l !AnimationTrackState+4,X
    PLY

    INX
    INX
    INX
    INX
    INX
    LDA.l !AnimationTrackCount
    INC A
    STA.l !AnimationTrackCount
    BRA .next_track

.done
    PLY
    PLX
    PLA
    PLP
    RTS

; Replace PrepareOAMForTransfer's overworld animation timer. Dungeon layouts
; retain the displaced vanilla load and continue through its existing timer.
ScheduleOverworldAnimations:
    LDA.w $0219
    CMP.w #!OverworldHUD
    BEQ .overworld

    LDA.l $7EC00D              ; Run hi-jacked instruction
    JML $8086E3

.overworld
    PHP
    PHX
    PHY
    SEP #$20
    REP #$10
    JSR BuildAnimationDescriptorList
    SEP #$10
    PLY
    PLX
    PLP
    JML $808719

; Tick every active track and combine due frame descriptors in one WRAM list.
; The list begins with the standard empty palette terminator; $35-$37 points
; one byte past it because NMI consumes only the character descriptors.
BuildAnimationDescriptorList:
    LDA.l !AnimationTrackCount
    BNE .active
    RTS
.active
    STA.b $0D
    LDA.l !AnimationSuspended
    ORA.b $17
    ORA.w $0710
    BEQ .build
    RTS
.build

    LDA.b #$00
    STA.l !AnimationDescriptorList
    REP #$20
    LDA.w #$0001
    STA.b $09
    STZ.b $0B
    SEP #$20

.next_track
    LDX.b $0B
    LDA.l !AnimationTrackState,X
    STA.b $03
    LDA.l !AnimationTrackState+1,X
    STA.b $04
    LDA.l !AnimationTrackState+2,X
    STA.b $05

    LDA.l !AnimationTrackState+4,X
    DEC A
    STA.l !AnimationTrackState+4,X
    BNE .advance_state

    LDY.w #$0001
    LDA.b [$03],Y
    STA.l !AnimationTrackState+4,X
    LDA.l !AnimationTrackState+3,X
    INC A
    LDY.w #$0000
    CMP.b [$03],Y
    BCC .store_frame
    LDA.b #$00
.store_frame
    STA.l !AnimationTrackState+3,X
    STA.b $0E
    STZ.b $0F

    REP #$20
    LDA.b $0E
    ASL A
    CLC
    ADC.b $0E
    CLC
    ADC.w #$0004
    TAY
    SEP #$20

    LDA.b [$03],Y
    STA.b $08
    INY
    LDA.b [$03],Y
    STA.b $06
    INY
    LDA.b [$03],Y
    STA.b $07
    LDY.w #$0001

.next_descriptor
    LDA.b [$06],Y
    BEQ .advance_state
    LDX.b $09
    STA.l !AnimationDescriptorList,X
    INX
    INY
    LDA.b [$06],Y
    STA.l !AnimationDescriptorList,X
    INX
    INY
    LDA.b [$06],Y
    STA.l !AnimationDescriptorList,X
    INX
    INY
    LDA.b [$06],Y
    STA.l !AnimationDescriptorList,X
    INX
    INY
    STX.b $09
    BRA .next_descriptor

.advance_state
    REP #$20
    LDA.b $0B
    CLC
    ADC.w #$0005
    STA.b $0B
    SEP #$20
    DEC.b $0D
    BEQ .tracks_done
    JMP .next_track

.tracks_done
    LDX.b $09
    LDA.b #$00
    STA.l !AnimationDescriptorList,X
    CPX.w #$0001
    BEQ .return

    REP #$20
    LDA.w #!AnimationDescriptorList+1
    STA.b $35
    SEP #$20
    LDA.b #!AnimationDescriptorList>>16
    STA.b $37
    LDA.b #$03
    STA.b $17
.return
    RTS

; Retain the complete vanilla animated-background upload in dungeons. The
; overworld path returns after the surrounding dynamic OBJ transfers instead.
UploadOverworldAnimations:
    LDA.w $021A
    CMP.b #!OverworldHUD>>8
    BEQ .skip

    ; Run hi-jacked instructions:
    LDX.w $0ADC
    STX.w $4302

    JML $808B56

.skip
    JML $808B67

assert pc() <= !free_space_bank_a0_end
