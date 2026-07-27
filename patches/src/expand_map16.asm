; Expand Map16 definitions, by using a separate bank for each of the four 8x8 quadrants
; (giving 4x larger capacity), and migrate terrain, entrance, and tilemap code
; to read from these banks.

lorom

!Map16TopLeft = $288000
!Map16TopRight = $298000
!Map16BottomLeft = $2A8000
!Map16BottomRight = $2B8000

; There's not enough space to patch GetOverworldTileType in place,
; so we hook the middle of it.
org $00884E
    JML GetOverworldTileType_Banked
assert pc() <= $008852

; Vanilla has loaded the next horizontal-stripe Map16 ID into A.
org $02F1AD
CreateMap16Stripes_Horizontal_Banked_Next:

org $02F1B6
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
    BNE CreateMap16Stripes_Horizontal_Banked_Next

    TYA
    CLC
    ADC.w #$0042
    STA.b $0E

    RTS
assert pc() <= $02F1E4

; Vanilla has loaded the next vertical-stripe Map16 ID into A.
org $02F275
CreateMap16Stripes_Vertical_Banked_Next:

org $02F27E
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
    BNE CreateMap16Stripes_Vertical_Banked_Next

    TYA
    CLC
    ADC.w #$0040
    STA.b $0E

    RTS
assert pc() <= $02F2AC

; Inputs:
;   A.w: Map16 ID read from the WRAM map
;   $00.w: world Y coordinate in pixels (bit 3 selects top or bottom)
;   $02.w: world X coordinate divided by 8 (bit 0 selects left or right)
;   16-bit accumulator and index registers
;
; Loads the selected 8x8 tile word into A, then resumes the vanilla routine.
org $02F39C
GetOverworldTileType_Banked:
    ASL A
    TAX

    LDA.b $02
    LSR A                   ; carry: left (clear) or right (set)
    LDA.b $00
    BIT.w #$0008
    BNE .bottom

    BCC .top_left
    LDA.l !Map16TopRight,X
    BRA .loaded

.top_left
    LDA.l !Map16TopLeft,X
    BRA .loaded

.bottom
    BCC .bottom_left
    LDA.l !Map16BottomRight,X
    BRA .loaded

.bottom_left
    LDA.l !Map16BottomLeft,X

.loaded
    JML $008868             ; Resume the vanilla terrain and slope handling.

; Use bank $7F for short tilemap-buffer stores. This also moves the temporary
; row buffer at $0500 from mirrored low WRAM to $7F0500.
org $02FA8B
    LDA.b #$7F
assert pc() <= $02FA8D

org $02FA9C
    LDA.b #$7F
assert pc() <= $02FA9E

; Expand 16 Map16 IDs from the temporary row buffer into two rows of 8x8
; tilemap words. X indexes the banked definitions and Y indexes the output
; (swapped compared to the vanilla routine), to allow us to do long loads
; from the 4 banks holding the new Map16 data.
org $02FB5B
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
assert pc() <= $02FBAB

; Special-overworld entry and return triggers inspect the top-left 8x8 word.
org $04E88E
    LDA.l !Map16TopLeft,X
assert pc() <= $04E892

; Return the Map16 ID * 2 in X for the banked definition tables.
org $04E909
    ASL A
    TAX
    RTS
assert pc() <= $04E90C

org $04E928
    LDA.l !Map16TopLeft,X
assert pc() <= $04E92C

; UseOverworldEntrance checks fixed quadrants for entrance and door graphics.
; Keep the vanilla instruction addresses so its existing branches are unchanged.
; Replace two ASL instructions with NOPs, since the offset into each bank is
; now Map16 ID * 2 instead of Map16 ID * 8.
org $1BBC22
    NOP
    NOP
assert pc() <= $1BBC24

org $1BBC2C
    LDA.l !Map16TopRight,X
assert pc() <= $1BBC30

org $1BBC48
    NOP
    NOP
assert pc() <= $1BBC4A

org $1BBC4B
    LDA.l !Map16TopLeft,X
assert pc() <= $1BBC4F

org $1BBCC1
    LDA.l !Map16BottomLeft,X
assert pc() <= $1BBCC5

org $1BBCCA
    LDA.l !Map16BottomRight,X
assert pc() <= $1BBCCE

; Hammer sound selection inspects the top-left 8x8 word.
org $1BBF1E
    ASL A
    NOP
    NOP
assert pc() <= $1BBF21

org $1BBF22
    LDA.l !Map16TopLeft,X
assert pc() <= $1BBF26

; DrawMap16Anywhere expands one Map16 tile into its four 8x8 words.
org $1BC984
    ASL A
    NOP
    NOP
assert pc() <= $1BC987

org $1BC9B2
    LDA.l !Map16TopLeft,X
assert pc() <= $1BC9B6

org $1BC9B9
    LDA.l !Map16TopRight,X
assert pc() <= $1BC9BD

org $1BC9C0
    LDA.l !Map16BottomLeft,X
assert pc() <= $1BC9C4

org $1BC9C7
    LDA.l !Map16BottomRight,X
assert pc() <= $1BC9CB

; AlterMap16Hardcore performs the same expansion while changing the WRAM map.
org $1BC9E4
    ASL A
    NOP
    NOP
assert pc() <= $1BC9E7

org $1BCA3F
    LDA.l !Map16TopLeft,X
assert pc() <= $1BCA43

org $1BCA46
    LDA.l !Map16TopRight,X
assert pc() <= $1BCA4A

org $1BCA4D
    LDA.l !Map16BottomLeft,X
assert pc() <= $1BCA51

org $1BCA54
    LDA.l !Map16BottomRight,X
assert pc() <= $1BCA58
