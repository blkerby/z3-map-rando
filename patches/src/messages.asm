; Custom English text in the native uppercase font. Message IDs from
; !FirstCustomMessage select entries of CustomMessages; the vanilla start-menu
; IDs are replaced by text matching the available restart destinations.
lorom
incsrc "symbols.inc"

!free_space_bank_aa_start = $AA8000
!free_space_bank_aa_end = $AA9000

'A' = $AA : 'B' = $AB : 'C' = $AC : 'D' = $AD : 'E' = $AE : 'F' = $AF
'G' = $B0 : 'H' = $B1 : 'I' = $B2 : 'J' = $B3 : 'K' = $B4 : 'L' = $B5
'M' = $B6 : 'N' = $B7 : 'O' = $B8 : 'P' = $B9 : 'Q' = $BA : 'R' = $BB
'S' = $BC : 'T' = $BD : 'U' = $BE : 'V' = $BF : 'W' = $C0 : 'X' = $C1
'Y' = $C2 : 'Z' = $C3 : ' ' = $FF : '?' = $C6 : '!' = $C7 : ',' = $C8
'.' = $CD

; Text commands.
!Row2 = $F8
!Row3 = $F9
!WaitForKey = $FA
!Scroll = $F6
!End = $FB
!Arrow = $E4
!ChooseLowerTwo = $FE,$68

; The parser owns 16-bit A/X/Y and $04-$07.
org $8EEE88
hook_parse_message:
    JML ResolveMessage
org $8EEE98
return_parse_message:

org !free_space_bank_aa_start

ResolveMessage:
    LDA.w $1CF0
    CMP.w #!FirstCustomMessage
    BCS .custom_id
    CMP.w #$0182
    BEQ .two_choices
    CMP.w #$0183
    BNE .vanilla
    LDA.w #SpawnMessageHouseSanctuaryMountain
    BRA .custom

.custom_id
    SBC.w #!FirstCustomMessage
    ASL A
    TAX
    LDA.l CustomMessages,X
    BRA .custom

.two_choices
    LDA.l $7EF3CA
    AND.w #$00FF
    BNE .pyramid
    LDA.l !SpawnVisitFlags
    BIT.w #$0002
    BEQ .sanctuary
    LDA.w #SpawnMessageHouseMountain
    BRA .custom

.sanctuary
    LDA.w #SpawnMessageHouseSanctuary
    BRA .custom

.pyramid
    LDA.w #SpawnMessageHousePyramid

.custom
    STA.b $04
    LDA.w #ResolveMessage>>16
    STA.b $06
    JML return_parse_message

.vanilla
    ; Run hi-jacked instructions:
    LDA.w $1CF0
    ASL A
    TAX
    LDA.l $7F71C0,X
    STA.b $04
    LDA.w #$001C
    STA.b $06

    JML return_parse_message

; Indexed by message ID minus !FirstCustomMessage, in the order of symbols.inc.
CustomMessages:
    dw MessageLakeHyliaFairyOffer
    dw MessageLakeHyliaFairyWish
    dw MessageLakeHyliaFairyNoBottle
    dw MessageFairyBoomerangOffer, MessageFairyBoomerangMissing, MessageFairyBoomerangUpgrade, MessageFairyBoomerangReturn
    dw MessageFairyMailOffer, MessageFairyMailMissing, MessageFairyMailUpgrade, MessageFairyMailReturn
    dw MessageFairyShieldOffer, MessageFairyShieldMissing, MessageFairyShieldUpgrade, MessageFairyShieldReturn
    dw MessageFairySwordOffer, MessageFairySwordMissing, MessageFairySwordUpgrade, MessageFairySwordReturn
    dw MessageFairyBowOffer, MessageFairyBowMissing, MessageFairyBowUpgrade, MessageFairyBowReturn
    dw MessageFairyGlovesOffer, MessageFairyGlovesMissing, MessageFairyGlovesUpgrade, MessageFairyGlovesReturn
    dw MessageFairyBombsOffer, MessageFairyBombsMissing, MessageFairyBombsUpgrade, MessageFairyBombsReturn
    dw MessageFairyArrowsOffer, MessageFairyArrowsMissing, MessageFairyArrowsUpgrade, MessageFairyArrowsReturn

; Start-menu labels substitute the saved player name in the house option.
macro spawn_message_house()
    db $FE, $6D, $00, $FC, $00, !Arrow, $FE, $6A
    db $D8, "S HOUSE" ; 'S HOUSE
    db !Row2, " "
endmacro

SpawnMessageHouseSanctuary:
    %spawn_message_house()
    db "SANCTUARY", $FE, $72, !End

SpawnMessageHouseMountain:
    %spawn_message_house()
    db "MOUNTAIN", $FE, $72, !End

SpawnMessageHouseSanctuaryMountain:
    %spawn_message_house()
    db "SANCTUARY", !Row3, " "
    db "MOUNTAIN", $FE, $71, !End

SpawnMessageHousePyramid:
    %spawn_message_house()
    db "PYRAMID", $FE, $72, !End

; Rows hold 14 characters. Choice text starts in column 4, after the arrow
; that the vanilla choice messages redraw in column 2.
MessageLakeHyliaFairyOffer:
    db "THROW IN A", !Row2
    db "BOTTLE AND", !Row3
    db "YOUR WISHES", !WaitForKey, !Scroll
    db "WILL COME TRUE", !Scroll
    db "  ", !Arrow, " YES", !Scroll
    db "    NO", !ChooseLowerTwo, !End

MessageLakeHyliaFairyWish:
    db "WISH FOR", !Row2
    db "  ", !Arrow, " FAIRY", !Row3
    db "    GOLDEN BEE", !ChooseLowerTwo, !End

MessageLakeHyliaFairyNoBottle:
    db "COME BACK WHEN", !Row2
    db "YOU HAVE AN", !Row3
    db "EMPTY BOTTLE.", !End

; Waterfall and Pyramid fairy messages for each item class: the offer, the
; missing prerequisite, and the reply after the item is thrown.
macro fairy_offer(class, name)
MessageFairy<class>Offer:
    db "THROW <name>?", !Row2
    db "  ", !Arrow, " YES", !Row3
    db "    NO", !ChooseLowerTwo, !End
endmacro

macro fairy_replies(class, name, prerequisite)
MessageFairy<class>Missing:
    db "COME BACK WHEN", !Row2
    db "YOU HAVE", !Row3
    db "<prerequisite>.", !End

MessageFairy<class>Upgrade:
    db "I SEE YOU", !Row2
    db "DROPPED YOUR", !Row3
    db "<name>.", !WaitForKey, !Scroll
    db "I WILL GIVE", !Scroll
    db "YOU SOMETHING", !Scroll
    db "BETTER.", !End

MessageFairy<class>Return:
    db "I SEE YOU", !Row2
    db "DROPPED YOUR", !Row3
    db "<name>.", !WaitForKey, !Scroll
    db "I WILL GIVE IT", !Scroll
    db "BACK.", !End
endmacro

; "THROW BOOMERANG?" does not fit in one row.
MessageFairyBoomerangOffer:
    db "THROW YOUR", !Row2
    db "BOOMERANG?", !Row3
    db "  ", !Arrow, " YES", !Scroll
    db "    NO", !ChooseLowerTwo, !End
%fairy_replies(Boomerang, "BOOMERANG", "A BOOMERANG")
%fairy_offer(Mail, "MAIL")
%fairy_replies(Mail, "MAIL", "BLUE MAIL")
%fairy_offer(Shield, "SHIELD")
%fairy_replies(Shield, "SHIELD", "A SHIELD")
%fairy_offer(Sword, "SWORD")
%fairy_replies(Sword, "SWORD", "A SWORD")
%fairy_offer(Bow, "BOW")
%fairy_replies(Bow, "BOW", "A BOW")
%fairy_offer(Gloves, "GLOVES")
%fairy_replies(Gloves, "GLOVES", "GLOVES")
%fairy_offer(Bombs, "BOMBS")
%fairy_replies(Bombs, "BOMBS", "BOMBS")
%fairy_offer(Arrows, "ARROWS")
%fairy_replies(Arrows, "ARROWS", "ARROWS")

assert pc() <= !free_space_bank_aa_end
