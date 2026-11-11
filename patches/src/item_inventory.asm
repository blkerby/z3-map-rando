; Track shared-slot ownership independently of the pause-menu implementation.
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0F060
!free_space_bank_a0_end = $A0F100

org $898635
hook_receive_inventory_item:
    JML GrantTrackedInventoryItem
org $89863C
return_receive_inventory_item:

org $88D395
hook_activate_ocarina:
    JSL ActivateTrackedOcarina
    NOP #2

org $85E4D7
hook_give_mushroom:
    JSL RemoveTrackedMushroom
    NOP #2

org !free_space_bank_a0_start

; A/X/Y are 8-bit, DB is the receipt-table bank, Y is the receipt, and
; $00-$02 is its destination.
; Preserve X/Y/DB and scratch RAM; only the vanilla inventory write uses $00.
GrantTrackedInventoryItem:
    CPY.b #$0C
    BNE +
    LDA.b #$80
    BRA .track
+   CPY.b #$2A
    BNE +
    LDA.b #$40
    BRA .track
+   CPY.b #$29
    BNE +
    LDA.b #$28
    BRA .track
+   CPY.b #$0D
    BNE +
    LDA.b #$10
    BRA .track
+   CPY.b #$13
    BNE +
    LDA.b #$04
    BRA .track
+   CPY.b #$14
    BNE +
    LDA.l !InventoryTracking
    BIT.b #$01
    BNE .active_ocarina
    LDA.b #$02
    BRA .track
+   CPY.b #$4A
    BNE .grant
    LDA.l !InventoryTracking
    AND.b #$FC
    ORA.b #$01
    STA.l !InventoryTracking
.active_ocarina
    LDA.b #$03
    STA.b [$00]
    JML return_receive_inventory_item
.track
    ORA.l !InventoryTracking
    STA.l !InventoryTracking
.grant
    ; Run hi-jacked instructions:
    LDA.w !ItemReceiptValues,Y
    BMI .done
    STA.b [$00]

.done
    JML return_receive_inventory_item

; Both event hooks use 8-bit A and preserve X/Y/DB and scratch RAM.
ActivateTrackedOcarina:
    LDA.l !InventoryTracking
    AND.b #$FC
    ORA.b #$01
    STA.l !InventoryTracking

    ; Run hi-jacked instructions:
    LDA.b #$03
    STA.l $7EF34C

    RTL

RemoveTrackedMushroom:
    LDA.l !InventoryTracking
    AND.b #$DF
    STA.l !InventoryTracking
    AND.b #$10
    BEQ .empty
    LDA.b #$02
.empty
    STA.l $7EF344
    RTL

assert pc() <= !free_space_bank_a0_end
