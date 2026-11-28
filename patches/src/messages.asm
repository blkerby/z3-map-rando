; Custom English text in the native uppercase font. IDs from !FirstCustomMessage
; select fixed messages, followed by the generated Small Key receipts. Vanilla
; start-menu IDs select text matching the available restart destinations.
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

; The cursor has advanced one 8-pixel tile. Skip the glyph's extra three columns
; so redrawing a character (including a choice arrow) only touches its own cell.
org $8EF372
hook_draw_message_character:
    BRA return_draw_message_character
org $8EF3D6
return_draw_message_character:

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
    CMP.w #!FirstPairedKeyMessage
    BCC .listed
    SBC.w #!FirstPairedKeyMessage
    ASL #6 ; each generated receipt occupies 64 bytes in this bank
    CLC
    ADC.w #!PairedKeyMessages&$FFFF
    BRA .custom
.listed
    SEC
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
    dw MessageFairyClosed, MessageWrongKey

MessageWrongKey:
    db "NO KEY FITS", !Row2, "THIS LOCK.", !End

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

; Rows hold 19 characters. Choice text starts in column 4, after the arrow
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

MessageFairyClosed:
    db "SORRY WE", $D8, "RE", !Row2
    db "CLOSED", !End

; Waterfall and Pyramid fairy messages for each item class: the offer, the
; missing prerequisite, and the reply after the item is thrown.
macro fairy_messages(class, name, prerequisite)
MessageFairy<class>Offer:
    db "THROW <name>?", !Row2
    db "  ", !Arrow, " YES", !Row3
    db "    NO", !ChooseLowerTwo, !End

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

%fairy_messages(Boomerang, "BOOMERANG", "A BOOMERANG")
%fairy_messages(Mail, "MAIL", "BLUE MAIL")
%fairy_messages(Shield, "SHIELD", "A SHIELD")
%fairy_messages(Sword, "SWORD", "A SWORD")
%fairy_messages(Bow, "BOW", "A BOW")
%fairy_messages(Gloves, "GLOVES", "GLOVES")
%fairy_messages(Bombs, "BOMBS", "BOMBS")
%fairy_messages(Arrows, "ARROWS", "ARROWS")

assert pc() <= !free_space_bank_aa_end

assert !PairedKeyMessages>>16 == ResolveMessage>>16
