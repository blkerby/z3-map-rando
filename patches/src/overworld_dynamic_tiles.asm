; Resolve overworld terrain replacements from generated, graphics-independent
; Map16 tables. Rust writes the fixed group directory at $A78300 and interns
; the group bodies and descriptors in the generated-data region.
;
;   group:
;       +$00  byte    entry count
;       +$01          entries
;
;   entry:
;       +$00  word    contacted before Map16 ID
;       +$02  byte    signed X displacement to the footprint origin
;       +$03  byte    signed Y displacement to the footprint origin
;       +$04  3 bytes little-endian descriptor pointer
;
;   descriptor:
;       +$00  byte    width in Map16 cells
;       +$01  byte    height in Map16 cells
;       +$02  byte    frame count
;       +$03  words   before Map16 IDs, row-major
;       +...  words   after-frame Map16 IDs, frame-major and row-major
;
; ResolveDynamicTile takes the group index in 16-bit A and the contacted
; $7E2000 byte offset in 16-bit X. It preserves $00/$02 and clobbers $04-$09
; and Y. Carry set returns the first cell of the first after frame in A and the
; footprint origin in X. Carry clear restores the contacted offset in X.

lorom

incsrc "symbols.inc"

!free_space_bank_a0_start = $A0AF00
!free_space_bank_a0_end = $A10000

!DynamicCutGrass = 0
!DynamicDigTerrain = 1
!DynamicGreenBush = 2
!DynamicHeavyBush = 3
!DynamicHammerPeg = 4
!DynamicLiftSign = 5
!DynamicSmallGrayRock = 6
!DynamicSmallBlackRock = 7
!DynamicLargeGrayRock = 8
!DynamicLargeBlackRock = 9
!DynamicRockPile = 10
!DynamicSecretHole = 11
!DynamicSecretPortal = 12
!DynamicSecretBombableEntrance = 13
!DynamicSecretStairs = 14
!DynamicWoodenDoor = 15
!DynamicSanctuaryDoor = 16
!DynamicHyruleCastleDoor = 17
!DynamicGraveCorpse = 18
!DynamicGraveStairs = 19
!DynamicGravePit = 20
!SecretObjectTypes = $9BC89C

!DynamicOriginalOffset = $7ECC50
!DynamicOrigin = $7ECC52
!DynamicEntryCount = $7ECC54
!DynamicWidth = $7ECC56
!DynamicHeight = $7ECC58
!DynamicColumnCount = $7ECC5A
!DynamicRowOffset = $7ECC5C
!DynamicResult = $7ECC5E
!DynamicInteractionOffset = $7ECC60
!DynamicDescriptorOffset = $7ECC62
!DynamicResolved = $7ECC64
; ponytail: NPC door animation opens one door at a time; match generated after
; footprints directly if simultaneous door-closing animations are introduced.
!DynamicWoodenDoorDescriptor = $7ECC66
!DynamicWoodenDoorOrigin = $7ECC69
!DynamicMap32Descriptor = $7ECC6B
!DynamicMap32Origin = $7ECC6E
!DynamicMap32AfterOffset = $7ECC70

; HandleItemTileAction_Overworld has calculated the contacted Map16 offset in
; X. Replace its graphics-ID classifier with property routing and group lookup.
org $9BBDA4
hook_resolve_item_tile_action:
    JML DispatchDynamicItemTileAction
assert pc() == $9BBDA8

; Use the resolved result already stored in $0E instead of vanilla's fixed
; dig-terrain replacement.
org $9BBE58
    LDY.b $0E
    NOP
assert pc() == $9BBE5B

; Use the resolved result instead of vanilla's fixed cut-grass replacement.
org $9BBE88
    LDY.b $0E
    NOP
assert pc() == $9BBE8B

; Bush setup has already selected its effect and coordinates. Skip the source
; Map16 comparison that formerly selected one of two fixed replacements.
org $9BBEAF
    LDY.b $0E
    BRA return_reveal_dynamic_tile_secret
assert pc() == $9BBEB3

org $9BBEBA
return_reveal_dynamic_tile_secret:

; HandleOverworldLiftables has saved the rounded coordinates and loaded no
; tile yet. Route liftable properties through generated data.
org $9BBFA8
hook_resolve_liftable:
    JML DispatchDynamicLiftable
assert pc() == $9BBFAC

; SmashRockPileFromHere has located the contacted Map16 cell. Replace its
; fixed rock-pile IDs and constituent offsets with a generated footprint.
org $9BC081
hook_resolve_rock_pile:
    JML DispatchDynamicRockPile
assert pc() == $9BC085

; The large-interaction path has finished secret handling. Draw the generated
; ordinary footprint or resolve the generated stairs footprint.
org $9BC0DD
hook_draw_large_interaction:
    JML FinishDynamicLargeInteraction
assert pc() == $9BC0E1

; BombOverworldTiles has filtered the follower case and retained the contacted
; offset in $04. Replace the grass and bush graphics-ID comparisons.
org $9BC172
hook_resolve_bomb_tile:
    JML DispatchDynamicBombTile
assert pc() == $9BC176

; BombOverworldTiles has confirmed a bombable-entrance secret. Replace its
; fixed two-cell drawing with the generated footprint.
org $9BC1E4
hook_draw_bombable_entrance:
    JML DrawDynamicBombableEntrance
assert pc() == $9BC1E8

; RevealOverworldSecret has selected a tile-reveal type. Replace its vanilla
; Map16 result with a generated single-cell secret when one matches.
org $9BC92D
hook_resolve_revealed_secret:
    JSL ResolveDynamicSecret
assert pc() == $9BC931

; DrawWoodenDoor receives the generated footprint origin in X and carry set
; when an NPC-operated door is closing.
org $9BC952
hook_draw_wooden_door:
    JML DrawDynamicWoodenDoor
    NOP
assert pc() == $9BC957

; UseOverworldEntrance has calculated the contacted Map16 offset in X and Y.
; Recognize generated animated doors before vanilla inspects their graphics.
org $9BBC1D
hook_resolve_animated_door_entrance:
    JML OpenDynamicAnimatedDoorAtEntrance
assert pc() == $9BBC21

; DoMap32Update has established bank $02 and is about to draw one of its
; fixed 2x2 frames. Select generated door and grave frames for their states.
org $82AC89
hook_draw_map32_update:
    JML DrawDynamicMap32Update
    NOP
    NOP
assert pc() == $82AC8F

; DrawOverworldQuadrantsAndOverlays has selected the Link's house doorway
; location in X. Replace its fixed open-door Map16 IDs during map loading.
org $82EC5B
hook_draw_exit_wooden_door:
    JML DrawDynamicExitWoodenDoor
assert pc() == $82EC5F

org $82EC8D
return_draw_exit_wooden_door:

; OverworldOverlay_DrawRevealedStairs receives the generated footprint
; coordinate in X. Replace its fixed revealed-stairs Map16 IDs.
org $87FC44
hook_draw_revealed_stairs_overlay:
    JML DrawDynamicRevealedStairsOverlay
assert pc() == $87FC48

org $87FC56
return_draw_revealed_stairs_overlay:

; ApplyOverworldOverlay has loaded the pristine Graveyard map and is about to
; write the fixed King's Tomb Map16 IDs. Replace them with the generated frame.
org $87FC57
hook_draw_kings_tomb_overlay:
    JML DrawDynamicKingsTombOverlay
assert pc() == $87FC5B

org $87FC73
return_draw_kings_tomb_overlay:

org !free_space_bank_a0_start

OpenDynamicWoodenDoorAtEntrance:
    LDA.w #!DynamicWoodenDoor
    JSR ResolveDynamicTile
    BCC .miss
    JSR DrawDynamicFootprint

    SEP #$20
    LDA.b #$15
    STA.w $012F
    REP #$30
    SEC
    RTL

.miss
    CLC
    RTL

OpenDynamicAnimatedDoorAtEntrance:
    TXA
    PHA
    SEC
    SBC.w #$0080
    TAX
    LDA.w #!DynamicSanctuaryDoor
    JSR ResolveDynamicTile
    BCS .sanctuary
    LDA.w #!DynamicHyruleCastleDoor
    JSR ResolveDynamicTile
    BCC .miss
    LDA.w #$0018
    BRA .open

.sanctuary
    LDA.w #$0000

.open
    STA.w $0692
    PLA
    JSR CacheDynamicMap32Descriptor
    STX.w $0698

    SEP #$20
    LDA.b #$15
    STA.w $012F
    STZ.b $B0
    STZ.w $0690
    LDA.b #$0C
    STA.b $11
    SEP #$30
    RTL

.miss
    PLA
    TAX
    TAY
    LDA.l $7E2000,X           ; Run hi-jacked instruction
    JML $9BBC21

DrawDynamicWoodenDoor:
    BCS .closed
    REP #$30
    SEP #$20
    LDA.b #$00
    STA.l !DynamicWoodenDoorDescriptor+2
    REP #$20
    LDA.w #!DynamicWoodenDoor
    JSR ResolveDynamicTile
    BCC .vanilla

    LDA.b $07
    STA.l !DynamicWoodenDoorDescriptor
    SEP #$20
    LDA.b $09
    STA.l !DynamicWoodenDoorDescriptor+2
    REP #$20
    TXA
    STA.l !DynamicWoodenDoorOrigin

    JSR DrawDynamicFootprint
    JML $9BC975

.vanilla
    LDA.w #$0D9E
    JML $9BC957

.closed
    REP #$30
    TXA
    CMP.l !DynamicWoodenDoorOrigin
    BNE .vanilla_closed
    SEP #$20
    LDA.l !DynamicWoodenDoorDescriptor+2
    BEQ .vanilla_closed_8_bit
    STA.b $09
    LDA.b #$00
    STA.l !DynamicWoodenDoorDescriptor+2
    REP #$20
    LDA.l !DynamicWoodenDoorDescriptor
    STA.b $07
    LDA.w #$0003
    STA.l !DynamicDescriptorOffset
    TXA
    STA.l !DynamicOrigin
    JSR DrawDynamicFootprint
    JML $9BC975

.vanilla_closed_8_bit
    REP #$20
.vanilla_closed
    JML $9BC960

DrawDynamicMap32Update:
    LDX.w $0698
    LDA.w $0692
    CMP.w #$0018
    BCC .sanctuary_door
    CMP.w #$0028
    BCC .hyrule_castle_door
    CMP.w #$0030
    BEQ .grave_corpse_bottom
    CMP.w #$0038
    BEQ .grave_stairs_bottom
    CMP.w #$0040
    BNE +
    JMP .grave_top
    +
    CMP.w #$0048
    BNE +
    JMP .grave_top
    +
    CMP.w #$0058
    BEQ .grave_pit_bottom
    CMP.w #$0060
    BNE .not_generated
    JMP .grave_top

.not_generated
    JMP .vanilla

.sanctuary_door
    PHA
    CMP.w #$0000
    BNE .draw_door_frame

.resolve_sanctuary_door
    LDA.w #!DynamicSanctuaryDoor
    BRA .resolve_door_frame

.hyrule_castle_door
    SEC
    SBC.w #$0018
    PHA
    BNE .draw_door_frame
    LDA.w #!DynamicHyruleCastleDoor

.resolve_door_frame
    JSR ResolveDynamicTile
    BCC .frame_miss
    JSR CacheDynamicMap32Descriptor

.draw_door_frame
    SEP #$20
    LDA.l !DynamicMap32Descriptor+2
    BEQ .frame_miss_8_bit
    REP #$30
    TXA
    CMP.l !DynamicMap32Origin
    BNE .frame_miss
    LDA.l !DynamicMap32Descriptor
    STA.b $07
    SEP #$20
    LDA.l !DynamicMap32Descriptor+2
    STA.b $09
    REP #$20
    LDA.l !DynamicMap32AfterOffset
    STA.l !DynamicDescriptorOffset
    PLA
    CLC
    ADC.l !DynamicDescriptorOffset
    STA.l !DynamicDescriptorOffset
    JMP .draw

.frame_miss_8_bit
    REP #$30
.frame_miss
    PLA
    JMP .generated_miss

.grave_corpse_bottom
    LDA.w #!DynamicGraveCorpse
    BRA .resolve_grave_bottom

.grave_stairs_bottom
    LDA.w #!DynamicGraveStairs
    BRA .resolve_grave_bottom

.grave_pit_bottom
    LDA.w #!DynamicGravePit

.resolve_grave_bottom
    PHA
    TXA
    SEC
    SBC.w #$0080
    TAX
    PLA
    JSR ResolveDynamicTile
    BCS +
    JMP .generated_miss
    +

    JSR CacheDynamicMap32Descriptor
    JSR LoadDynamicFootprintDimensions

    LDA.l !DynamicWidth
    ASL A
    CLC
    ADC.l !DynamicDescriptorOffset
    STA.l !DynamicDescriptorOffset
    LDA.l !DynamicOrigin
    CLC
    ADC.w #$0080
    STA.l !DynamicOrigin
    LDA.l !DynamicHeight
    DEC A
    STA.l !DynamicHeight
    JSR DrawDynamicFootprintConfigured
    JML $82AD44

.grave_top
    SEP #$20
    LDA.l !DynamicMap32Descriptor+2
    BEQ .generated_miss_8_bit
    REP #$30
    TXA
    CMP.l !DynamicMap32Origin
    BNE .generated_miss
    LDA.l !DynamicMap32Descriptor
    STA.b $07
    SEP #$20
    LDA.l !DynamicMap32Descriptor+2
    STA.b $09
    LDA.b #$00
    STA.l !DynamicMap32Descriptor+2
    REP #$20
    TXA
    STA.l !DynamicOrigin
    LDA.l !DynamicMap32AfterOffset
    STA.l !DynamicDescriptorOffset
    JSR LoadDynamicFootprintDimensions
    LDA.l !DynamicHeight
    DEC A
    STA.l !DynamicHeight
    JSR DrawDynamicFootprintConfigured
    JML $82AD44

.generated_miss_8_bit
    REP #$30
.generated_miss
    SEP #$20
    LDA.b #$00
    STA.l !DynamicMap32Descriptor+2
    REP #$30
    JML $82AD44

.draw
    JSR DrawDynamicFootprint
    JML $82AD44

.vanilla
    ; Run hi-jacked instructions:
    LDA.w $0698
    LDX.w $04AC

    JML $82AC8F

DrawDynamicKingsTombOverlay:
    LDX.w #$0532
    LDA.w #!DynamicGraveStairs
    JSR ResolveDynamicTile
    BCC .done

    LDA.l !DynamicDescriptorOffset
    TAY
    LDA.b [$07],Y
    STA.l $7E2000,X
    INY
    INY
    LDA.b [$07],Y
    STA.l $7E2002,X
    INY
    INY
    LDA.b [$07],Y
    STA.l $7E2080,X
    INY
    INY
    LDA.b [$07],Y
    STA.l $7E2082,X
    INY
    INY
    LDA.b [$07],Y
    STA.l $7E2100,X
    INY
    INY
    LDA.b [$07],Y
    STA.l $7E2102,X

.done
    JML return_draw_kings_tomb_overlay

DrawDynamicRevealedStairsOverlay:
    LDA.w #!DynamicSecretStairs
    JSR ResolveDynamicTile
    BCC .done

    LDA.l !DynamicDescriptorOffset
    TAY
    LDA.b [$07],Y
    STA.l $7E2000,X
    INY
    INY
    LDA.b [$07],Y
    STA.l $7E2002,X
    INY
    INY
    LDA.b [$07],Y
    STA.l $7E2080,X
    INY
    INY
    LDA.b [$07],Y
    STA.l $7E2082,X

.done
    JML return_draw_revealed_stairs_overlay

DrawDynamicExitWoodenDoor:
    LDA.w #!DynamicWoodenDoor
    JSR ResolveDynamicTile
    BCC .done

    LDA.l !DynamicDescriptorOffset
    TAY
    LDA.b [$07],Y
    STA.l $7E2000,X
    JSL $84E780
    LDA.l !DynamicDescriptorOffset
    CLC
    ADC.w #$0002
    TAY
    LDA.b [$07],Y
    STA.l $7E2002,X
    INX
    INX
    JSL $84E780
    DEX
    DEX

.done
    STZ.w $0696
    JML return_draw_exit_wooden_door

DispatchDynamicItemTileAction:
    LDA.l $7E2000,X
    PHA
    PHX
    JSL !GetOverworldTileTypeIndependent
    REP #$30
    AND.w #$00FF
    STA.b $0E
    PLX

    LDA.w $0301
    AND.w #$0002
    BNE .hammer

    LDA.w $0301
    AND.w #$0040
    BNE .powder

    LDA.b $0E
    CMP.w #$0040
    BEQ .grass
    CMP.w #$0048
    BEQ .dig
    CMP.w #$0050
    BEQ .green_bush
    CMP.w #$0051
    BEQ .heavy_bush
    JML $9BBDEC

.powder
    LDA.b $0E
    CMP.w #$0050
    BEQ .green_bush
    CMP.w #$0051
    BEQ .heavy_bush
    JML $9BBDEC

.hammer
    LDA.b $0E
    CMP.w #$0027
    BNE .hammer_miss
    LDA.w #!DynamicHammerPeg
    JSR ResolveDynamicTile
    BCC .hammer_miss
    STA.b $0E

    SEP #$20
    LDA.b #$11
    STA.w $012E
    REP #$20

    JSL $84E7A7
    LDA.b $0E
    JML $9BBEC3

.hammer_miss
    PLA
    PHA
    JML $9BBDE9

.grass
    LDA.w #!DynamicCutGrass
    JSR ResolveDynamicTile
    BCC .miss
    STA.b $0E
    JML $9BBE5D

.dig
    LDA.w #!DynamicDigTerrain
    JSR ResolveDynamicTile
    BCC .miss
    STA.b $0E
    JML $9BBE3E

.green_bush
    LDA.w #!DynamicGreenBush
    JSR ResolveDynamicTile
    BCC .miss
    STA.b $0E
    LDY.w #$0002
    JML $9BBE8D

.heavy_bush
    LDA.w #!DynamicHeavyBush
    JSR ResolveDynamicTile
    BCC .miss
    STA.b $0E
    LDY.w #$0004
    JML $9BBE8D

.miss
    JML $9BBDEC

DispatchDynamicLiftable:
    LDA.w #$0000
    STA.l !DynamicResolved
    LDA.l $7E2000,X
    PHX
    TAX
    SEP #$20
    LDA.l !Map16PropertyTopLeft,X
    REP #$30
    AND.w #$00FF
    PLX

    CMP.w #$0050
    BEQ .green_bush
    CMP.w #$0051
    BEQ .heavy_bush
    CMP.w #$0052
    BEQ .gray_rock
    CMP.w #$0053
    BEQ .black_rock
    CMP.w #$0054
    BEQ .sign
    CMP.w #$0055
    BEQ .large_gray_rock
    CMP.w #$0056
    BEQ .large_black_rock

    LDA.l $7E2000,X
    JML $9BBFAC

.green_bush
    LDA.w #!DynamicGreenBush
    BRA .resolve

.heavy_bush
    LDA.w #!DynamicHeavyBush
    BRA .resolve

.gray_rock
    LDA.w #!DynamicSmallGrayRock
    BRA .resolve

.black_rock
    LDA.w #!DynamicSmallBlackRock
    BRA .resolve

.sign
    LDA.w #!DynamicLiftSign

.resolve
    JSR ResolveDynamicTile
    BCC .miss
    STA.b $0E
    LDA.l $7E2000,X
    LDY.b $0E
    JML $9BC008

.miss
    LDA.l $7E2000,X
    JML $9BC027

.large_gray_rock
    LDA.w #!DynamicLargeGrayRock
    BRA .resolve_large

.large_black_rock
    LDA.w #!DynamicLargeBlackRock

.resolve_large
    TAY
    LDA.l $7E2000,X
    PHA
    TYA
    JSR ResolveDynamicTile
    BCC .large_miss
    LDA.l !DynamicOriginalOffset
    STA.l !DynamicInteractionOffset
    LDA.w #$0001
    STA.l !DynamicResolved
    STX.w $0698
    JML $9BC0B0

.large_miss
    PLA
    JML $9BBFAC

DispatchDynamicBombTile:
    PHX
    LDA.l $7E2000,X
    TAX
    SEP #$20

    LDA.l !Map16PropertyTopLeft,X
    CMP.b #$40
    BEQ .grass
    CMP.b #$50
    BEQ .green_bush
    CMP.b #$51
    BEQ .heavy_bush
    LDA.l !Map16PropertyTopRight,X
    CMP.b #$40
    BEQ .grass
    CMP.b #$50
    BEQ .green_bush
    CMP.b #$51
    BEQ .heavy_bush
    LDA.l !Map16PropertyBottomLeft,X
    CMP.b #$40
    BEQ .grass
    CMP.b #$50
    BEQ .green_bush
    CMP.b #$51
    BEQ .heavy_bush
    LDA.l !Map16PropertyBottomRight,X
    CMP.b #$40
    BEQ .grass
    CMP.b #$50
    BEQ .green_bush
    CMP.b #$51
    BEQ .heavy_bush

    REP #$30
    PLX
    JML $9BC1D6

.grass
    REP #$30
    PLX
    LDA.w #!DynamicCutGrass
    LDY.w #$0003
    BRA .resolve

.green_bush
    REP #$30
    PLX
    LDA.w #!DynamicGreenBush
    LDY.w #$0002
    BRA .resolve

.heavy_bush
    REP #$30
    PLX
    LDA.w #!DynamicHeavyBush
    LDY.w #$0004

.resolve
    PHY
    LDA.b $08
    PHA
    JSR ResolveDynamicTile
    BCC .miss
    STA.b $0E
    PLA
    STA.b $08
    PLY
    STX.b $04
    TYX
    LDY.b $0E
    JML $9BC197

.miss
    PLA
    STA.b $08
    PLY
    STX.b $04
    JML $9BC1D6

DispatchDynamicRockPile:
    LDA.w #$0000
    STA.l !DynamicResolved
    LDA.l $7E2000,X
    PHA
    LDA.w #!DynamicRockPile
    JSR ResolveDynamicTile
    BCC .miss
    LDA.l !DynamicOriginalOffset
    STA.l !DynamicInteractionOffset
    LDA.w #$0001
    STA.l !DynamicResolved
    STX.w $0698
    JML $9BC0B0

.miss
    PLA
    JML $9BC05D

FinishDynamicLargeInteraction:
    LDA.l !DynamicResolved
    BEQ .vanilla
    LDA.w #$0000
    STA.l !DynamicResolved

    LDA.b $0E
    CMP.w #$FFFF
    BNE .draw
    LDA.l !DynamicInteractionOffset
    TAX
    LDA.w #!DynamicSecretStairs
    JSR ResolveDynamicTile
    BCC .draw_vanilla

.draw
    JSR DrawDynamicFootprint
    STZ.w $0692
    JML $9BC024

.draw_vanilla
    LDX.w $0698
    JSL $82AC5B
    JML $9BC024

.vanilla
    ; Run hi-jacked instructions:
    LDX.b $0C
    LDA.b $00

    JML $9BC0E1

DrawDynamicBombableEntrance:
    LDA.w #!DynamicSecretBombableEntrance
    JSR ResolveDynamicTile
    BCC .vanilla
    JSR DrawDynamicFootprint
    JML $9BC205

.vanilla
    LDA.w #$0DAE
    STA.l $7E2000,X            ; Run hi-jacked instruction
    JML $9BC1E8

ResolveDynamicSecret:
    LDA.l !SecretObjectTypes,X  ; Run hi-jacked instruction
    CMP.w #$0DC6
    BEQ .hole
    CMP.w #$0212
    BEQ .portal
    RTL

.hole
    LDY.w #!DynamicSecretHole
    BRA .resolve

.portal
    LDY.w #!DynamicSecretPortal

.resolve
    STA.b $0E
    LDX.b $04
    TYA
    JSR ResolveDynamicTile
    STX.b $04
    BCS .done
    LDA.b $0E

.done
    RTL

DrawDynamicFootprint:
    JSR LoadDynamicFootprintDimensions

DrawDynamicFootprintConfigured:
    LDA.l !DynamicOrigin
    STA.l !DynamicRowOffset

.next_row
    LDA.l !DynamicWidth
    STA.l !DynamicColumnCount
    LDA.l !DynamicRowOffset
    TAX

.next_cell
    LDA.l !DynamicDescriptorOffset
    TAY
    LDA.b [$07],Y
    STA.l !DynamicResult
    INY
    INY
    TYA
    STA.l !DynamicDescriptorOffset
    LDA.l !DynamicResult
    STA.l $7E2000,X
    JSL $84E780
    LDA.l !DynamicResult
    JSL $9BC980
    INX
    INX
    LDA.l !DynamicColumnCount
    DEC A
    STA.l !DynamicColumnCount
    BNE .next_cell

    LDA.l !DynamicRowOffset
    CLC
    ADC.w #$0080
    STA.l !DynamicRowOffset
    LDA.l !DynamicHeight
    DEC A
    STA.l !DynamicHeight
    BNE .next_row

    SEP #$20
    LDA.b #$01
    STA.b $14
    REP #$30
    LDA.l !DynamicOrigin
    TAX
    RTS

LoadDynamicFootprintDimensions:
    LDY.w #$0000
    SEP #$20
    LDA.b [$07],Y
    REP #$20
    AND.w #$00FF
    STA.l !DynamicWidth
    INY
    SEP #$20
    LDA.b [$07],Y
    REP #$20
    AND.w #$00FF
    STA.l !DynamicHeight
    RTS

CacheDynamicMap32Descriptor:
    LDA.b $07
    STA.l !DynamicMap32Descriptor
    SEP #$20
    LDA.b $09
    STA.l !DynamicMap32Descriptor+2
    REP #$30
    LDA.l !DynamicOrigin
    STA.l !DynamicMap32Origin
    LDA.l !DynamicDescriptorOffset
    STA.l !DynamicMap32AfterOffset
    RTS

ResolveDynamicTile:
    PHA
    TXA
    STA.l !DynamicOriginalOffset
    PLA
    STA.b $04
    ASL A
    CLC
    ADC.b $04
    TAX

    SEP #$20
    LDA.l !DynamicTileGroupPointers,X
    STA.b $04
    LDA.l !DynamicTileGroupPointers+1,X
    STA.b $05
    LDA.l !DynamicTileGroupPointers+2,X
    STA.b $06
    BNE .has_group

    REP #$30
    LDA.l !DynamicOriginalOffset
    TAX
    CLC
    RTS

.has_group
    REP #$30
    LDA.l !DynamicOriginalOffset
    TAX
    SEP #$20

    LDY.w #$0000
    LDA.b [$04],Y
    REP #$20
    AND.w #$00FF
    STA.l !DynamicEntryCount
    INY

.next_entry
    LDA.b [$04],Y
    CMP.l $7E2000,X
    BEQ .candidate

    TYA
    CLC
    ADC.w #$0007
    TAY
    LDA.l !DynamicEntryCount
    DEC A
    STA.l !DynamicEntryCount
    BNE .next_entry
    JMP .not_found

.candidate
    PHY
    LDA.l !DynamicEntryCount
    PHA

    INY
    INY
    SEP #$20
    LDA.b [$04],Y
    REP #$20
    AND.w #$00FF
    CMP.w #$0080
    BCC .positive_x
    ORA.w #$FF00

.positive_x
    ASL A
    CLC
    ADC.l !DynamicOriginalOffset
    STA.l !DynamicOrigin

    INY
    SEP #$20
    LDA.b [$04],Y
    REP #$20
    AND.w #$00FF
    CMP.w #$0080
    BCC .positive_y
    ORA.w #$FF00

.positive_y
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    CLC
    ADC.l !DynamicOrigin
    STA.l !DynamicOrigin

    INY
    SEP #$20
    LDA.b [$04],Y
    STA.b $07
    INY
    LDA.b [$04],Y
    STA.b $08
    INY
    LDA.b [$04],Y
    STA.b $09

    LDY.w #$0000
    LDA.b [$07],Y
    REP #$20
    AND.w #$00FF
    STA.l !DynamicWidth
    INY
    SEP #$20
    LDA.b [$07],Y
    REP #$20
    AND.w #$00FF
    STA.l !DynamicHeight
    LDA.l !DynamicOrigin
    STA.l !DynamicRowOffset
    LDY.w #$0003

.next_row
    LDA.l !DynamicWidth
    STA.l !DynamicColumnCount
    LDA.l !DynamicRowOffset
    TAX

.next_cell
    LDA.b [$07],Y
    CMP.l $7E2000,X
    BNE .candidate_failed
    INY
    INY
    INX
    INX
    LDA.l !DynamicColumnCount
    DEC A
    STA.l !DynamicColumnCount
    BNE .next_cell

    LDA.l !DynamicRowOffset
    CLC
    ADC.w #$0080
    STA.l !DynamicRowOffset
    LDA.l !DynamicHeight
    DEC A
    STA.l !DynamicHeight
    BNE .next_row

    TYA
    STA.l !DynamicDescriptorOffset
    LDA.b [$07],Y
    STA.l !DynamicResult
    PLA
    PLY
    LDA.l !DynamicOrigin
    TAX
    LDA.l !DynamicResult
    SEC
    RTS

.candidate_failed
    PLA
    STA.l !DynamicEntryCount
    PLY
    TYA
    CLC
    ADC.w #$0007
    TAY
    LDA.l !DynamicOriginalOffset
    TAX
    LDA.l !DynamicEntryCount
    DEC A
    STA.l !DynamicEntryCount
    BEQ .not_found
    JMP .next_entry

.not_found
    LDA.l !DynamicOriginalOffset
    TAX
    CLC
    RTS

assert pc() <= !free_space_bank_a0_end
