; Switch between owned items in vanilla shared menu slots with the Y button.
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0F100
!free_space_bank_a0_end = $A0F200

org $8DDF38
hook_select_inventory_item:
    JML ToggleInventoryItem
org $8DDF3E
return_delete_inventory_item:
org $8DDF49
return_move_inventory_cursor:
org $8DDF88
return_draw_inventory_items:

org !free_space_bank_a0_start

; The menu uses 8-bit A/X/Y. Preserve X/Y, DB, and the old cursor in $00.
ToggleInventoryItem:
    LDA.b $F4
    AND.b #$40
    BEQ .vanilla
    LDA.w $0202
    CMP.b #$05
    BNE .ocarina
    LDA.l !InventoryTracking
    AND.b #$30
    CMP.b #$30
    BNE .draw
    LDA.l $7EF344
    EOR.b #$03
    STA.l $7EF344
    BRA .changed
.ocarina
    CMP.b #$0D
    BNE .vanilla
    ; A paused digging animation must finish with the shovel still selected.
    LDA.w $037A
    CMP.b #$01
    BEQ .draw
    LDA.l !InventoryTracking
    BIT.b #$04
    BEQ .draw
    AND.b #$03
    BEQ .draw
    LDA.l $7EF34C
    CMP.b #$01
    BNE .shovel
    LDA.l !InventoryTracking
    AND.b #$01
    BEQ .inactive
    LDA.b #$03
    BRA .select
.inactive
    LDA.b #$02
    BRA .select
.shovel
    LDA.b #$01
.select
    STA.l $7EF34C
.changed
    LDA.b #$10
    STA.w $0207
    LDA.b #$20
    STA.w $012F
.draw
    JML return_draw_inventory_items
.vanilla
    ; Run hi-jacked instructions:
    LDA.b $F7
    AND.b #$10
    BEQ .move

    JML return_delete_inventory_item
.move
    JML return_move_inventory_cursor

assert pc() <= !free_space_bank_a0_end
