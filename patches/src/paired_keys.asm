; The patcher gives each modeled door a bit in the save file. Chest records
; and loose-key rooms name that bit, as do both native faces of a locked door.
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0B930
!free_space_bank_a0_end = $A0BA30
!OwnedKeys = $7EF390

org $81EC07
hook_open_key_chest:
    JSL ReadChestItem

; Both loose-key paths have just incremented the HUD count in A8.
org $86D192
hook_collect_loose_key:
    JSL CollectLooseKey
org $85FC83
hook_collect_torch_key:
    JSL CollectLooseKey

org $81CF59
hook_unlock_small_key_door:
    JML CheckPairedKey
org $81CF19
return_check_big_key:
org $81CF62
return_spend_key:
org $81CF41
return_leave_door_locked:
org $81CF36
return_display_missing_key:

org !free_space_bank_a0_start

; A/X/Y are 16-bit. X indexes the vanilla chest record and must survive.
ReadChestItem:
    LDA.l $81E96E,X ; Run hi-jacked instruction
    PHA
    AND.w #$00FF
    CMP.w #$0024
    BNE .done
    PHX
    PHY
    SEP #$20
    LDA.l !ChestKeyDoors,X
    SEP #$10
    JSR CollectPairedKey
    STA.l !ChestKeyMessage
    REP #$10
    PLY
    PLX
.done
    PLA
    RTL

; A/X/Y are 8-bit. The native routine still needs its sprite index for
; the room pickup flag and the torch's sound. The message routine uses A8.
CollectLooseKey:
    STA.l $7EF36F ; Run hi-jacked instruction
    PHX
    PHY
    LDX.b $A0
    LDA.l !LooseKeyDoors,X
    JSR CollectPairedKey
    STA.w $1CF0
    SEP #$20
    JSL $9CFD69 ; PrepAndDisplayMessage
    PLY
    PLX
    RTL

; A8 = door index. Return its A16 receipt message and record ownership.
CollectPairedKey:
    PHA
    JSR GetKeyMask
    ORA.l !OwnedKeys,X
    STA.l !OwnedKeys,X
    PLA
    REP #$20
    AND.w #$00FF
    CLC
    ADC.w #!FirstPairedKeyMessage
    RTS

; A8/X8/Y8. Return the byte offset in X and the bit mask in A.
GetKeyMask:
    PHA
    LSR #3
    TAY
    PLA
    AND.b #$07
    TAX
    LDA.l KeyMasks,X
    TYX
    RTS

KeyMasks:
    db $01,$02,$04,$08,$10,$20,$40,$80

; A/X/Y are 16-bit. X is the collision tile used by the opening animation.
; Y is twice the door slot. Opposite faces within a room share its save bit.
; $00 held the tile coordinate and is dead after the vanilla collision lookup.
CheckPairedKey:
    PHX
    LDA.b $A0
    ASL #3
    STA.b $00
    TYA
    AND.w #$0006
    ORA.b $00
    LSR
    TAX
    SEP #$20
    LDA.l !PairedKeyLocks,X
    CMP.b #$FF
    BEQ .big_key
    SEP #$10
    JSR GetKeyMask
    AND.l !OwnedKeys,X
    REP #$30
    BEQ .missing
    PLX
    JML return_spend_key

.big_key
    REP #$20
    PLX
    JML return_check_big_key

.missing
    PLX
    LDA.w $04B8
    BNE .latched
    INC.w $04B8
    LDA.w #!MessageWrongKey
    JML return_display_missing_key
.latched
    JML return_leave_door_locked

assert pc() <= !free_space_bank_a0_end
