; ALTTPR-compatible ammo capacity: $7EF370/$7EF371 hold the bomb/arrow
; capacities directly instead of indexes into the vanilla CapacityUpgrades
; tables. New saves start at 10 bombs and 30 arrows (see new_game.asm).
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0FB00
!free_space_bank_a0_end = $A0FC00

!CapacityIncrement = 5
!MaxCapacity = 99

; HUD refill compares current ammo against the capacity byte directly.
org $8DDC27
hook_check_bomb_refill_capacity:
    LDA.l $7EF343
    CMP.l $7EF370
    BRA return_check_bomb_refill_capacity
org $8DDC33
return_check_bomb_refill_capacity:

org $8DDC49
hook_check_arrow_refill_capacity:
    LDA.l $7EF377
    CMP.l $7EF371
    BRA return_check_arrow_refill_capacity
org $8DDC55
return_check_arrow_refill_capacity:

; Shop purchases compare the loaded capacity byte against current ammo.
org $9EE19D
hook_check_bomb_shop_capacity:
    BRA return_check_bomb_shop_capacity
org $9EE1A4
return_check_bomb_shop_capacity:

org $9EF2B6
hook_check_shop_arrow_capacity:
    BRA return_check_shop_arrow_capacity
org $9EF2BD
return_check_shop_arrow_capacity:

org $9EF2F7
hook_check_shop_bomb_capacity:
    BRA return_check_shop_bomb_capacity
org $9EF2FE
return_check_shop_bomb_capacity:

; Lake Hylia fairy purchases grant the same upgrade as the shuffled items.
; Entry is A8/X8/Y8 with X holding the fairy's sprite slot.
org $86C6AB
hook_upgrade_fairy_bombs:
    LDY.b #$51
    JSL UpgradeFairyCapacity
    BCS return_refuse_fairy_bombs
    BRA return_show_fairy_bombs
org $86C6C6
return_show_fairy_bombs:
org $86C6CF
return_refuse_fairy_bombs:

org $86C72E
hook_upgrade_fairy_arrows:
    LDY.b #$53
    JSL UpgradeFairyCapacity
    BCS return_refuse_fairy_arrows
    BRA return_show_fairy_arrows
org $86C749
return_show_fairy_arrows:
org $86C752
return_refuse_fairy_arrows:

org !free_space_bank_a0_start

; Entry is A8/X8/Y8 with receipt $51 (bombs) or $53 (arrows) in Y.
; Raise the capacity and queue the same amount of ammo. Preserve X/Y/DB.
UpgradeCapacity:
assert UpgradeCapacity == !UpgradeCapacity
    PHX
    ; Bit 1 of the receipt selects the bomb or arrow byte of each pair.
    TYA
    AND.b #$02
    LSR A
    TAX
    LDA.l $7EF370,X
    CLC
    ADC.b #!CapacityIncrement
    CMP.b #!MaxCapacity+1
    BCC .store
    LDA.b #!MaxCapacity
.store
    STA.l $7EF370,X
    LDA.l $7EF375,X
    CLC
    ADC.b #!CapacityIncrement
    STA.l $7EF375,X
    PLX
    RTL

; Entry is A8/X8/Y8 with the capacity receipt in Y. Carry set means the
; capacity is already full. Otherwise upgrade and store the new capacity as
; BCD in $1CF2 for the fairy's message. Preserve X/DB; clobbers A/Y.
UpgradeFairyCapacity:
    PHX
    TYA
    AND.b #$02
    LSR A
    TAX
    LDA.l $7EF370,X
    CMP.b #!MaxCapacity
    BCS .done
    JSL UpgradeCapacity
    LDA.l $7EF370,X
    LDY.b #$00
.tens
    CMP.b #10
    BCC .ones
    SBC.b #10
    INY
    BRA .tens
.ones
    STA.l $7E1CF2
    TYA
    ASL A
    ASL A
    ASL A
    ASL A
    ORA.l $7E1CF2
    STA.l $7E1CF2
    CLC
.done
    PLX
    RTL

assert pc() <= !free_space_bank_a0_end
