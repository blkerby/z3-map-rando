; Expand Map16 definitions by using a separate bank for each of the four 8x8
; quadrants (giving 4x larger capacity), and migrate graphical consumers to
; read from these banks.

lorom

incsrc "symbols.inc"

; Vanilla has loaded the next horizontal-stripe Map16 ID into A.
org $82F1AD
return_CreateMap16StripesHorizontalLoop:

org $82F1B6
    ASL A
    TAX

    LDA.l !Map16TopLeft,X
    STA.w $1100,Y

    LDA.l !Map16TopRight,X
    STA.w $1142,Y

    INY
    INY

    LDA.l !Map16BottomLeft,X
    STA.w $1100,Y

    LDA.l !Map16BottomRight,X
    STA.w $1142,Y

    INY
    INY

    DEC.b $06
    BNE return_CreateMap16StripesHorizontalLoop

    TYA
    CLC
    ADC.w #$0042
    STA.b $0E

    RTS
assert pc() <= $82F1E4

; Vanilla has loaded the next vertical-stripe Map16 ID into A.
org $82F275
return_CreateMap16StripesVerticalLoop:

org $82F27E
    ASL A
    TAX

    LDA.l !Map16TopLeft,X
    STA.w $1100,Y

    LDA.l !Map16BottomLeft,X
    STA.w $1140,Y

    INY
    INY

    LDA.l !Map16TopRight,X
    STA.w $1100,Y

    LDA.l !Map16BottomRight,X
    STA.w $1140,Y

    INY
    INY

    DEC.b $06
    BNE return_CreateMap16StripesVerticalLoop

    TYA
    CLC
    ADC.w #$0040
    STA.b $0E

    RTS
assert pc() <= $82F2AC

; Use bank $7F for short tilemap-buffer stores. This also moves the temporary
; row buffer at $0500 from mirrored low WRAM to $7F0500.
org $82FA8B
    LDA.b #$7F
assert pc() <= $82FA8D

org $82FA9C
    LDA.b #$7F
assert pc() <= $82FA9E

; Expand 16 Map16 IDs from the temporary row buffer into two rows of 8x8
; tilemap words. X indexes the banked definitions and Y indexes the output
; (swapped compared to the vanilla routine), to allow us to do long loads
; from the 4 banks holding the new Map16 data.
org $82FB5B
CopyOneMap16Segment_Banked:
    STA.b $02

    LDX.b $0A

    LDA.b $00
    ORA.b $CC
    STA.l $7F4000,X

    INX
    INX
    STX.b $0A

    LDY.b $0E

    LDA.w #$0010
    STA.b $0C

.next
    LDX.b $02

    LDA.w $0500,X

    INX
    INX
    STX.b $02

    ASL A
    TAX

    LDA.l !Map16TopLeft,X
    STA.w $2000,Y

    LDA.l !Map16BottomLeft,X
    STA.w $2040,Y

    INY
    INY

    LDA.l !Map16TopRight,X
    STA.w $2000,Y

    LDA.l !Map16BottomRight,X
    STA.w $2040,Y

    INY
    INY

    DEC.b $0C
    BNE .next

    TYA
    CLC
    ADC.w #$0040
    STA.b $0E

    RTS
assert pc() <= $82FBAB

; Special-overworld entry and return triggers inspect the top-left 8x8 word.
org $84E88E
    LDA.l !Map16TopLeft,X
assert pc() <= $84E892

; Return the Map16 ID * 2 in X for the banked definition tables.
org $84E909
    ASL A
    TAX
    RTS
assert pc() <= $84E90C

org $84E928
    LDA.l !Map16TopLeft,X
assert pc() <= $84E92C

; UseOverworldEntrance checks fixed quadrants for animated door graphics.
; Keep the vanilla instruction addresses so its existing branches are unchanged.
; Replace two ASL instructions with NOPs, since the offset into each bank is
; now Map16 ID * 2 instead of Map16 ID * 8.
org $9BBC22
    NOP
    NOP
assert pc() <= $9BBC24

org $9BBC2C
    LDA.l !Map16TopRight,X
assert pc() <= $9BBC30

org $9BBC48
    NOP
    NOP
assert pc() <= $9BBC4A

org $9BBC4B
    LDA.l !Map16TopLeft,X
assert pc() <= $9BBC4F

; DrawMap16Anywhere expands one Map16 tile into its four 8x8 words.
org $9BC984
    ASL A
    NOP
    NOP
assert pc() <= $9BC987

org $9BC9B2
    LDA.l !Map16TopLeft,X
assert pc() <= $9BC9B6

org $9BC9B9
    LDA.l !Map16TopRight,X
assert pc() <= $9BC9BD

org $9BC9C0
    LDA.l !Map16BottomLeft,X
assert pc() <= $9BC9C4

org $9BC9C7
    LDA.l !Map16BottomRight,X
assert pc() <= $9BC9CB

; AlterMap16Hardcore performs the same expansion while changing the WRAM map.
org $9BC9E4
    ASL A
    NOP
    NOP
assert pc() <= $9BC9E7

org $9BCA3F
    LDA.l !Map16TopLeft,X
assert pc() <= $9BCA43

org $9BCA46
    LDA.l !Map16TopRight,X
assert pc() <= $9BCA4A

org $9BCA4D
    LDA.l !Map16BottomLeft,X
assert pc() <= $9BCA51

org $9BCA54
    LDA.l !Map16BottomRight,X
assert pc() <= $9BCA58
