; Expand Map16 definitions into one bank per 8x8 quadrant (giving 4x larger capacity),
; and migrate GetOverworldTileType to read from those banks.

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

; A 16-bit Map16 ID has just been read from the WRAM map.
; Select the 8x8 quadrant containing the queried world coordinate.
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
