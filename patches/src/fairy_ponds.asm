; Great Fairy ponds (sprite $72). The Lake Hylia fairy exchanges an empty bottle
; for a bottled fairy or golden bee, any number of times.
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0FC00
!free_space_bank_a0_end = $A0FF00

; Receipts refilling an empty bottle (see item_receipt_tables.inc).
!FairyRefillReceipt = $B8
!GoldenBeeRefillReceipt = $B9

; Sprite $72 runs its Lake Hylia state with A8/X8/Y8 and X holding its slot.
org $86C454
hook_run_lake_hylia_fairy:
    JSL RunLakeHyliaFairy
    RTS

org !free_space_bank_a0_start

; $0D80,X is the state: 0 waits for Link, 1 answers the offer, 2 answers the
; wish, and 3 waits for the thrown bottle. $0DF0,X is the sprite's countdown,
; and $0DC0,X holds the wished-for receipt. DB is the sprite bank, which mirrors
; low WRAM. Preserve X; the vanilla calls clobber A/Y.
RunLakeHyliaFairy:
    LDA.w $0D80,X
    BEQ .wait_for_link
    DEC A
    BEQ .answer_offer
    DEC A
    BEQ .answer_wish
    LDA.w $0DF0,X
    BNE .exit
    PHX
    STZ.w $02E9
    LDY.w $0DC0,X
    JSL $87999D ; GrantItemReceipt
    PLX
    BRA .finish

.wait_for_link
    STZ.w $02E4
    LDA.w $0DF0,X
    BNE .exit
    JSL $87F4AA ; CheckIfLinkIsBusy
    BCS .exit
    LDA.b #!MessageLakeHyliaFairyOffer
    LDY.b #!MessageLakeHyliaFairyOffer>>8
    JSL $85E1F0 ; ShowMessageOnContact
    BCC .exit
    INC.w $0D80,X
    JSL $87F18C ; ResetLinkProperties_A
    JSL $89ADB3 ; TerminateSparkles
    STZ.b $2F
.exit
    RTL

.answer_offer
    LDA.w $1CE8
    BNE .finish
    LDA.b #!MessageLakeHyliaFairyWish
    LDY.b #!MessageLakeHyliaFairyWish>>8
    JSL $85E219 ; ShowMessageUnconditional
    INC.w $0D80,X
    RTL

.answer_wish
    LDA.b #!FairyRefillReceipt
    LDY.w $1CE8
    BEQ .store_wish
    LDA.b #!GoldenBeeRefillReceipt
.store_wish
    STA.w $0DC0,X
    PHX
    LDX.b #$03
.check_bottle
    LDA.l $7EF35C,X
    CMP.b #$02
    BEQ .throw_bottle
    DEX
    BPL .check_bottle
    PLX
    LDA.b #!MessageLakeHyliaFairyNoBottle
    LDY.b #!MessageLakeHyliaFairyNoBottle>>8
    JSL $85E219 ; ShowMessageUnconditional
    BRA .finish

.throw_bottle
    ; The bottle keeps its inventory value until the refill receipt.
    LDX.b #$16
    LDY.b #$04
    LDA.b #$28
    JSL $898A1E ; AncillaAdd_TossedPondItem
    PLX
    LDA.b #$01
    STA.w $02E4
    LDA.b #$40
    STA.w $0DF0,X
    INC.w $0D80,X
    RTL

.finish
    STZ.w $0D80,X
    LDA.b #$FF
    STA.w $0DF0,X
    RTL

assert pc() <= !free_space_bank_a0_end
