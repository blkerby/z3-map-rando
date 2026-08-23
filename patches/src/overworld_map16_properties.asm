; Give each Map16 quadrant an independent property byte. GetOverworldTileType
; then reads the selected quadrant's property instead of deriving it from the
; quadrant's graphics word.

lorom

incsrc "symbols.inc"

; Vanilla has already loaded the Map16 ID into A.
;
; Inputs:
;   A.w: Map16 ID
;   $00.w: world Y coordinate in pixels (bit 3 selects top or bottom)
;   $02.w: world X coordinate divided by 8 (bit 0 selects left or right)
;   16-bit accumulator and index registers
;
; Returns the selected property in A with 8-bit accumulator and index registers.
org $80884E
GetOverworldTileType_Independent:
    TAX

    LDA.b $02
    LSR A                   ; carry: left (clear) or right (set)
    LDA.b $00
    BIT.w #$0008
    SEP #$20
    BNE .bottom

    BCC .top_left
    LDA.l !Map16PropertyTopRight,X
    BRA .loaded

.top_left
    LDA.l !Map16PropertyTopLeft,X
    BRA .loaded

.bottom
    BCC .bottom_left
    LDA.l !Map16PropertyBottomRight,X
    BRA .loaded

.bottom_left
    LDA.l !Map16PropertyBottomLeft,X

.loaded
    SEP #$10
    RTL
assert pc() <= $808888

; ReadOverworldTileType has already located the Map16 ID and leaves it in A
; with the same $00/$02 coordinate API. Tail-call the shared quadrant lookup
; so its RTL returns directly to the original caller.
org $85FAC2
    JML GetOverworldTileType_Independent
assert pc() <= $85FAC6

; Terrain actions enter with the Map16 ID in A and the standard $00/$02
; coordinate API. Preserve the 16-bit property push expected by the rest of
; the vanilla routine.
org $9BBEDF
    JSL GetOverworldTileType_Independent
    REP #$30
    AND.w #$00FF
    PHA
    JMP.w $BF02
assert pc() <= $9BBEEC

; PickHammerSFX always inspects the top-left quadrant.
org $9BBF1E
    TAX
    SEP #$20
    LDA.l !Map16PropertyTopLeft,X
    JMP.w $BF2E
assert pc() <= $9BBF28

; GetLinkMap16Coords rounds liftable coordinates to a 16x16 boundary, so the
; old quadrant calculation always selected top-left. $06 holds the property
; while the vanilla coordinate values are restored from the stack.
org $9BC027
    TAX
    SEP #$20
    LDA.l !Map16PropertyTopLeft,X
    STA.b $06
    REP #$30
    PLA
    STA.b $00
    PLA
    STA.b $02
    SEP #$31
    LDA.b $06
    RTL
assert pc() <= $9BC03D

; Let ordinary sprites walk through shallow water while preserving its water behavior.
org $9DF6D8
    db $00
