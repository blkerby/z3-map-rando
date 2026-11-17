; Resolve weather-controlled music in foreground code before yielding to NMI.
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A09A00
!free_space_bank_a0_end = $A09B00

; PrepareOAMForTransfer returns with A8/X8/Y8. Preserve A/P; X/Y, DB, DP,
; scratch RAM, and the stack are unchanged when the main loop resumes.
org $80805D
hook_finish_game_frame:
    JML FinishGameFrame

org $808034
return_wait_for_nmi:

; Iris rendering waits for the beam and can cross vblank before returning to
; the main loop. Resolve its queued music before that foreground work starts.
; DB is already $80; the displaced instructions select A16/X16/Y16.
org $80F310
hook_prepare_iris_frame:
    JML PrepareIrisFrame
    NOP

org $80F315
return_prepare_iris_frame:

; House music must be selected before the indoor music bank is loaded.
; Spawn decoding has A8/X16/Y16; only A and flags change, as in the loads.
org $82DAD2
hook_select_spawn_music:
    JSL SelectSpawnMusic
    NOP
    NOP

; Indoor rain ambience accompanies the opening track, including house spawns.
; Entry has A8; X/Y, DB, scratch, and the caller's stack remain untouched.
org $828296
hook_select_indoor_rain_sound:
    JML SelectIndoorRainSound

org !free_space_bank_a0_start
FinishGameFrame:
    JSR UpdateWeatherMusic
    STZ.b $12                  ; Run hi-jacked instruction
    JML return_wait_for_nmi

PrepareIrisFrame:
    JSR UpdateWeatherMusic

    ; Run hi-jacked instructions:
    REP #$30
    LDA.b $20
    SEC

    JML return_prepare_iris_frame

; Preserve widths, A/X/Y, DB, DP, and scratch while resolving the pending song.
UpdateWeatherMusic:
    PHP
    SEP #$20
    PHA
    LDA.w $012C
    BEQ .restore
    LDA.w $0136                ; Regional song IDs belong to the overworld bank.
    BNE .restore
    LDA.b $10
    CMP.b #$08
    BCC .restore
    CMP.b #$11
    BCS .restore
    LDA.b $1B
    BEQ .outdoors

    ; An indoor exit can keep Rain playing into the outdoor destination.
    LDA.b $10
    CMP.b #$0F
    BNE .restore
    LDA.w $012C
    CMP.b #$F1
    BEQ .retain_rain

.restore
    PLA
    PLP
    RTS

.outdoors
    LDA.b $10
    CMP.b #$0F                  ; Entering an interior retains its music fade.
    BEQ .restore
    LDA.w $04C6                ; Entrance cutscenes own their music commands.
    BNE .restore
    LDA.w $012C
    CMP.b #$F1
    BEQ .retain_rain

    ; Keep explicit bunny-track requests.
    CMP.b #$04
    BEQ .restore

    ; Only regional tracks change; event tracks and volume commands stay intact.
    CMP.b #$02
    BCC .restore
    CMP.b #$06
    BCC .regional
    CMP.b #$07
    BEQ .regional
    CMP.b #$09
    BEQ .regional
    CMP.b #$0D
    BNE .restore

.regional
    PHA
    ; Transition recovery can request a regional track while Link is a bunny.
    LDA.w $02E0
    ORA.b $56
    BEQ .weather
    PLA
    LDA.b #$04
    BRA .store

.weather
    LDA.l !EventFlags
    AND.b #$80
    BNE .dry
    PLA
    LDA.b #$03
    CMP.w $0130
    BNE .store
    LDA.b #$F3                  ; Repeating the song ID would restart it on the SPC.
.store
    STA.w $012C
    BRA .restore

.dry
    PLA
    CMP.b #$03
    BNE .restore
    LDA.b #$02                  ; Replace vanilla's story-selected rain track.
    BRA .store

.retain_rain
    LDA.l !EventFlags
    AND.b #$80
    BNE .restore
    LDA.w $0130
    CMP.b #$03
    BNE .restore
    STZ.w $012C                ; Consume the transition fade without stopping Rain.
    BRA .restore

SelectSpawnMusic:
    LDA.l $82D9B2,X            ; Run hi-jacked instruction
    CPX.w #$0000
    BNE .store
    PHA
    LDA.l !EventFlags
    AND.b #$80
    BNE .keep
    PLA
    LDA.b #$03
    BRA .store
.keep
    PLA
.store
    STA.w $0132               ; Run hi-jacked instruction
    RTL

SelectIndoorRainSound:
    LDA.l !EventFlags
    AND.b #$80
    BNE .quiet
    LDA.w $0132
    CMP.b #$03
    BNE .quiet
    JML $82829E                ; Use the floor and Sanctuary audibility rules.
.quiet
    JML $8282BC

assert pc() <= !free_space_bank_a0_end
