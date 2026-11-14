; Great Fairy ponds (sprite $72). The Lake Hylia fairy exchanges an empty bottle
; for a bottled fairy or golden bee, any number of times. The Waterfall and
; Pyramid fairies each upgrade one item class once (see plans/fairies.md).
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0FC00
!free_space_bank_a0_end = $A0FF00

; Receipts refilling an empty bottle (see item_receipt_tables.inc).
!FairyRefillReceipt = $B8
!GoldenBeeRefillReceipt = $B9

; Sprite $72 runs its pond states with A8/X8/Y8 and X holding its slot.
org $86C431
hook_run_upgrade_fairy:
    JSL RunUpgradeFairy
    RTS

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

; $0D80,X is the state, following the vanilla sequence: 0 waits for Link, 1
; answers the offer, 2 waits for the thrown item and summons the fairy, 3 fades
; her in and replies, 7 begins her retreat after the reply, and 8 fades her out
; and grants any upgrade. States 5 and 6 are unused, since the pond draws
; vanilla's held item in them. $0DE0,X is the fairy (0 Waterfall, 1 Pyramid),
; $0DB0,X her item class, $0EB0,X nonzero for an upgrade, and $0E90,X the
; visible fairy's slot. DB is the sprite bank. Preserve X; the vanilla calls
; clobber A/Y.
RunUpgradeFairy:
    LDA.w $0D80,X
    BEQ .wait_for_link
    CMP.b #$01
    BEQ .answer_offer
    CMP.b #$02
    BEQ .summon_fairy
    CMP.b #$03
    BEQ .fade_in
    CMP.b #$07
    BNE .fade_out
    JSR StartFairyRetreat
    INC.w $0D80,X
    RTL

.wait_for_link
    STZ.w $02E4
    LDA.w $0DF0,X
    BNE .exit
    JSL $87F4AA ; CheckIfLinkIsBusy
    BCS .exit
    ; Without the item, skip the offer and ask Link to come back.
    JSR LoadFairyClass
    JSR LoadFairyLevel
    BEQ .missing_item
    LDA.b #$00
    BRA .greet
.missing_item
    LDA.b #$01
.greet
    PHA
    JSR LoadFairyMessage
    JSL $85E1F0 ; ShowMessageOnContact
    PLA
    BCC .exit
    PHA
    JSL $87F18C ; ResetLinkProperties_A
    JSL $89ADB3 ; TerminateSparkles
    STZ.b $2F
    PLA
    BNE .finish
    INC.w $0D80,X
.exit
    RTL

.answer_offer
    LDA.w $1CE8
    BNE .finish
    JSR LoadFairyLevel
    JSR ThrowFairyItem
    INC.w $0D80,X
    RTL

.summon_fairy
    LDA.w $0DF0,X
    BNE .exit
    JSR SpawnUpgradeFairy
    INC.w $0D80,X
    RTL

.fade_in
    JSR FadeFairy
    BNE .exit
    STZ.w $0EB0,X
    JSR CheckFairyUpgrade
    LDA.b #$03
    BCC .reply
    INC.w $0EB0,X
    LDA.b #$02
.reply
    JSR ShowFairyMessage
    JSR EndFairyTranslucency
    LDA.b #$07
    STA.w $0D80,X
    RTL

.fade_out
    JSR FadeFairy
    BNE .exit
    PHX
    JSL $80EC22 ; PaletteFilter_RestoreSprite5Left
    PLX
    JSR EndFairyTranslucency
    LDA.w $0EB0,X
    BEQ .finish
    PHX
    LDA.w $0DE0,X
    TAX
    LDA.l FairyUsedFlags,X
    ORA.l !EventFlags
    STA.l !EventFlags
    STZ.w $02E9
    LDA.l !FairyItems,X
    TAY
    JSL $87999D ; GrantItemReceipt
    PLX

.finish
    STZ.w $0D80,X
    LDA.b #$FF
    STA.w $0DF0,X
    RTL

; Summon the visible fairy above the pond, with her music and translucency.
SpawnUpgradeFairy:
    LDA.b #$72
    JSL $9DF65D ; SpawnSpriteDynamically
    LDA.b #$1B
    STA.w $012C
    STZ.w $0133
    LDA.b $00
    STA.w $0D10,Y
    LDA.b $01
    STA.w $0D30,Y
    LDA.b $02
    SEC
    SBC.b #$50
    STA.w $0D00,Y
    LDA.b $03
    SBC.b #$00
    STA.w $0D20,Y
    LDA.b #$01
    STA.w $0DA0,Y
    TYA
    STA.w $0E90,X
    PHX
    JSL $82FD04 ; AssertTranslucencySwap
    JSL $80EBF5 ; PaletteFilter_WishPonds
    PLX
    RTS

; Step the fairy's palette fade every eighth frame, removing her when the
; fade-out (state 8) passes $1E. Z is set once the fade is complete.
FadeFairy:
    LDA.b $1A
    AND.b #$07
    BNE .pending
    PHX
    JSL $80EC3D ; PaletteFilter_Sprite5Left
    PLX
    LDA.w $0D80,X
    CMP.b #$08
    BNE .check_done
    LDA.l $7EC007
    CMP.b #$1E
    BNE .done
    LDY.w $0E90,X
    LDA.b #$00
    STA.w $0DD0,Y
    LDA.b #$1E
    BRA .done
.check_done
    LDA.l $7EC007
.done
    CMP.b #$00
    RTS
.pending
    LDA.b #$01
    RTS

; End translucency so messages are legible, as vanilla does before replying.
EndFairyTranslucency:
    PHX
    JSL $82FD68 ; RevertTranslucencySwap
    STZ.b $1D
    LDA.b #$20
    STA.b $9A
    INC.b $15
    PLX
    RTS

; Restore translucency for the fairy's fade-out.
StartFairyRetreat:
    PHX
    JSL $82FD04 ; AssertTranslucencySwap
    LDA.b #$02
    STA.b $1D
    LDA.b #$30
    STA.b $9A
    INC.b $15
    PLX
    RTS

; Store the fairy in $0DE0,X and the class of her placed receipt in $0DB0,X.
LoadFairyClass:
    LDA.l $7EF3CA
    BEQ .store_fairy
    LDA.b #$01
.store_fairy
    STA.w $0DE0,X
    PHX
    TAX
    LDA.l !FairyItems,X
    LDX.b #$07
.next_class
    CMP.l FairyClassReceipts,X
    BEQ .found
    DEX
    BNE .next_class
.found
    TXA
    PLX
    STA.w $0DB0,X
    RTS

; Return the owned level of the fairy's class in A, with Z set for none.
LoadFairyLevel:
    PHX
    LDA.w $0DB0,X
    TAX
    LDA.l FairyClassLevels,X
    TAX
    LDA.l $7EF300,X
    PLX
    CMP.b #$00
    RTS

; Carry set if the fairy's upgrade is unused and her class is not maxed.
; Clobbers A/Y.
CheckFairyUpgrade:
    PHX
    LDY.w $0DB0,X
    LDA.w $0DE0,X
    TAX
    LDA.l FairyUsedFlags,X
    AND.l !EventFlags
    BNE .unavailable
    TYX
    LDA.l FairyClassLevels,X
    TAX
    LDA.l $7EF300,X
    TYX
    CMP.l FairyClassMaxima,X
    BCS .unavailable
    PLX
    SEC
    RTS
.unavailable
    PLX
    CLC
    RTS

; Show the fairy's message of the kind in A (see !FirstFairyMessage).
ShowFairyMessage:
    JSR LoadFairyMessage
    JSL $85E219 ; ShowMessageUnconditional
    RTS

; Return the ID of the fairy's message of the kind in A, low byte in A and
; high byte in Y.
LoadFairyMessage:
    PHA
    LDA.w $0DB0,X
    ASL A
    ASL A
    CLC
    ADC.b $01,S
    ADC.b #!FirstFairyMessage&$FF
    PLY
    LDY.b #!FirstFairyMessage>>8
    RTS

; Throw the owned item, given its level in A; it keeps its inventory value.
ThrowFairyItem:
    CMP.b #$05
    BCC .clamped
    LDA.b #$04
.clamped
    PHX
    PHA
    LDA.w $0DB0,X
    ASL A
    ASL A
    ADC.w $0DB0,X
    ADC.b $01,S
    TAX
    PLA
    LDA.l FairyThrowReceipts,X
    TAX
    LDY.b #$04
    LDA.b #$28
    JSL $898A1E ; AncillaAdd_TossedPondItem
    PLX
    LDA.b #$01
    STA.w $02E4
    LDA.b #$40
    STA.w $0DF0,X
    RTS

; Item classes in the order of plans/fairies.md: boomerang, mail, shield,
; sword, bow, gloves, bombs, arrows. Each has its placed receipt, its level
; byte in $7EF300, its maximum level, and the receipts drawn when throwing
; owned levels 1-4.
FairyClassReceipts:
    db $62,$60,$5F,$5E,$64,$61,$51,$53
FairyClassLevels:
    db $41,$5B,$5A,$59,$40,$54,$70,$71
FairyClassMaxima:
    db 2,2,3,4,3,2,99,99
FairyThrowReceipts:
    db $00,$0C,$2A,$2A,$2A
    db $00,$22,$23,$23,$23
    db $00,$04,$05,$06,$06
    db $00,$49,$01,$02,$03
    db $00,$3A,$3A,$3B,$3B
    db $00,$1B,$1C,$1C,$1C
    db $00,$27,$27,$27,$27
    db $00,$43,$43,$43,$43

; !EventFlags bits of the Waterfall and Pyramid fairies' used upgrades.
FairyUsedFlags:
    db $08,$10

assert pc() <= !free_space_bank_a0_end
