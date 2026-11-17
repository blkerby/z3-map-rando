; Based on ALTTPR's accessibility patch:
; https://github.com/KatDevsGames/z3randomizer/blob/dcb0a2b42d14445f7994a0e9e4d63cbecf4b98d3/accessibility.asm
;
; Suppress weather, spell, electrical, cutscene, and boss palette flashes.
; Dark Mountain and Ganon's Tower palette effects are disabled separately by
; overworld_lightning.asm because they conflict with generated theme palettes.
lorom

!free_space_bank_a0_start = $A09B00
!free_space_bank_a0_end = $A09B80

; OverworldOverlay_HandleRain: half color math, subtraction on CG0, OBJ, BG2.
org $82A3FD
hook_set_rain_lightning_color_math:
    LDA.b #$72

; RefreshEquipmentPalettes_zap: retain the upload request without replacing
; sword, shield, or mail colors. Both callers enter with 8-bit A/X/Y.
org $82FD7E
hook_flash_electrical_equipment:
    INC.b $15
    RTL

; PaletteFilter_Oversaturation is shared by Ether, Agahnim's lightning and
; Zelda cutscene, Vitreous, the Magic Bat, and the attract-mode cutscene.
org $82FE04
hook_flash_background:
    JML RevealInvisibleFloor

; The floor-only filter calls this color helper with JSL. Its vanilla JSR
; callers are all inside the replaced PaletteFilter_Oversaturation body.
org $82FEE6
    RTL

; Ether's normal-color phase must leave the revealed floor colors intact.
; The surrounding PHX/PLX and normal equipment palette load still run.
org $88AAF9
hook_restore_ether_background:
    BRA return_ether_palette_restore

org $88AAFD
return_ether_palette_restore:

; Advance the pedestal effect's timers, but skip its flashing OAM draw.
org $88C29E
hook_draw_master_sword_effect:
    BRA return_master_sword_effect

org $88C300
return_master_sword_effect:

; LayerEffect_Agahnim2 still installs its layer configuration before returning.
org $8AFF0D
hook_run_agahnim_room_lightning:
    BRA return_agahnim_room_layers

org $8AFF43
return_agahnim_room_layers:

org !free_space_bank_a0_start

; Match ALTTPR's invisible-floor exceptions without whitening other colors.
; A/X/Y are 8-bit on entry and return; X/Y/DB are preserved. The color helper
; uses the same $00/$02/$04 scratch as the vanilla background flash.
RevealInvisibleFloor:
    LDA.b $10
    CMP.b #$07
    BNE .done

    REP #$20
    LDA.b $A0
    CMP.w #$003C              ; Hookshot Cave.
    BEQ .reveal
    CMP.w #$009D              ; GT right-side invisible floor.
    BEQ .reveal
    CMP.w #$009C              ; GT big invisible-floor room.
    BEQ .reveal
    CMP.w #$00A5              ; GT Wizzrobe room.
    BNE .done

.reveal
    LDA.l $7EC5F6
    JSL $82FEAB               ; OversaturateColor.
    STA.l $7EC5F6
    LDA.l $7EC5F8
    JSL $82FEAB
    STA.l $7EC5F8

.done
    SEP #$30
    RTL

assert pc() <= !free_space_bank_a0_end
