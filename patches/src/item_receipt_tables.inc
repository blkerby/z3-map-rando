; Unified 256-entry receipt tables. The patcher copies the vanilla prefixes
; from the input ROM; ASM writes only the custom entries below.
; Native receipt initialization uses DB=$B1. Its unchanged dungeon-mask and
; bottle lookup tables are mirrored at $B185C6/$B1891E by the patcher.

!free_space_bank_b1_start = $B18000
!free_space_bank_b1_end = $B20000

org !free_space_bank_b1_start
assert pc() <= !ItemReceiptTables
org !ItemReceiptTables
ItemReceiptTables:
assert ItemReceiptTables == !ItemReceiptTables
ItemReceiptOffsetsY:     skip $100
ItemReceiptOffsetsX:     skip $100
ItemReceiptGraphics:     skip $100
ItemReceiptWidths:       skip $100
ItemReceiptPalettes:     skip $100
ItemReceiptDestinations: skip $200
ItemReceiptValues:       skip $100
ItemReceiptMessages:     skip $200
assert ItemReceiptValues == !ItemReceiptValues

; Graphics IDs index 128-byte entries of four 4bpp tiles. The patcher supplies
; the vanilla entries; custom artwork follows in the same format.
assert pc() <= !ItemGraphics
org !ItemGraphics
ItemGraphics:
assert ItemGraphics == !ItemGraphics
skip ($39*$80)
HalfMagicGraphics:
    incsrc "half_magic_graphics.inc"
BombCapacityUpgradeGraphics:
    incsrc "bomb_capacity_upgrade_graphics.inc"
ArrowCapacityUpgradeGraphics:
    incsrc "arrow_capacity_upgrade_graphics.inc"
assert pc() <= !free_space_bank_b1_end

org ItemReceiptOffsetsY+$4E
    db $FC
org ItemReceiptOffsetsX+$4E
    db $00
org ItemReceiptGraphics+$4E
    db $39
org ItemReceiptWidths+$4E
    db $02
org ItemReceiptPalettes+$4E
    db $04
org ItemReceiptDestinations+($4E*2)
    dw $F37B
org ItemReceiptValues+$4E
    db $01
org ItemReceiptMessages+($4E*2)
    dw $010F

; Capacity upgrades write nothing through the generic table; see
; capacity_upgrades.asm. They show no message.
org ItemReceiptOffsetsY+$51
    db $FC
org ItemReceiptOffsetsX+$51
    db $00
org ItemReceiptGraphics+$51
    db $3A
org ItemReceiptWidths+$51
    db $02
org ItemReceiptPalettes+$51
    db $02
org ItemReceiptDestinations+($51*2)
    dw $F370
org ItemReceiptValues+$51
    db $FF
org ItemReceiptMessages+($51*2)
    dw $FFFF

org ItemReceiptOffsetsY+$53
    db $FC
org ItemReceiptOffsetsX+$53
    db $00
org ItemReceiptGraphics+$53
    db $3B
org ItemReceiptWidths+$53
    db $02
org ItemReceiptPalettes+$53
    db $04
org ItemReceiptDestinations+($53*2)
    dw $F371
org ItemReceiptValues+$53
    db $FF
org ItemReceiptMessages+($53*2)
    dw $FFFF

; Long indexed reads can address the replacement tables directly.
org $86C4DF
    dl ItemReceiptPalettes
org $86C4EF
    dl ItemReceiptWidths
org $87B58A
    dl ItemReceiptDestinations
org $87B590
    dl ItemReceiptDestinations+1
org $88C6DF
    dl ItemReceiptWidths

; These absolute indexed reads run inside the receipt initializer with DB=$B1.
org $898612
    dw ItemReceiptDestinations&$FFFF
org $898617
    dw (ItemReceiptDestinations+1)&$FFFF
org $898620
    dw ItemReceiptValues&$FFFF
org $898628
    dw ItemReceiptDestinations&$FFFF
org $89862D
    dw (ItemReceiptDestinations+1)&$FFFF
org $89876A
    dw ItemReceiptGraphics&$FFFF
org $89884E
    dw ItemReceiptOffsetsY&$FFFF
org $89885C
    dw ItemReceiptOffsetsX&$FFFF
org $8988B8
    dw ItemReceiptWidths&$FFFF
org $8988F0
    dw ItemReceiptOffsetsY&$FFFF
org $898909
    dw ItemReceiptOffsetsX&$FFFF
