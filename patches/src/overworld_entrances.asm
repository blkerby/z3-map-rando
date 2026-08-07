; Resolve ordinary overworld entrances and pits from generated area-local
; lists, independently of the Map16 graphics used to draw them.
;
; The Rust code populates the entrance lists in each area's OverworldAreaAssets.
; These are optional 24-bit pointers located in the area record selected
; based on the current area ID in $8A:
;
;   +18: ordinary entrances
;        count, then [16-bit Map16 offset, entrance ID, follower flag]
;   +21: pit entrances
;        count, then [16-bit Map16 offset, entrance ID]
;
; A zero pointer means the area has no entries of that kind. The runtime looks
; up the current Map16 buffer offset in the applicable list.

lorom

incsrc "symbols.inc"

!free_space_bank_a0_start = $A0A0C1
!free_space_bank_a0_end = $A0A200

!AreaEntrancePointerOffset = 18
!AreaPitEntrancePointerOffset = 21

; GetPitEntranceDestination has already calculated the Map16 buffer offset in
; $00. Replace its global screen-and-coordinate scan with the current area's
; generated pit list.
org $9BB88E
hook_resolve_pit_entrance:
    JML FindAreaPitEntrance
assert pc() == $9BB892

; Preserve the preceding graphics-based door-opening animations. Once those
; checks decline to animate a door, identify ordinary entrances solely by the
; current area's generated coordinate list.
org $9BBCC1
hook_resolve_overworld_entrance:
    JML FindAreaEntrance
assert pc() == $9BBCC5

org !free_space_bank_a0_start

; Load the current area's list pointer at area-record offset Y into $03-$05.
LoadCurrentAreaList:
    PHY
    LDA.b $8A
    JSR.w !ResolveOverworldAreaRecord
    PLY
    LDA.b [$00],Y
    STA.b $03
    INY
    LDA.b [$00],Y
    STA.b $04
    INY
    LDA.b [$00],Y
    STA.b $05
    RTS

; Find an ordinary entrance at the Map16 buffer offset in Y. Each list is a
; count followed by four-byte records: offset, entrance ID, follower flag.
FindAreaEntrance:
    PHY
    TYX
    JSL !OpenDynamicWoodenDoorAtEntrance
    BCC +
    JMP .opened_door
    +

    SEP #$20
    REP #$10
    LDY.w #!AreaEntrancePointerOffset
    JSR LoadCurrentAreaList

    PLY
    REP #$20
    TYA
    STA.b $08
    SEP #$20

    LDA.b $05
    BEQ .not_found

    LDY.w #$0000
    LDA.b [$03],Y
    STA.b $06
    INY

.next
    REP #$20
    LDA.b [$03],Y
    CMP.b $08
    SEP #$20
    BEQ .found

    INY
    INY
    INY
    INY
    DEC.b $06
    BNE .next

.not_found
    REP #$20
    STZ.w $04B8
    SEP #$30
    RTL

.found
    SEP #$20

    INY
    INY
    LDA.b [$03],Y
    STA.b $07
    INY
    LDA.b [$03],Y
    STA.b $06

    REP #$20
    LDA.l $7EF3D3
    AND.w #$00FF
    BNE .allowed

    LDA.w $02DA
    AND.w #$00FF
    CMP.w #$0001
    BEQ .forbidden

    LDA.l $7EF3CC
    AND.w #$00FF
    BEQ .allowed
    CMP.w #$0005              ; Zelda telepathy
    BEQ .allowed
    CMP.w #$000E              ; Telepathy
    BEQ .allowed
    CMP.w #$0001              ; Zelda
    BEQ .allowed
    CMP.w #$0007              ; Frog
    BEQ .check_frog_dwarf
    CMP.w #$0008              ; Dwarf
    BNE .forbidden

.check_frog_dwarf
    SEP #$20
    LDA.b $06
    AND.b #$01
    BEQ .forbidden_8_bit

.allowed
    SEP #$20
    LDA.b $07
    JML $9BBD63               ; Continue vanilla entrance setup with this ID.

.opened_door
    PLY
    SEP #$30
    RTL

.forbidden_8_bit
    REP #$20
.forbidden
    JML $9BBCF0               ; Use vanilla follower-denial handling.

; Find the destination for the pit Map16 buffer offset in $00. Pit lists are
; a count followed by three-byte records: offset and entrance ID.
FindAreaPitEntrance:
    LDA.b $00
    STA.b $08

    SEP #$20
    REP #$10
    LDY.w #!AreaPitEntrancePointerOffset
    JSR LoadCurrentAreaList
    BEQ .default

    LDY.w #$0000
    LDA.b [$03],Y
    STA.b $06
    INY

.next
    REP #$20
    LDA.b [$03],Y
    CMP.b $08
    SEP #$20
    BEQ .found

    INY
    INY
    INY
    DEC.b $06
    BNE .next

.default
    LDA.b #$00
    STA.l $7EF3CA             ; Unmatched pits lead to Houlihan in Light World.
    LDA.b #$82
    BRA .store

.found
    INY
    INY
    LDA.b [$03],Y

.store
    STA.w $010E
    STZ.w $010F
    SEP #$30
    PLB
    RTL

assert pc() <= !free_space_bank_a0_end
