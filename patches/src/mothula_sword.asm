; Treat L4 as L3 when selecting sword damage against Mothula.
lorom

!free_space_bank_a0_start = $A0F040
!free_space_bank_a0_end = $A0F060

; A/X/Y are 8-bit and X is still the sprite slot here. X/Y, DB, and scratch
; RAM are untouched; subsequent damage lookup replaces the carry flag.
org $86ED55
hook_select_sword_damage:
    JSL LoadMothulaSwordLevel

org !free_space_bank_a0_start
LoadMothulaSwordLevel:
    LDA.w $0E20,X
    CMP.b #$88
    BNE .load_sword_level

    LDA.l $7EF359
    CMP.b #$04
    BNE .return

    LDA.b #$03
    RTL

.load_sword_level
    LDA.l $7EF359 ; Run hi-jacked instruction
.return
    RTL

assert pc() <= !free_space_bank_a0_end
