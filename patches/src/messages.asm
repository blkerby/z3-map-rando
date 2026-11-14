; Custom English text in the native uppercase font. Message IDs from
; !FirstCustomMessage select entries of CustomMessages; the vanilla start-menu
; IDs are replaced by text matching the available restart destinations.
lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A09900
!free_space_bank_a0_end = $A09E00

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

org !free_space_bank_a0_start

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

assert pc() <= !free_space_bank_a0_end
