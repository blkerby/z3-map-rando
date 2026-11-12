; Preserve world context through death and save-and-quit. Normal overworld
; destinations synchronize it before graphics and bunny initialization;
; interiors and special areas retain their entrance's context.

lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0F500
!free_space_bank_a0_end = $A0F580

; Skip both the pre-Agahnim world reset and its forced-human override.
org $89F520
hook_recover_world_after_death:
    BRA return_continue_death_recovery

org $89F538
return_continue_death_recovery:

; Recovery has 8-bit A/X/Y. Only A and flags change, as in the displaced
; load; X/Y, scratch RAM, and the stack are untouched.
org $89F544
hook_recover_followers:
    JML ClearFollowersAfterRecovery
return_check_death_followers:

; All callers have 8-bit A/X/Y. Preserve A and flags around bookkeeping;
; only the displaced LDX changes X. No scratch RAM or data-bank changes.
; This shared loader covers scrolling, exits, flute travel, and mirror warps.
org $82AA07
hook_load_overworld_world_state:
    JML SynchronizeLoadedOverworldWorld
    NOP
return_load_overworld_graphics:

org !free_space_bank_a0_start
SynchronizeLoadedOverworldWorld:
    PHP
    PHA
    LDA.b $8A
    CMP.b #$80
    BCS .finish
    AND.b #$40
    STA.l $7EF3CA

    LDA.b $8A
    CMP.b #$03
    BEQ .mountain
    CMP.b #$05
    BEQ .mountain
    CMP.b #$07
    BNE .finish

.mountain
    LDA.l !SpawnVisitFlags
    ORA.b #$02
    STA.l !SpawnVisitFlags

.finish
    PLA
    PLP

    ; Run hi-jacked instructions:
    LDX.b $8A
    STZ.w $0412

    JML return_load_overworld_graphics

ClearFollowersAfterRecovery:
    LDA.b $B0
    CMP.b #$01
    BEQ .clear
    LDA.b $1B
    BEQ .clear
    LDA.w $040C
    CMP.b #$FF
    BNE .finish

.clear
    LDA.b #$00
    STA.l $7EF3CC

.finish
    LDA.l $7EF3CC ; Run hi-jacked instruction
    JML return_check_death_followers
assert pc() <= !free_space_bank_a0_end
