; Execute generated dungeon-entrance cutscene scripts and apply persistent
; event terrain when an affected area is loaded.
;
; Generated cutscene script format:
;   $00                         End
;   $01, frames                 Wait
;   $02, channel, sound         Play sound on channel 1-3
;   $03, song                   Play music
;   $04, count, {offset, id}[]  Draw Map16 writes
;   $05                         Set the current area's event bit $20
;   $06                         Start screen shake
;   $07                         Stop screen shake
;
; Generated overworld overlay format:
;   count, {Map16 offset word, Map16 ID word}[]
;
; Direct-page scratch used here:
;   $07-$09  24-bit generated-data pointer
;   $0A-$0B  Overlay mode: zero updates WRAM only, nonzero also queues VRAM
;   $0E-$0F  16-bit write count

lorom

incsrc "symbols.inc"

!free_space_bank_a0_start = $A0B6A0
!free_space_bank_a0_end = $A0C000

!CutsceneCursor = $7ECC7D ; 24-bit pointer to the next script opcode.
!CutsceneWait = $7ECC80   ; Frames remaining before script execution resumes.
!CutsceneShake = $7ECC81  ; Nonzero while per-frame BG1 shake is active.

; Replace the vanilla entrance-cutscene dispatcher with the generated script
; interpreter. A nonzero $04C6 selects one of the generated scripts.
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

; Replace the vanilla Thieves' Town Map16 writes with the generated layer.
org $85E2C6
hook_open_thieves_town:
    JSL OpenGeneratedThievesTown
assert pc() == $85E2CA

; Keep all generated cutscene and overlay code in one free-space block.
org !free_space_bank_a0_start

; Run one frame of the active generated entrance cutscene.
;
; $04C6 is a one-based script selector. !CutsceneCursor is zero until the
; first frame initializes it, then points at the next opcode to execute.
; Actions execute in the same frame until one waits or ends the script.
RunGeneratedEntranceCutscene:
    ; Run hi-jacked instructions:
    STA.w $02E4
    STA.w $0FC1
    STA.w $0710

    LDA.l !CutsceneCursor       ; Test all three bytes of the saved cursor.
    ORA.l !CutsceneCursor+1
    ORA.l !CutsceneCursor+2
    BNE .initialized           ; A nonzero cursor resumes the current script.

    LDA.w $04C6                ; Convert the one-based trigger to table index.
    DEC A
    REP #$30                   ; Use 16-bit A and index registers for pointers.
    AND.w #$00FF
    STA.b $00                  ; Preserve index for multiplication by three.
    ASL A
    CLC
    ADC.b $00                  ; A = script index * 3.
    TAX
    LDA.l !CutscenePointers,X  ; Load pointer low and high bytes.
    STA.l !CutsceneCursor
    SEP #$20
    LDA.l !CutscenePointers+2,X ; Load pointer bank byte.
    STA.l !CutsceneCursor+2

.initialized
    SEP #$30                   ; Script opcodes and timers are bytes.
    LDA.l !CutsceneWait
    BEQ .execute               ; Zero means the next action may run now.
    DEC A
    STA.l !CutsceneWait
    BEQ .execute               ; Execute on the frame the timer reaches zero.
    JMP .finish                ; Otherwise only maintain screen shake.

.execute
    REP #$20
    LDA.l !CutsceneCursor      ; Copy the persistent cursor to DP indirect-long
    STA.b $07                  ; scratch for [pointer],Y reads.
    SEP #$20
    LDA.l !CutsceneCursor+2
    STA.b $09
    REP #$10                   ; Y is the byte offset within this execution run.
    LDY.w #$0000

.next_action
    LDA.b [$07],Y              ; Read an opcode and advance to its operands.
    INY
    CMP.b #$00                 ; $00: end
    BNE +
    JMP .end
+   CMP.b #$01                 ; $01: wait frames
    BNE +
    JMP .wait
+   CMP.b #$02                 ; $02: play sound
    BNE +
    JMP .play_sound
+   CMP.b #$03                 ; $03: play music
    BNE +
    JMP .play_music
+   CMP.b #$04                 ; $04: draw Map16 writes
    BNE +
    JMP .draw
+   CMP.b #$05                 ; $05: mark the current area complete
    BNE +
    JMP .set_complete
+   CMP.b #$06                 ; $06: start screen shake
    BNE +
    JMP .start_shake
+   JMP .stop_shake            ; $07: stop screen shake

; Pause script execution for the encoded number of frames. Save the cursor
; after the operand so execution resumes at the following opcode.
.wait
    LDA.b [$07],Y              ; Load the wait duration.
    STA.l !CutsceneWait
    INY                        ; Advance past the duration byte.
    REP #$20
    TYA                        ; Convert the relative Y cursor to an absolute
    CLC
    ADC.b $07                  ; address within the current data bank.
    STA.l !CutsceneCursor
    SEP #$20
    LDA.b $09                  ; The script records never cross a bank.
    STA.l !CutsceneCursor+2
    JMP .finish

; Play a sound through one of the three audio-engine sound-effect queues.
.play_sound
    LDA.b [$07],Y              ; Encoded channels are one-based.
    INY
    DEC A                      ; Convert channel 1-3 to queue offset 0-2.
    SEP #$10
    TAX
    LDA.b [$07],Y              ; Read the sound ID.
    INY
    STA.w $012D,X              ; Queue it in SFX1, SFX2, or SFX3.
    REP #$10                   ; Restore 16-bit Y for the script cursor.
    JMP .next_action

; Queue a song command for the audio engine.
.play_music
    LDA.b [$07],Y              ; Read and consume the song ID.
    INY
    STA.w $012C                ; Queue it in the music command byte.
    JMP .next_action

; Apply an encoded list of Map16 changes immediately. DrawPersistentMap16
; updates the $7E2000 Map16 buffer and appends the corresponding VRAM stripes.
.draw
    LDA.b [$07],Y              ; Read the byte-sized write count.
    STA.b $0E
    STZ.b $0F                  ; Extend it for a 16-bit decrement below.
    INY                        ; Advance to the first offset/ID pair.
    REP #$30                   ; Offsets, IDs, X, and Y are all 16-bit.

.next_write
    LDA.b [$07],Y              ; Read the $7E2000-relative Map16 offset.
    TAX
    INY
    INY
    LDA.b [$07],Y              ; Read the generated Map16 ID.
    INY
    INY
    PHY                        ; Vanilla drawing uses Y for its stripe buffer.
    JSL $9BC97C                ; DrawPersistentMap16
    PLY                        ; Restore the script cursor.
    DEC.b $0E                  ; Continue until all pairs are consumed.
    BNE .next_write

    SEP #$20                   ; Opcodes are bytes; keep Y 16-bit.
    LDA.b #$01
    STA.b $14                  ; Ask NMI to upload the queued stripes.
    JMP .next_action

; Set event bit $20 for the current overworld area. This makes the final
; generated overlay persistent when the area is loaded again.
.set_complete
    SEP #$30                   ; Area IDs and event flags are bytes.
    LDX.b $8A                  ; Index the per-area event-state table.
    LDA.l $7EF280,X
    ORA.b #$20
    STA.l $7EF280,X
    REP #$10                   ; Restore 16-bit Y for the script cursor.
    JMP .next_action

; Enable the per-frame alternating BG1 scroll offsets below.
.start_shake
    LDA.b #$01
    STA.l !CutsceneShake
    JMP .next_action

; Disable shaking and immediately return every BG1 offset to zero.
.stop_shake
    LDA.b #$00
    STA.l !CutsceneShake
    STZ.w $011A                ; BG1 horizontal offset, low byte.
    STZ.w $011B                ; BG1 horizontal offset, high byte.
    STZ.w $011C                ; BG1 vertical offset, low byte.
    STZ.w $011D                ; BG1 vertical offset, high byte.
    JMP .next_action

; Terminate the entrance cutscene and restore normal overworld processing.
.end
    SEP #$30                   ; All state cleared here is byte-sized.
    STZ.w $04C6                ; Clear the entrance-cutscene selector.
    STZ.b $B0                  ; Reset the overworld sub-submodule.
    STZ.b $C8                  ; Clear vanilla transition scratch state.
    STZ.w $0710                ; Re-enable normal OAM processing.
    STZ.w $02E4                ; Release the cutscene lock.
    STZ.w $0FC1                ; Resume sprite processing.
    LDA.b #$00
    STA.l !CutsceneCursor      ; Forget the script position so the next
    STA.l !CutsceneCursor+1
    STA.l !CutsceneCursor+2    ; cutscene starts from its own first opcode.
    STA.l !CutsceneWait        ; Clear any outstanding wait.
    STA.l !CutsceneShake       ; Disable shake state.
    STZ.w $011A                ; Restore neutral BG1 horizontal and vertical
    STZ.w $011B
    STZ.w $011C
    STZ.w $011D                ; offsets before returning to gameplay.

; Finish the current frame. When shaking is active, alternate opposite
; one-pixel BG1 horizontal and vertical offsets on even and odd frames.
.finish
    SEP #$30                   ; Read byte-sized shake and frame state.
    LDA.l !CutsceneShake
    BEQ .done                  ; No shake means there is no per-frame work.
    LDA.b $1A                  ; Use frame parity to choose shake direction.
    LSR A
    BCS .odd_shake

    REP #$20                   ; Write signed 16-bit BG1 offsets.
    LDA.w #$0001
    STA.w $011A                ; Shift BG1 one pixel right.
    LDA.w #$FFFF
    STA.w $011C                ; Shift BG1 one pixel up.
    BRA .shake_done

.odd_shake
    REP #$20
    LDA.w #$FFFF
    STA.w $011A                ; Shift BG1 one pixel left.
    LDA.w #$0001
    STA.w $011C                ; Shift BG1 one pixel down.

.shake_done
    SEP #$30                   ; Restore the caller's expected register widths.

.done
    RTL

; Apply an area's completed-event overlay while its Map16 buffer is loading.
; Vanilla first produces the normal result for the current area. The shared
; processor then overwrites affected cells when this theme has replacement
; data, without queuing redundant VRAM stripes during the load.
ApplyGeneratedOverworldOverlay:
    JSL $87FAE2                ; ApplyOverworldOverlay

    REP #$20                   ; Store the processor's 16-bit mode flag.
    STZ.b $0A                  ; Zero selects Map16-buffer-only writes.
    BRA ProcessGeneratedOverworldOverlay

; Apply the current area's generated overlay while it is already visible.
; The shared processor updates the Map16 buffer and queues each changed cell
; for VRAM upload. This is the generic entry point for live overworld events.
DrawGeneratedOverworldOverlay:
    REP #$20                   ; Store the processor's 16-bit mode flag.
    LDA.w #$0001
    STA.b $0A                  ; Nonzero selects immediate drawing.

; Read and apply the generated overlay record for the current area in $8A.
; Entry: $0A word is zero for buffer-only or nonzero for immediate drawing.
; Exit: A, X, and Y are 8-bit. A missing pointer is a successful no-op.
ProcessGeneratedOverworldOverlay:
    REP #$30                   ; Use words for pointers, offsets, IDs, and Y.
    LDA.b $8A                  ; Load the current overworld area ID.
    AND.w #$00FF
    STA.b $00                  ; Preserve it for multiplication by three.
    ASL A
    CLC
    ADC.b $00                  ; A = area ID * 3.
    TAX
    LDA.l !OverworldOverlayPointers,X ; Load pointer low and high bytes.
    STA.b $07                  ; Store them in DP indirect-long scratch.
    SEP #$20
    LDA.l !OverworldOverlayPointers+2,X ; Load the pointer bank byte.
    BEQ .no_generated_overlay  ; A zero bank marks an empty table entry.
    STA.b $09

    REP #$10                   ; Y is the byte cursor within the record.
    LDY.w #$0000
    LDA.b [$07],Y              ; Read the byte-sized write count.
    STA.b $0E
    STZ.b $0F                  ; Extend it for a 16-bit decrement below.
    INY                        ; Advance to the first offset/ID pair.
    REP #$20                   ; Map16 offsets and IDs are words.

.next_overlay_write
    LDA.b [$07],Y              ; Read the $7E2000-relative Map16 offset.
    TAX
    INY
    INY
    LDA.b [$07],Y              ; Read the generated Map16 ID.
    STA.l $7E2000,X            ; Always update the logical Map16 buffer.
    PHA                        ; Preserve the ID while testing draw mode.
    LDA.b $0A
    BEQ .skip_draw             ; Area loading needs no immediate VRAM update.
    PLA                        ; Restore the Map16 ID expected by DrawMap16.
    PHY                        ; Preserve the generated-record cursor.
    JSL $9BC980                ; DrawMap16: queue this cell's VRAM stripes.
    PLY
    BRA .write_done

.skip_draw
    PLA                        ; Balance the saved Map16 ID.

.write_done
    INY                        ; Advance past the Map16 ID.
    INY
    DEC.b $0E                  ; Continue until all pairs are consumed.
    BNE .next_overlay_write

.no_generated_overlay
    SEP #$30                   ; Restore the caller's expected register widths.
    RTL

; Finish the live Bird Statue event. The generic draw entry point replaces
; vanilla's hard-coded weather-vane Map16 IDs with this theme's generated
; overlay, then the original persistence and NMI signals are reproduced.
BreakGeneratedBirdStatue:
    JSL DrawGeneratedOverworldOverlay ; Draw all generated Bird Statue cells.

    SEP #$30                   ; Area IDs and event flags are bytes.
    LDX.b $8A                  ; Index the current area's event-state byte.
    LDA.l $7EF280,X
    ORA.b #$20                 ; Persist the completed overworld event.
    STA.l $7EF280,X
    LDA.b #$01
    STA.b $14                  ; Ask NMI to upload the queued VRAM stripes.
    RTL

; Finish the live Thieves' Town opening event. The generic draw entry point
; replaces vanilla's hard-coded entrance Map16 IDs with this theme's generated
; overlay, then the original persistence, sound, and NMI signals are reproduced.
OpenGeneratedThievesTown:
    JSL DrawGeneratedOverworldOverlay ; Draw the generated entrance cells.

    SEP #$30                   ; Area IDs and event flags are bytes.
    LDX.b $8A                  ; Index the current area's event-state byte.
    LDA.l $7EF280,X
    ORA.b #$20                 ; Persist the completed overworld event.
    STA.l $7EF280,X
    LDA.b #$1B
    STA.w $012F                ; Play the vanilla opening sound on channel 3.
    LDA.b #$01
    STA.b $14                  ; Ask NMI to upload the queued VRAM stripes.
    RTL

assert pc() <= !free_space_bank_a0_end
