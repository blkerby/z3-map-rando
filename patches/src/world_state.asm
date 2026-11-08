; Keep the saved world flag ($7EF3CA) consistent with dungeon continuations
; and loaded overworld areas without granting early access to the Pyramid.
;
; Vanilla death recovery clears the world flag and forces human form whenever
; Agahnim 1 is unbeaten ($7EF3C5 < $03). This returns an early Dark World visit
; to Light World spawn selection, which we retain for outdoor and cave deaths.
; However, the randomizer also allows Dark World dungeons before Agahnim 1.
; Continuing in one must retain its world and bunny context, just as it would
; after Agahnim 1. Otherwise, a later dungeon exit can load its Dark World
; screen while leaving the flag set to Light World; the next outdoor boundary
; then selects the Light World counterpart (e.g. Lost Woods instead of Skull
; Woods). Skip both vanilla overrides only for dungeon deaths. Ganon's room
; remains an outdoor recovery case, matching vanilla's handling later in the
; same routine. Cave deaths retain vanilla's non-dungeon recovery behavior.
;
; Also synchronize the flag from the resolved destination when loading normal
; overworld screen properties: areas $00-$3F are Light World, $40-$7F are Dark
; World. Do this before graphics, sprite, and later bunny initialization use
; the destination context. This repairs an inconsistent flag on dungeon exit
; as well as preventing later transitions from using the wrong world. Special
; areas ($80 and above) retain their existing context; indoor room numbers do
; not identify a world and must retain their entrance's context instead.

lorom

!free_space_bank_a0_start = $A0F500
!free_space_bank_a0_end = $A0F580

; Death recovery has 8-bit A/X/Y. Only A and flags are scratch here; X/Y,
; the data bank, direct-page scratch RAM, and the stack are untouched.
org $89F520
hook_recover_world_after_death:
    JML PreserveDungeonWorldOnDeath
return_check_death_progress:

org $89F538
return_continue_death_recovery:

; All callers have 16-bit A here. X/Y widths vary and are left untouched.
; Replace the five-byte camera-bias setup, then return with its original A,
; flags, and $EC value. The status push/pop is balanced on both paths;
; the data bank and direct-page scratch RAM are untouched.
org $82E2EF
hook_load_overworld_world_state:
    JML SynchronizeLoadedOverworldWorld
    NOP
return_load_overworld_properties:

org !free_space_bank_a0_start
PreserveDungeonWorldOnDeath:
    LDA.b $1B
    BEQ .vanilla
    LDA.b $A0
    ORA.b $A1
    BEQ .vanilla               ; Vanilla treats Ganon's room as an outdoor death.
    LDA.w $040C
    CMP.b #$FF
    BNE .dungeon

.vanilla
    LDA.l $7EF3C5              ; Run hi-jacked instruction
    JML return_check_death_progress

.dungeon
    JML return_continue_death_recovery

SynchronizeLoadedOverworldWorld:
    PHP
    LDA.b $8A
    CMP.w #$0080
    BCS .finish
    AND.w #$0040
    STA.l $7EF3CA              ; Also retain the required zero high byte.

.finish
    PLP

    ; Run hi-jacked instructions:
    LDA.w #$FFF8
    STA.b $EC

    JML return_load_overworld_properties
assert pc() <= !free_space_bank_a0_end
