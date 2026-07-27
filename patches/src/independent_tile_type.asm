; Make GetOverworldTileType read the selected Map16 quadrant's independent
; property byte instead of deriving it from the quadrant's graphics word.

lorom

!Map16PropertyTopLeft = $2C8000
!Map16PropertyTopRight = $2CC000
!Map16PropertyBottomLeft = $2D8000
!Map16PropertyBottomRight = $2DC000

; Vanilla has already loaded the Map16 ID into A.
;
; Inputs:
;   A.w: Map16 ID
;   $00.w: world Y coordinate in pixels (bit 3 selects top or bottom)
;   $02.w: world X coordinate divided by 8 (bit 0 selects left or right)
;   16-bit accumulator and index registers
;
; Returns the selected property in A with 8-bit accumulator and index registers.
org $00884E
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
assert pc() <= $008888

; ReadOverworldTileType has already located the Map16 ID and leaves it in A
; with the same $00/$02 coordinate API. Tail-call the shared quadrant lookup
; so its RTL returns directly to the original caller.
org $05FAC2
    JML GetOverworldTileType_Independent
assert pc() <= $05FAC6
