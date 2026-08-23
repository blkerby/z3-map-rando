; Execute generated dungeon-entrance cutscene scripts and apply persistent
; event terrain when an affected area is loaded.

lorom

incsrc "symbols.inc"

!free_space_bank_a0_start = $A0B6A0
!free_space_bank_a0_end = $A0C000

!CutsceneCursor = $7ECC7D
!CutsceneWait = $7ECC80
!CutsceneShake = $7ECC81

org $82A493
hook_run_entrance_cutscene:
    JSL RunGeneratedEntranceCutscene
assert pc() == $82A497

; Preserve every unrelated vanilla overlay, then replace cutscene terrain with
; the final generated state for the selected theme.
org $82ECB4
hook_apply_overworld_overlay:
    JSL ApplyGeneratedOverworldOverlay
assert pc() == $82ECB8

; Replace the vanilla weather-vane Map16 writes with the generated layer.
org $88D0B5
hook_break_bird_statue:
    JSL BreakGeneratedBirdStatue
assert pc() == $88D0B9

org !free_space_bank_a0_start

RunGeneratedEntranceCutscene:
    STA.w $02E4
    STA.w $0FC1
    STA.w $0710

    LDA.l !CutsceneCursor
    ORA.l !CutsceneCursor+1
    ORA.l !CutsceneCursor+2
    BNE .initialized

    LDA.w $04C6
    DEC A
    REP #$30
    AND.w #$00FF
    STA.b $00
    ASL A
    CLC
    ADC.b $00
    TAX
    LDA.l !CutscenePointers,X
    STA.l !CutsceneCursor
    SEP #$20
    LDA.l !CutscenePointers+2,X
    STA.l !CutsceneCursor+2

.initialized
    SEP #$30
    LDA.l !CutsceneWait
    BEQ .execute
    DEC A
    STA.l !CutsceneWait
    BEQ .execute
    JMP .finish

.execute
    REP #$20
    LDA.l !CutsceneCursor
    STA.b $07
    SEP #$20
    LDA.l !CutsceneCursor+2
    STA.b $09
    REP #$10
    LDY.w #$0000

.next_action
    LDA.b [$07],Y
    INY
    CMP.b #$00
    BNE +
    JMP .end
+   CMP.b #$01
    BNE +
    JMP .wait
+   CMP.b #$02
    BNE +
    JMP .play_sound
+   CMP.b #$03
    BNE +
    JMP .play_music
+   CMP.b #$04
    BNE +
    JMP .draw
+   CMP.b #$05
    BNE +
    JMP .set_complete
+   CMP.b #$06
    BNE +
    JMP .start_shake
+   JMP .stop_shake

.wait
    LDA.b [$07],Y
    STA.l !CutsceneWait
    INY
    REP #$20
    TYA
    CLC
    ADC.b $07
    STA.l !CutsceneCursor
    SEP #$20
    LDA.b $09
    STA.l !CutsceneCursor+2
    JMP .finish

.play_sound
    LDA.b [$07],Y
    INY
    DEC A
    SEP #$10
    TAX
    LDA.b [$07],Y
    INY
    STA.w $012D,X
    REP #$10
    JMP .next_action

.play_music
    LDA.b [$07],Y
    INY
    STA.w $012C
    JMP .next_action

.draw
    LDA.b [$07],Y
    STA.b $0E
    STZ.b $0F
    INY
    REP #$30

.next_write
    LDA.b [$07],Y
    TAX
    INY
    INY
    LDA.b [$07],Y
    INY
    INY
    PHY
    JSL $9BC97C
    PLY
    DEC.b $0E
    BNE .next_write

    SEP #$20
    LDA.b #$01
    STA.b $14
    JMP .next_action

.set_complete
    SEP #$30
    LDX.b $8A
    LDA.l $7EF280,X
    ORA.b #$20
    STA.l $7EF280,X
    REP #$10
    JMP .next_action

.start_shake
    LDA.b #$01
    STA.l !CutsceneShake
    JMP .next_action

.stop_shake
    LDA.b #$00
    STA.l !CutsceneShake
    STZ.w $011A
    STZ.w $011B
    STZ.w $011C
    STZ.w $011D
    JMP .next_action

.end
    SEP #$30
    STZ.w $04C6
    STZ.b $B0
    STZ.b $C8
    STZ.w $0710
    STZ.w $02E4
    STZ.w $0FC1
    LDA.b #$00
    STA.l !CutsceneCursor
    STA.l !CutsceneCursor+1
    STA.l !CutsceneCursor+2
    STA.l !CutsceneWait
    STA.l !CutsceneShake
    STZ.w $011A
    STZ.w $011B
    STZ.w $011C
    STZ.w $011D

.finish
    SEP #$30
    LDA.l !CutsceneShake
    BEQ .done
    LDA.b $1A
    LSR A
    BCS .odd_shake

    REP #$20
    LDA.w #$0001
    STA.w $011A
    LDA.w #$FFFF
    STA.w $011C
    BRA .shake_done

.odd_shake
    REP #$20
    LDA.w #$FFFF
    STA.w $011A
    LDA.w #$0001
    STA.w $011C

.shake_done
    SEP #$30

.done
    RTL

ApplyGeneratedOverworldOverlay:
    JSL $87FAE2

    REP #$30
    LDA.b $8A
    AND.w #$00FF
    STA.b $00
    ASL A
    CLC
    ADC.b $00
    TAX
    LDA.l !OverworldOverlayPointers,X
    STA.b $07
    SEP #$20
    LDA.l !OverworldOverlayPointers+2,X
    BEQ .no_generated_overlay
    STA.b $09

    REP #$10
    LDY.w #$0000
    LDA.b [$07],Y
    STA.b $0E
    STZ.b $0F
    INY
    REP #$20

.next_overlay_write
    LDA.b [$07],Y
    TAX
    INY
    INY
    LDA.b [$07],Y
    STA.l $7E2000,X
    INY
    INY
    DEC.b $0E
    BNE .next_overlay_write

.no_generated_overlay
    SEP #$30
    RTL

BreakGeneratedBirdStatue:
    REP #$30
    LDA.b $8A
    AND.w #$00FF
    STA.b $00
    ASL A
    CLC
    ADC.b $00
    TAX
    LDA.l !OverworldOverlayPointers,X
    STA.b $07
    SEP #$20
    LDA.l !OverworldOverlayPointers+2,X
    STA.b $09

    REP #$10
    LDY.w #$0000
    LDA.b [$07],Y
    STA.b $0E
    STZ.b $0F
    INY
    REP #$20

.next_write
    LDA.b [$07],Y
    TAX
    INY
    INY
    LDA.b [$07],Y
    INY
    INY
    PHY
    JSL $9BC97C
    PLY
    DEC.b $0E
    BNE .next_write

    SEP #$30
    LDX.b $8A
    LDA.l $7EF280,X
    ORA.b #$20
    STA.l $7EF280,X
    LDA.b #$01
    STA.b $14
    RTL

assert pc() <= !free_space_bank_a0_end
