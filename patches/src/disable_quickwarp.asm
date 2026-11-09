; Disable vanilla quickwarps by requiring every wave sample to be flat.
; This patch is not applied in the randomizer, preserving the speedrun mechanic.

lorom

!free_space_bank_a0_start = $A080FE
!free_space_bank_a0_end = $A08180

; OR can equal the base scroll while individual samples still contain a wave.
org $80FF7C
hook_mirror_wave_completion:
    JML CheckMirrorWaveComplete
org $80FF8C
return_mirror_wave_complete:
org $80FFB4
return_mirror_wave_active:

org !free_space_bank_a0_start

; Entry is A16/X16/Y16. Only A and comparison flags change.
CheckMirrorWaveComplete:
    LDA.b $E2
    CMP.w $1C80
    BNE .active
    CMP.w $1C90
    BNE .active
    CMP.w $1CA0
    BNE .active
    CMP.w $1CB0
    BNE .active
    JML return_mirror_wave_complete
.active
    JML return_mirror_wave_active

assert pc() <= !free_space_bank_a0_end
