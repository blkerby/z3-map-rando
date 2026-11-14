; New saves begin with Zelda rescued. Restart choices depend on the saved
; world and visited destinations; dungeon deaths and Mirror retain their entrance.
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0F900
!free_space_bank_a0_end = $A0FB00

; InitializeSaveFile has 16-bit A/X/Y and X holds the selected SRAM slot's
; offset. Set progress before vanilla calculates the save checksum.
org $8CDBBC
hook_initialize_save_checksum:
    JSL InitializeGameState

; File loading and spawn dispatch have 8-bit A/X/Y. The vanilla loaders
; clobber these registers; no additional direct-page scratch is used.
org $8280A2
hook_select_starting_location:
    JML SelectStartingLocation

org $828496
hook_load_selected_spawn:
    JML LoadSelectedSpawn

; Zelda and the Old Man must not substitute a checkpoint for a dungeon retry.
org $89F5AF
hook_select_dungeon_death_spawn:
    BRA return_check_death_dungeon
org $89F5B7
return_check_death_dungeon:

org $82D6EB
hook_select_underworld_restart:
    BRA return_check_underworld_restart
org $82D6F4
return_check_underworld_restart:

; Keep the receipt/dialogue accumulator and flags, but skip progress rewrites.
org $85DF67
hook_receive_uncle_equipment:
    BRA return_finish_uncle_equipment
org $85DF6B
return_finish_uncle_equipment:

; Keep Zelda available regardless of sword level.
org $85EBCF
hook_initialize_zelda_spawn:
    BRA return_initialize_zelda_spawn
org $85EBD7
return_initialize_zelda_spawn:

; The preceding follower check still suppresses a duplicate castle Zelda.
org $85EC40
hook_check_castle_zelda_spawn:
    RTS

org $85EDE1
hook_complete_zelda_rescue:
    BRA return_finish_zelda_rescue
org $85EDE5
return_finish_zelda_rescue:

; Priest preparation has 8-bit A/X/Y; its progress checks only clobber A.
; Entering Sanctuary unlocks its spawn and always initializes a living priest.
org $85DA49
hook_initialize_sanctuary_priest:
    JML RecordSanctuaryVisit
org $85DA67
return_initialize_living_priest:

org $85DA7B
hook_select_priest_sequence:
    BRA return_check_priest_follower
org $85DA8F
return_check_priest_follower:

org !free_space_bank_a0_start
InitializeGameState:
    SEP #$20
    LDA.b #$02
    STA.l $7003C5,X
    ; Starting bomb and arrow capacities (see capacity_upgrades.asm).
    LDA.l StartingCapacities
    STA.l $700370,X
    LDA.l StartingCapacities+1
    STA.l $700371,X
    REP #$20

    ; Run hi-jacked instructions:
    LDY.w #$0000
    TYA

    RTL

SelectStartingLocation:
    LDA.l $7EF3C6
    BIT.b #$10
    BNE .resume

    ; Uncle's departure flag also identifies whether the opening was skipped.
    ORA.b #$10
    STA.l $7EF3C6
    JMP LoadHouseSpawn

.resume
    LDA.l $7EC011             ; Indoor Mirror warps leave an active mosaic fade.
    BNE .dungeon
    LDA.w $010A
    BEQ .menu
    LDA.w $04AA
    BNE .menu

.dungeon
    JML $828118               ; Continue at the dungeon entrance.

.menu
    LDA.l $7EF3CA
    BEQ .light_world
    LDA.l $7EF353
    CMP.b #$02
    BEQ LoadPyramidSpawn
    BRA .two_choices

.light_world
    LDA.l !SpawnVisitFlags
    AND.b #$03
    BEQ LoadHouseSpawn
    CMP.b #$03
    BNE .two_choices
    REP #$10
    LDX.w #$0183
    BRA .show_menu

.two_choices
    REP #$10
    LDX.w #$0182

.show_menu
    STX.w $1CF0
    SEP #$10
    JML $828102               ; Display the native spawn-selection window.

LoadSelectedSpawn:
    LDA.l $7EF3CA
    BEQ .light_world
    LDA.w $1CE8
    BNE LoadPyramidSpawn
    BRA LoadHouseSpawn

.light_world
    LDX.w $1CE8
    BEQ LoadHouseSpawn
    LDA.l !SpawnVisitFlags
    BIT.b #$01
    BNE .select
    INX                       ; Mountain occupies Sanctuary's row when it is locked.

.select
    LDA.l SpawnPointIds,X
    BRA LoadIndoorSpawn

LoadHouseSpawn:
    LDA.b #$00

LoadIndoorSpawn:
    TAY
    LDA.l $7EF3C8
    PHA
    TYA
    STA.l $7EF3C8

    LDA.b #$00
    STA.l $7EF3CA
    JSL $82846E               ; ForceNonbunnyStatus for the Light World destination.
    STZ.b $B0
    LDA.b #$01
    STA.w $04AA               ; Load the selected spawn rather than the last dungeon entrance.
    JSL $828118

    PLA
    STA.l $7EF3C8
    RTL

LoadPyramidSpawn:
    STZ.b $1B
    STZ.w $010A
    STZ.w $04AA
    JML $8280AC               ; Rebuild the HUD and load the Pyramid overworld spawn.

RecordSanctuaryVisit:
    LDA.l !SpawnVisitFlags
    ORA.b #$01
    STA.l !SpawnVisitFlags
    JML return_initialize_living_priest

SpawnPointIds:
    db $00, $01, $06

assert pc() <= !StartingCapacities
org !StartingCapacities
StartingCapacities:
assert StartingCapacities == !StartingCapacities
    skip 2

assert pc() <= !free_space_bank_a0_end
