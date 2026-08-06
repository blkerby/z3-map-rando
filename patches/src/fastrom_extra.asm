; Based on https://raw.githubusercontent.com/codemann8/z3randomizer/e40be2a148f66c7928ef64fc04608e31e2a41753/fastrom.asm

!free_space_bank_a0_start = $A08040
!free_space_bank_a0_end = $A08080

;===================================================================================================

; InitializeMemoryAndSRAM has completed and returned in 8-bit mode. Replace
; vanilla's NMITIMEN setup with a jump that enables both FastROM and NMI,
; then continues into the FastROM mirror of the main loop.
org $80802F
    JML hook_InitializeMemoryAndSRAMAfterReturn
    NOP

assert pc() == $808034

; Hardware enters NMI through the 16-bit vector in bank $00. Jump to a
; high-bank copy before doing the substantial NMI work.
org $8080C9
    JML hook_NMIEntry

; Mark the cartridge as FastROM LoROM rather than SlowROM LoROM.
org $80FFD5
    db $30

org !free_space_bank_a0_start
hook_InitializeMemoryAndSRAMAfterReturn:
    LDA.b #$01
    STA.l $00420D

    ; Run hi-jacked instructions:
    LDA.b #$81
    STA.l $004200

    JML $808034

; Reproduce the NMI instructions displaced by the entry hook, then continue
; through the existing routine's FastROM mirror.
hook_NMIEntry:
    ; Run hi-jacked instructions:
    SEI
    REP #$30
    PHA

    JML $8080CD

assert pc() <= !free_space_bank_a0_end

;===================================================================================================

; this guy here is removed to make space for a small rewrite
; org $8CC2A8 : db $8C
; because fast rom is just too fast! and the title card relies on lag for timing

org $8CC2A5
TitleCardFix:
	; thankfully, the title screen song ends instead of loops
	; so we'll just wait for it to end
	LDA.w $2140
	BNE .exit

	LDA.b #$14
	STA.b $10

	STZ.b $11
	STZ.b $22

.exit
	JML $8CC3D2

assert pc() <= $8CC2B6

;===================================================================================================
; THESE MOTHERFUCKERS
; This routine relies on a PLB followed by a BMI to clear the N flag
; which means it doesn't work properly in fastrom.
; There is no way to fix this in vanilla space without affecting other stuff
; so it stays in slow rom.
; That is its punishment.
;===================================================================================================
; These are the vanilla values, but they're explicitly set for reference
org $81DB3A : db $06
org $87C1CB : db $06
org $9BBF13 : db $06
org $9BC1CB : db $06

;===================================================================================================

; Similar problem, except in vanilla the PLB causes the branch to always fail
; so I'll just make it always fail here too
org $9D875C
	BRA ++ : ++

;===================================================================================================
; Super secret text performance - skip decompress routine
org $8EEE58 : db $08

;===================================================================================================
