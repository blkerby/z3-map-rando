; Resolve ordinary entrances, pits, and special-overworld transitions from
; generated area-local lists, independently of their Map16 graphics.
;
; The Rust code populates the entrance lists in each area's OverworldAreaAssets.
; These are optional 24-bit pointers located in the area record selected
; based on the current area ID in $8A:
;
;   +18: ordinary entrances
;        count, then [16-bit Map16 offset, entrance ID, follower flag]
;   +21: pit entrances
;        count, then [16-bit Map16 offset, entrance ID]
;   +29: special-overworld transitions
;        count, then [16-bit Map16 offset, 16-bit special-area ID, direction]
;        A zero special-area ID identifies a return to the saved overworld.
;
; A zero pointer means the area has no entries of that kind. The runtime looks
; up the current Map16 buffer offset in the applicable list.

lorom

incsrc "symbols.inc"

!free_space_bank_a0_start = $A0A0C1
!free_space_bank_a0_end = $A0A400

!AreaEntrancePointerOffset = 18
!AreaPitEntrancePointerOffset = 21
!AreaSpecialTransitionPointerOffset = 29

; GetPitEntranceDestination has already calculated the Map16 buffer offset in
; $00. Replace its global screen-and-coordinate scan with the current area's
; generated pit list.
org $9BB88E
hook_resolve_pit_entrance:
    JML FindAreaPitEntrance
assert pc() == $9BB892

; Generated animated-door matching has already run. Skip vanilla's checks for
; fixed graphics characters and resolve ordinary doors and entrances from the
; current area's generated data.
org $9BBC25
hook_resolve_overworld_entrance:
    JML FindAreaEntrance
assert pc() == $9BBC29

; Both vanilla special-transition checks have calculated the contacted Map16
; buffer offset in Y. Replace their graphics-character comparisons with the
; current area's generated coordinate list.
org $84E88E
hook_resolve_special_overworld_entry:
    JML FindAreaSpecialEntry
assert pc() == $84E892

org $84E928
hook_resolve_special_overworld_return:
    JML FindAreaSpecialReturn
assert pc() == $84E92C

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
    INY
    INY
    LDA.b [$03],Y
    STA.b $00
    INY
    LDA.b [$03],Y
    STA.b $02

    LDA.b $2F
    BNE .not_wooden_door

    REP #$20
    LDX.b $08
    JSL !OpenDynamicWoodenDoorAtEntrance
    BCC .not_generated_wooden_door
    SEP #$30
    RTL

.not_generated_wooden_door
    SEP #$20

.not_wooden_door
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
    LDA.b $02
    AND.b #$01
    BEQ .forbidden_8_bit

.allowed
    SEP #$20
    LDA.b $00
    JML $9BBD63               ; Continue vanilla entrance setup with this ID.

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

; Find a special transition at the Map16 buffer offset in Y. On success, Y
; points to its 16-bit special-area ID in the five-byte record.
FindCurrentSpecialTransition:
    STY.b $08

    SEP #$20
    REP #$10
    LDY.w #!AreaSpecialTransitionPointerOffset
    JSR LoadCurrentAreaList
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
    INY
    DEC.b $06
    BNE .next

.not_found
    CLC
    RTS

.found
    INY
    INY
    SEC
    RTS

; Store the forced movement direction in A and its bit index in the two
; transition-direction caches.
SetSpecialTransitionDirection:
    STA.b $67
    LDX.w #$0004

.shift
    DEX
    LSR A
    BCC .shift

    TXA
    STA.w $0418
    STA.w $069C
    RTS

FindAreaSpecialEntry:
    JSR FindCurrentSpecialTransition
    BCC .exit

    REP #$20
    LDA.b [$03],Y
    BEQ .exit
    STA.b $A0

    INY
    INY
    SEP #$20
    LDA.b [$03],Y
    STA.w $0410
    STA.w $0416
    JSR SetSpecialTransitionDirection

    LDA.b #$17
    STA.b $11
    LDA.b #$0B
    STA.b $10

.exit
    SEP #$30
    RTL

FindAreaSpecialReturn:
    JSR FindCurrentSpecialTransition
    BCC .exit

    REP #$20
    LDA.b [$03],Y
    BNE .exit

    INY
    INY
    SEP #$20
    LDA.b [$03],Y
    JSR SetSpecialTransitionDirection

    LDA.b #$24
    STA.b $11
    STZ.b $B0
    STZ.b $A0

.exit
    SEP #$30
    RTL

assert pc() <= !free_space_bank_a0_end
