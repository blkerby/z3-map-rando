; New saves begin with Zelda rescued, matching the starting logic state.
lorom

!free_space_bank_a0_start = $A0F000
!free_space_bank_a0_end = $A0F040

; InitializeSaveFile has 16-bit A/X/Y and X holds the selected SRAM slot's
; offset. Set progress before vanilla calculates the save checksum.
org $8CDBBC
hook_initialize_save_checksum:
    JSL InitializeGameState

; File loading has 8-bit A. X/Y and scratch RAM are untouched.
org $8280DD
hook_select_starting_location:
    JML PrepareInitialSpawn
return_check_save_progress:

org $828118
return_load_initial_house:

org !free_space_bank_a0_start
InitializeGameState:
    SEP #$20
    LDA.b #$02
    STA.l $7003C5,X
    REP #$20

    ; Run hi-jacked instructions:
    LDY.w #$0000
    TYA

    RTL

PrepareInitialSpawn:
    LDA.l $7EF3C6
    BIT.b #$10
    BNE .resume

    ; Uncle's departure flag also identifies whether the opening was skipped.
    ORA.b #$10
    STA.l $7EF3C6
    LDA.b #$00
    STA.l $7EF3C8
    JML return_load_initial_house

.resume
    LDA.l $7EF3C5              ; Run hi-jacked instruction
    JML return_check_save_progress

assert pc() <= !free_space_bank_a0_end
