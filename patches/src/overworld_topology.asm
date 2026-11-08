; Keep logical area IDs in $8A/$040A while resolving physical origins separately.
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0F200
!free_space_bank_a0_end = $A0F400
!free_space_bank_bf_start = $BFE540
!free_space_bank_bf_end = $BFE580

; LoadGraphicsAndScreenSize has 16-bit A/X/Y here. The displaced origin
; calculation clobbers A/X and writes $0708/$070C; the replacement does likewise.
org $82AA4B
hook_load_overworld_transition_origin:
    JSL LoadPhysicalTransitionOrigin
    BRA return_load_overworld_transition_origin
org $82AA63
return_load_overworld_transition_origin:

; Entry has 8-bit A/X/Y. Return to the original camera routine with 16-bit A
; and 8-bit X/Y, matching the displaced index preparation.
org $82E304
hook_initialize_overworld_camera:
    JSL PreparePhysicalCameraIndices
    JSR.w $BFFB
    BRA return_initialize_overworld_camera
org $82E318
return_initialize_overworld_camera:

; Preserve the caller's widths and X; Y and scratch RAM are untouched.
org $89C4AC
hook_load_overworld_sprite_origin:
    JSL InitializePhysicalSpriteOrigin
    BRA return_load_overworld_sprite_origin
org $89C4C7
return_load_overworld_sprite_origin:

org !free_space_bank_a0_start
GetPhysicalAreaAnchor:
assert GetPhysicalAreaAnchor == !GetPhysicalAreaAnchor
    CMP.w #$0080
    BCS .special
    PHX
    AND.w #$003F
    TAX
    LDA.l !AreaToAnchor,X
    AND.w #$00FF
    PLX
.special
    RTL

LoadPhysicalTransitionOrigin:
    ; Use the physical anchor for table indexing.
    ; Run hi-jacked instructions:
    LDA.b $8A
    JSL GetPhysicalAreaAnchor
    AND.w #$00BF
    ASL A
    TAX
    LDA.l $82A7BB,X
    STA.w $0708
    LDA.l $82A83B,X
    LSR A
    LSR A
    LSR A
    STA.w $070C
    RTL

PreparePhysicalCameraIndices:
    REP #$30

    ; Use the physical anchor for table indexing.
    ; Run hi-jacked instructions:
    LDA.b $8A
    JSL GetPhysicalAreaAnchor
    AND.w #$003F
    ASL A
    TAY
    LDX.w #$0000
    LDA.w $0712
    BEQ .small
    INX
    INX
.small
    SEP #$10
    RTL

InitializePhysicalSpriteOrigin:
    PHP
    SEP #$20
    LDA.w $040A
    AND.b #$40
    STA.l !OverworldSpriteWorld
    REP #$30
    PHX
    LDA.w $040A
    JSL GetPhysicalAreaAnchor
    PHA
    AND.w #$0007
    XBA
    ASL A
    STA.w $0FBC
    PLA
    AND.w #$0038
    XBA
    LSR A
    LSR A
    STA.w $0FBE
    PLX
    PLP
    RTL
assert pc() <= !free_space_bank_a0_end

; Replace the vanilla physical-cell ownership table directly.
org !SlotToArea
SlotToArea:
assert SlotToArea == !SlotToArea

org !free_space_bank_bf_start
AreaToAnchor:
assert AreaToAnchor == !AreaToAnchor
; Rust supplies the two 64-byte tables.
skip 64
assert pc() <= !free_space_bank_bf_end
