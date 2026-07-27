; Expand Map16 definitions, by using a separate bank for each of the four 8x8 quadrants
; (giving 4x larger capacity), and migrate terrain, entrance, and tilemap code
; to read from these banks.
;
; This patch was implemented before independent_tile_type.asm, and several parts are
; replaced by it; we keep those parts as commented-out sections here in case someone ever
; wants to apply this patch without the independent_tile_type.asm.

lorom

!Map16TopLeft = $288000
!Map16TopRight = $298000
!Map16BottomLeft = $2A8000
!Map16BottomRight = $2B8000

; When using this patch without independent_tile_type.asm, enable this hook so
; GetOverworldTileType can still derive properties from the banked graphics.
; The combined build leaves it disabled because independent_tile_type.asm
; replaces the same part of the routine.
; org $00884E
;     JML GetOverworldTileType_Banked
; assert pc() <= $008852

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

;; This uses space freed up by the Map32 system being ripped out, so it assumes
;; flat_map16.asm is also applied. If not, this should be relocated elsewhere.
;; Though if independent_tile_type.asm is used, then this isn't used at all,
;; so it is commented out.
;;
;; Inputs:
;;   A.w: Map16 ID read from the WRAM map
;;   $00.w: world Y coordinate in pixels (bit 3 selects top or bottom)
;;   $02.w: world X coordinate divided by 8 (bit 0 selects left or right)
;;   16-bit accumulator and index registers
;;
;; Loads the selected 8x8 tile word into A, then resumes the vanilla routine.
;org $02F39C
;GetOverworldTileType_Banked:
;    ASL A
;    TAX
;
;    LDA.b $02
;    LSR A                   ; carry: left (clear) or right (set)
;    LDA.b $00
;    BIT.w #$0008
;    BNE .bottom
;
;    BCC .top_left
;    LDA.l !Map16TopRight,X
;    BRA .loaded
;
;.top_left
;    LDA.l !Map16TopLeft,X
;    BRA .loaded
;
;.bottom
;    BCC .bottom_left
;    LDA.l !Map16BottomRight,X
;    BRA .loaded
;
;.bottom_left
;    LDA.l !Map16BottomLeft,X
;
;.loaded
;    JML $008868             ; Resume the vanilla terrain and slope handling.


; Inputs:
;   X.w: vanilla interleaved definition offset:
;        Map16 ID * 8 + quadrant * 2
;   16-bit accumulator and index registers
;
; Returns the selected 8x8 tile word in A. X becomes Map16 ID * 2.
org $02F3C6
LoadBankedMap16Definition:
    ; Save the interleaved offset. Dividing it by four gives Map16 ID * 2,
    ; plus one for a bottom quadrant; clearing bit 0 removes that extra one.
    TXA
    PHA
    LSR A
    LSR A
    AND.w #$FFFE
    TAX

    ; The original offset's bits 1-2 encode TL=0, TR=2, BL=4, BR=6.
    ; Two shifts leave top/bottom in A and left/right in carry.
    PLA
    AND.w #$0006
    LSR A
    LSR A
    BNE .bottom

    BCC .top_left
    LDA.l !Map16TopRight,X
    RTL

.top_left
    LDA.l !Map16TopLeft,X
    RTL

.bottom
    BCC .bottom_left
    LDA.l !Map16BottomRight,X
    RTL

.bottom_left
    LDA.l !Map16BottomLeft,X
    RTL
assert pc() <= $02F3EE

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

; If using this patch without independent_tile_type.asm, uncomment these blocks,
; so terrain actions, hammer sounds, and liftables can still derive properties
; from the banked graphics. The combined build replaces all three paths.
;
;; Terrain actions have already formed the vanilla interleaved definition offset.
; org $1BBEF5
;     JSL LoadBankedMap16Definition
; assert pc() <= $1BBEF9
;
;; Hammer sound selection inspects the top-left 8x8 word.
; org $1BBF1E
;     ASL A
;     NOP
;     NOP
; assert pc() <= $1BBF21
;
; org $1BBF22
;     LDA.l !Map16TopLeft,X
; assert pc() <= $1BBF26
;
;; Liftable objects use the same vanilla interleaved definition offset.
; org $1BC040
;     JSL LoadBankedMap16Definition
; assert pc() <= $1BC044

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
