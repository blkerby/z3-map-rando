; Randomized rewards use vanilla receipt behavior and assigned pickup graphics.
; Chest bytes are patched directly. Other carriers retain their own identity
; and collection flag while reading a separate assigned reward.
;
; Receipt hooks enter with 8-bit A/X/Y. Helpers preserve X unless the displaced
; vanilla receipt already clobbers it; no direct-page scratch is introduced.
; $7EF3D4-$7EF3D8 are unused bytes in the vanilla saved-game block.

lorom
incsrc "symbols.inc"

!free_space_bank_a0_start = $A0C000
!free_space_bank_a0_end = $A0F000
!NpcFlags = $7EF3D4
!EventFlags = $7EF3D5
!SmithSword = $7EF3D7
!HalfMagicReceipt = $7EF3D8

; Source itemAddress slots reserved for the non-chest reward tables.
!CaveRewards = $B08000
!MiniMoldormReward = $B08010
!HypeCaveReward = $B08011
!BookReward = $B08012
!MushroomReward = $B08013
!PowderReward = $B08014
!BatReward = $B08015
!EtherReward = $B08016
!BombosReward = $B08017
!SmithReward = $B0802A
!DungeonPrizeTypes = $B08050
!OverworldRewards = $B08140
!GroveReward = $B0814A
!BossRewards = $B08150

; Replacing a long inventory load with a long flag read leaves X/Y and carry
; unchanged. Each reader returns the value expected by its vanilla comparison.
macro hook_flag(address, name)
org <address>
hook_check_<name>:
    JSL Read<name>Flag
endmacro

%hook_flag($868D86, SickKid)
%hook_flag($85F177, Sahasrahla)
%hook_flag($9EE90A, OldMan)
%hook_flag($868F16, KingZora)
%hook_flag($9DE102, Catfish)
org $9DE11C
hook_check_catfish_gift:
    JSL ReadCatfishFlag
%hook_flag($868D0E, Book)
%hook_flag($85EE53, Mushroom)
%hook_flag($85F53E, Powder)
%hook_flag($85FAFF, Bat)
%hook_flag($85F274, Bombos)
%hook_flag($85F285, Ether)
%hook_flag($86B489, Smith)

org $86B062
hook_check_stumpy_reward:
    JSL ReadStumpyPhase

; These vanilla LDY instructions remain intact so their immediate reward
; bytes can still be written using the source catalog's itemAddress.
org $86B9D4
hook_receive_sick_kid_reward:
    JSL GrantSickKidReward
org $85F200
hook_receive_sahasrahla_reward:
    JSL GrantSahasrahlaReward
org $9EE9FE
hook_receive_old_man_reward:
    JSL GrantOldManReward
org $86B0C9
hook_receive_stumpy_reward:
    JSL GrantStumpyReward
org $9DDF71
hook_receive_water_gift:
    JSL GrantWaterGift

macro hook_prepare(address, name)
org <address>
hook_prepare_<name>:
    JSL Prepare<name>Reward
    NOP
endmacro
%hook_prepare($85FD3F, Book)
%hook_prepare($85EE8C, Mushroom)
%hook_prepare($85F65D, Powder)
%hook_prepare($86B55B, Smith)

org $86B505
hook_take_smith_sword:
    JSL SaveSmithSword
    BRA +
+

; The receipt displays the assigned reward message; do not unconditionally
; announce HalfMagic when the Bat gives something else.
org $85FBBE
hook_skip_magic_bat_message:
    JML return_restore_magic_bat_palette
org $85FBC6
return_restore_magic_bat_palette:

org $85FBD1
hook_receive_magic_bat_reward:
    JSL GrantBatReward
    BRA +
+

org $9EF063
hook_receive_generous_gift:
    JML PrepareGenerousReward
    NOP

org $85EFCE
hook_receive_boss_reward:
    JSL GrantHeartReward
org $85EFEE
hook_receive_heart_container_reward:
    JSL GrantHeartReward
org $85F08A
hook_receive_heart_piece_reward:
    JML ReceiveHeartPieceReward

org $87A379
hook_spawn_grove_reward:
    JSL SpawnGroveReward
org $88D01B
hook_receive_grove_reward:
    JSL GrantGroveReward

org $88CB6A
hook_receive_milestone_reward:
    JSL GrantMilestoneReward
org $81C715
hook_check_dungeon_prize:
    JSL ReadDungeonPrizeType
org $9ECEC8
hook_choose_crystal_speech:
    JML ShowCrystalSpeech

; Two default pool IDs extend the vanilla receipt table: $4E is HalfMagic,
; $50 is a regular Master Sword without the pedestal-only cutscene.
org $87B57D
hook_check_chest_reward_replacement:
    JML ReadChestRewardReplacement
    NOP
org $87B582
return_check_chest_reward_replacement:
org $87B59F
return_receive_chest_reward:

org $87999D
hook_grant_item:
    JML GrantAssignedItem
    NOP

org $8987B4
    CPY.b #$FF
org $898870
    CMP.b #$FF
org $88C3DF
    CMP.b #$FF
org $88C434
    CMP.b #$FF
org $898497
    db $05 ; Use normal sword OAM properties instead of pedestal-specific ones.

; HalfMagic uses the small magic receipt artwork and its vanilla Bat message,
; but must not perform the small magic refill at the end of that animation.
org $88C4E6
hook_finish_magic_receipt:
    JML FinishMagicReceipt

org !free_space_bank_a0_start

macro read_flag(name, flags, mask, value)
Read<name>Flag:
    LDA.l <flags>
    AND.b #<mask>
    BEQ .uncollected
    LDA.b #<value>
.uncollected
    RTL
endmacro
%read_flag(SickKid, !NpcFlags, $01, $01)
%read_flag(Sahasrahla, !NpcFlags, $02, $01)
%read_flag(OldMan, !NpcFlags, $04, $02)
%read_flag(KingZora, !NpcFlags, $08, $01)
%read_flag(Catfish, !NpcFlags, $10, $01)
%read_flag(Book, !NpcFlags, $80, $01)
%read_flag(Mushroom, !NpcFlags, $40, $02)
%read_flag(Powder, !EventFlags, $02, $02)
%read_flag(Bat, !EventFlags, $01, $01)
%read_flag(Bombos, !EventFlags, $40, $01)
%read_flag(Ether, !EventFlags, $20, $01)
%read_flag(Smith, !EventFlags, $04, $03)

ReadStumpyPhase:
    LDA.l !NpcFlags
    AND.b #$20
    BEQ .uncollected
    ; Once collected, retain the vanilla Flute dialogue/animation phases.
    LDA.l $7EF34C
    AND.b #$03
    BNE .uncollected
    LDA.b #$01
.uncollected
    RTL

macro grant_flag(name, flags, mask)
Grant<name>Reward:
    LDA.l <flags>
    ORA.b #<mask>
    STA.l <flags>
    JML $87999D
endmacro
%grant_flag(SickKid, !NpcFlags, $01)
%grant_flag(Sahasrahla, !NpcFlags, $02)
%grant_flag(OldMan, !NpcFlags, $04)
%grant_flag(Stumpy, !NpcFlags, $20)

GrantWaterGift:
    ; $0EE0 is the vanilla gift's Zora/Catfish discriminator. Keep $0D90
    ; unchanged: zero there dispatches to the parent Catfish rather than a gift.
    LDA.w $0EE0,X
    BEQ .catfish
    LDA.l !NpcFlags
    ORA.b #$08
    STA.l !NpcFlags
    LDA.l !KingZoraReward
    TAY
    JML $87999D
.catfish
    LDA.l !NpcFlags
    ORA.b #$10
    STA.l !NpcFlags
    LDA.l !CatfishReward
    TAY
    JML $87999D

macro prepare_reward(name, flags, mask, reward)
Prepare<name>Reward:
    LDA.l <flags>
    ORA.b #<mask>
    STA.l <flags>
    LDA.l <reward>
    TAY
    STZ.w $02E9 ; Run hi-jacked instruction
    RTL
endmacro
%prepare_reward(Book, !NpcFlags, $80, !BookReward)
%prepare_reward(Mushroom, !NpcFlags, $40, !MushroomReward)
%prepare_reward(Powder, !EventFlags, $02, !PowderReward)

SaveSmithSword:
    LDA.l $7EF359
    STA.l !SmithSword
    ; Run hi-jacked instructions:
    LDA.b #$FF
    STA.l $7EF359
    RTL

PrepareSmithReward:
    ; Restore the sword taken for forging before granting the reward. A
    ; non-sword reward must not leave the player in the swordless forging state.
    LDA.l !SmithSword
    STA.l $7EF359
    LDA.l !EventFlags
    ORA.b #$04
    STA.l !EventFlags
    LDA.l !SmithReward
    TAY
    STZ.w $02E9 ; Run hi-jacked instruction
    RTL

GrantBatReward:
    LDA.l !EventFlags
    ORA.b #$01
    STA.l !EventFlags
    PHX
    LDA.l !BatReward
    TAY
    STZ.w $02E9
    JSL $87999D
    PLX
    RTL

PrepareGenerousReward:
    ; The caller has a bank-$1E JSR return on the stack. Tail-jump back to
    ; its original receipt routine; that routine's RTS consumes the return.
    PHP
    REP #$20
    LDA.w $048E
    CMP.w #$0123
    SEP #$20
    BEQ .mini_moldorm
    LDA.l !HypeCaveReward
    BRA .prepare
.mini_moldorm
    LDA.l !MiniMoldormReward
.prepare
    TAY
    PLP
    JML $9EF35A

GrantHeartReward:
    JSL ReadHeartReward
    TAY
    STZ.w $02E9
    JML $87999D

ReceiveHeartPieceReward:
    ; The sprite main routine was entered with JSR, not JSL. Resume in its
    ; original bank after granting so the vanilla flag setters return via RTS.
    PHX
    JSL ReadHeartReward
    TAY
    STZ.w $02E9
    JSL $87999D
    PLX
    JML $85F0C0

ReadHeartReward:
    PHX
    ; Boss drops are marked independently of the room's freestanding pickups.
    LDA.w $0D90,X
    BEQ +
    JMP .boss
+   LDA.b $1B
    BNE +
    JMP .outdoors
+
    PHP
    REP #$20
    LDA.w $048E
    CMP.w #$00E1
    BEQ .cave_0
    CMP.w #$00E2
    BEQ .cave_1
    CMP.w #$00EA
    BEQ .cave_2
    CMP.w #$011B
    BEQ .two_caves
    CMP.w #$0126
    BEQ .cave_5
    CMP.w #$0127
    BEQ .cave_6
    PLP
    LDA.b #$26
    BRA .finish
.cave_0
    PLP
    LDA.l !CaveRewards
    BRA .finish
.cave_1
    PLP
    LDA.l !CaveRewards+1
    BRA .finish
.cave_2
    PLP
    LDA.l !CaveRewards+2
    BRA .finish
.two_caves
    PLP
    LDA.w $0D30,X
    AND.b #$01
    BEQ .graveyard
    LDA.l !CaveRewards+3
    BRA .finish
.graveyard
    LDA.l !CaveRewards+4
    BRA .finish
.cave_5
    PLP
    LDA.l !CaveRewards+5
    BRA .finish
.cave_6
    PLP
    LDA.l !CaveRewards+6
    BRA .finish
.boss
    LDA.w $040C
    LSR A
    TAX
    LDA.l ChooseBossReward,X
    TAX
    LDA.l !BossRewards,X
    BRA .finish
.outdoors
    LDA.b $8A
    LDX.b #$09
.next_screen
    CMP.l HeartRewardScreens,X
    BEQ .read_screen
    DEX
    BPL .next_screen
    LDA.b #$17
    BRA .finish
.read_screen
    LDA.l !OverworldRewards,X
.finish
    PLX
    RTL

HeartRewardScreens:
    db $03,$05,$28,$30,$35,$3B,$4A,$5B,$68,$81
ChooseBossReward:
    db $00,$00,$00,$01,$00,$04,$03,$08,$05,$07,$02,$06,$09

SpawnGroveReward:
    LDA.l $7EF2AA
    AND.b #$40
    BNE .collected
    ; Run hi-jacked instructions:
    LDA.b #$36
    LDY.b #$00
    JML $898C5F
.collected
    RTL

GrantGroveReward:
    LDA.l $7EF2AA
    ORA.b #$40
    STA.l $7EF2AA
    LDA.l !GroveReward
    TAY
    JML $87999D

GrantMilestoneReward:
    LDA.b $1B
    BNE .dungeon_prize
    CPY.b #$10
    BEQ .ether
    CPY.b #$0F
    BNE .dungeon_prize
    LDA.l !EventFlags
    ORA.b #$40
    STA.l !EventFlags
    LDA.l !BombosReward
    BRA .tablet
.ether
    LDA.l !EventFlags
    ORA.b #$20
    STA.l !EventFlags
    LDA.l !EtherReward
.tablet
    TAY
    STZ.w $02E9
.dungeon_prize
    JML $87999D

ReadDungeonPrizeType:
    LDA.l !DungeonPrizeTypes,X
    RTL

ShowCrystalSpeech:
    ; Vanilla indexes maiden messages by the dungeon, which is invalid when
    ; a crystal moves to Eastern/Desert. Select the message by its crystal bit.
    PHX
    LDA.w $040C
    LSR A
    TAX
    LDA.l $82A09B,X
    LDX.b #$00
.find_crystal
    LSR A
    BCS .found_crystal
    INX
    BRA .find_crystal
.found_crystal
    LDA.l CrystalMessageIds,X
    CMP.b #$36
    BNE .select_message
    LDA.l $7EF37A
    AND.b #$7F
    CMP.b #$7F
    BEQ .last_crystal
    LDA.b #$3A
    BRA .select_message
.last_crystal
    LDA.b #$36
.select_message
    PHA
    LDA.l $7EF3C7
    CMP.b #$07
    BCS .show
    LDA.b #$07
    STA.l $7EF3C7
.show
    PLA
    LDY.b #$01
    PLX
    JSL $85E219
    JML $9ECEFE

; Vanilla crystal bits are not ordered by their maiden message numbers.
CrystalMessageIds:
    db $35,$30,$34,$36,$31,$33,$32

GrantAssignedItem:
    CPY.b #$50
    BNE .not_master_sword
    LDY.b #$01
.not_master_sword
    CPY.b #$4E
    BNE GrantNativeItem
    LDA.b #$01
    STA.l $7EF37B
    STA.l !HalfMagicReceipt
    LDY.b #$45
    JSL GrantNativeItem
    LDA.b #$0F
    LDY.b #$01
    JSL $85E219
    CLC
    RTL

GrantNativeItem:
    ; Run hi-jacked instructions. PHK at the relocated address would select
    ; bank $A0, so load the original table bank explicitly instead.
    PHB
    LDA.b #$87
    PHA
    PLB
    LDA.b $4D
    JML $8799A2

FinishMagicReceipt:
    CMP.b #$45 ; Run hi-jacked instruction
    BNE .not_magic
    LDA.l !HalfMagicReceipt
    BEQ .refill
    LDA.b #$00
    STA.l !HalfMagicReceipt
    LDA.b #$45
    JML $88C4F7
.refill
    LDA.b #$45
    JML $88C4EA
.not_magic
    JML $88C4F7

incsrc "item_graphics.inc"

ReadChestRewardReplacement:
    ; Entry is A8/X8/Y8 with DB=$87. Preserve X/Y and use only the original
    ; $03 scratch write. Custom IDs have no entries in the vanilla tables.
    CPY.b #$50
    BEQ .custom_reward
    CPY.b #$4E
    BEQ .custom_reward

    ; Run hi-jacked instructions:
    LDA.w $B511,Y
    STA.b $03

    JML return_check_chest_reward_replacement
.custom_reward
    JML return_receive_chest_reward

assert pc() <= !free_space_bank_a0_end
