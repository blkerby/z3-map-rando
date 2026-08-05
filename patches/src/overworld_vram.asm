; Set up a rearranged VRAM layout which allows for a larger overworld BG1/BG2
; character set (960 compared to the vanilla 512). All overworld scenes use
; this layout; dungeons keep their existing layout. The expanded character set
; is made possible by the reduction of BG1/2/3 tilemap (in reduce_bg3.asm and
; bg_streamer.asm). The active HUD destination doubles as the layout marker:
; $3C40 means overworld, while $6040 is the default gameplay destination.
;
; This requires installing a bunch of hooks to apply conditional logic
; for a VRAM destination depending on whether we are in overworld or not.
; This includes hooking the NMI handler, which is a bit undesirable but
; at this stage may be preferable to a wider overhaul. And the extra cost
; should be offset by the optimizations in nmi_optimize.asm to some extent.
;
; This patch also replaces the vanilla overworld graphics/palettes loading
; routines, switching them to use flexible loading descriptor lists which
; allow taking advantage of the expanded space.
;
lorom

!free_space_bank_any_start_1 = $A083A0
!free_space_bank_any_end_1 = $A08700
!free_space_bank_any_start_2 = $A09900
!free_space_bank_any_end_2 = $A09D80
!free_space_bank_any_start_3 = $A09F40
!free_space_bank_any_end_3 = $A0A400
!free_space_bank_82_start = $82F5C0
!free_space_bank_82_end = $82F5E7

!OverworldHUD = $3C40
!DefaultHUD = $6040
!OverworldAssetBundlePointers = $A78000
!OverworldAssetSchedule = $7EC906
!GeneratedAssetMarker = $7EC909
!Module15LayerEnable = $7EC90A
!AnimationTrackCount = $7EC90C
!AnimationSuspended = $7EC90D
; ponytail: Desert uses at most three tracks/rows; expand these ranges if a
; future theme exceeds 64 active tracks or 127 simultaneously updated rows.
!AnimationTrackState = $7EC910
!AnimationDescriptorList = $7ECA50
!CreditsFirstAssetKey = $A0
!SpriteSeedAssetKey = $FE
!CreditsCoolBackgroundAssetKey = $FF

; FileSelect_RebuildSave, module $05 while restoring a selected file: after
; seeding vanilla's four resident OBJ-sheet IDs, seed their matching graphics
; into VRAM through the generated full-load path.
org $828092
    JSL hook_FileLoadAfterSpriteCacheSeed
assert pc() == $828096

; Module08_00_LoadProperties, modules $08/$0A submodule $00: common
; forced-blank entry for normal and special overworld loads. Select and clear
; the relocated layout before any animated or static background graphics are
; loaded, then reproduce the displaced color-math setup.
org $8282D1
    JSL hook_Module08LoadProperties
assert pc() == $8282D5

; Credits_LoadOverworldScene_PrepGFX, module $1A while loading an overworld
; credits scene: replace its vanilla tilemap clear with the relocated-layout
; credits clear before it loads background graphics.
org $828506
    JSL hook_CreditsOverworldTilemapClear
assert pc() == $82850A

; Module19_02_LoadMusicAndScreen, module $19 submodule $02 while loading the
; Triforce room: replace its vanilla tilemap clear with the relocated-layout
; clear before it loads the special overworld screen, and discard any pending
; HUD upload so the scene's BG3 remains blank.
org $829F4A
    JSL hook_TriforceRoomBeforeSpecialOverworldLoad
assert pc() == $829F4E

; Credits_InitializePolyhedral briefly selects full brightness before
; Credits_InitializeTheActualCredits restores zero brightness later in the
; same frame. Keep the display black throughout that initialization so an NMI
; cannot expose partially loaded BG and OBJ graphics.
org $8CC999
    LDA.b #$00
assert pc() == $8CC99B

; Credits_InitializeTheActualCredits, module $1A submodule $20: replace its
; EraseTilemaps_bg3 call with the relocated-layout clear, retaining the
; distinct blank tiles used by the scrolling credits background.
org $8EE649
    JSL hook_CreditsOverworldTilemapClear
assert pc() == $8EE64D

; Credits_InitializeTheActualCredits initially selects vanilla's vertical BG2
; tilemap at $1000. Select its rebased static 32x64 map at $6000 instead.
org $8EE6F9
    LDA.b #$62
assert pc() == $8EE6FB

; Credits_FadeColorAndBeginAnimating later selects vanilla BG1/BG2 tilemaps
; together. Point both layers at the rebased shared static tilemap.
org $8EE783
    LDY.w #$6263
assert pc() == $8EE786

; Credits_AddEndingSequenceText, module $1A while preparing an overworld or
; underworld vignette: derive its full BG3 fill destination from the active
; layout instead of always targeting vanilla VRAM $6000.
org $8EECE7
    JSL hook_CreditsBeforeEndingTextBlankFill
    NOP
    NOP
assert pc() == $8EECED

; Credits_AddEndingSequenceText, while copying each text-stripe header: rebase
; its vanilla BG3 destination only when the overworld layout is active.
org $8EED0E
    JSL hook_CreditsBeforeEndingTextStripeCopy
assert pc() == $8EED12

; SetTargetOverworldWarpToPyramid, module $15 during Agahnim's mirror-warp
; transition: switch layouts when it loads the overworld destination without
; passing through module $08.
org $829D5C
hook_Module15OverworldLoad:
    JSR LoadModule15Overworld
assert pc() == $829D5F

; MirrorWarp_LoadSpritesAndColors normally changes the pyramid transition's
; target backdrop to black. Keep it white during Agahnim's Module $15 load.
org $82B329
hook_MirrorWarpSetPyramidTargetColor:
    JML SetMirrorWarpPyramidTargetColor
assert pc() == $82B32D

org $82B334
return_MirrorWarpSetPyramidTargetColor:

; WorldMap_RestoreGraphics, module $0E submodule $07 or $0A, internal phase
; $0200=$06: select and clear the overworld layout during the forced-blank
; restoration shared by the ordinary world map and flute map, then reload the
; current area's generated background assets.
org $8ABBF9
    JSL hook_WorldMapOverworldRestore
assert pc() == $8ABBFD

; InitializeTilesets, shared graphics-loading routine with no fixed module or
; submodule: generated overworld records own all four area-dependent OBJ slots,
; so return after the retained common-sprite load.
org $80E1F0
    JML hook_InitializeTilesetsBeforeAreaGraphics
    NOP
    NOP
assert pc() == $80E1F6

; InitializeTilesets, shared graphics-loading routine with no fixed module or
; submodule: normally begins its eight background sheets at $2000. Choose
; $0000 for the final overworld layout and retain $2000 elsewhere.
org $80E259
    JML hook_InitializeTilesetsBeforeBGSetup
    NOP
assert pc() == $80E25E

; Module08_00_LoadProperties, modules $08/$0A submodule $00: mark the
; forced-blank generated load before calling InitializeTilesets.
org $828390
    JSL hook_Module08InitializeTilesets
assert pc() == $828394

; Module08_00_LoadProperties, modules $08/$0A submodule $00: install the
; generated assets after vanilla resolves the area palette selectors and
; before it sets the background color and presents the palettes.
org $8283A7
    JSL hook_Module08AfterBGPaletteSelection
assert pc() == $8283AB

; OverworldLoadScreensPaletteSet, reached from modules $08/$0A submodule $00:
; keep its state, HUD, and sprite palette work but omit its final main-BG write
; while a generated full reload is active.
org $82C44A
    JSL hook_OverworldScreenPalettesAfterHUD
assert pc() == $82C44E

; OverworldPalettesLoader, shared by full overworld loads and module $09
; transition setup: keep palette selection and sprite writes but omit its three
; auxiliary-BG writes while a generated load marker is active.
org $8CFF4F
    JSL hook_OverworldPalettesAfterSetSelection
    BRA +
org $8CFF5B
+

; ReloadPreviouslyLoadedSheets, mirror/portal, Agahnim, and whirlpool cleanup:
; generated assets own both the static BG and area-dependent OBJ characters.
org $80D7CB
    JML hook_ReloadPreviouslyLoadedSheetsBeforeGraphics
assert pc() == $80D7CF

; AnimateMirrorWarp's NMI request table, mirror/portal and Agahnim transitions:
; generated batches replace the four vanilla BG-character uploads in phases 1-4.
org $80D896
    db $00, $00, $00, $00
assert pc() == $80D89A

; AnimateMirrorWarp_LoadPyramidIfAga: on the penultimate delay step, stage the
; relocated HUD after the old layout's final animated-tile write.
org $80D8DC
hook_Module15MirrorWarpBeforeLayoutSwitch:
    JML PrepareModule15HUDBeforeLayoutSwitch
assert pc() == $80D8E0

; AnimateMirrorWarp's NMI request table, phases $0C/$0D: generated full-load
; batches have already replaced the two old area-OBJ uploads.
org $80D8A1
    db $00, $00
assert pc() == $80D8A3

; AnimateMirrorWarp_DecompressNewTileSets, after it resolves the destination
; BG sheets: skip its obsolete OBJ-sheet cache resolution, then initialize and
; submit the first generated batch.
org $80D98B
    JML hook_MirrorBeforeSpriteSheetResolution
assert pc() == $80D98F

; AnimateMirrorWarp_DecompressBackgroundsA/B/C, phases 2-4: replace each
; obsolete BG decompression pass with the next generated full-reload batch.
org $80D9F9
    JML hook_MirrorNextGeneratedAssetBatch
assert pc() == $80D9FD

org $80DA38
    JML hook_MirrorNextGeneratedAssetBatch
assert pc() == $80DA3C

org $80DA6C
    JML hook_MirrorNextGeneratedAssetBatch
assert pc() == $80DA70

; AnimateMirrorWarp_DecompressSpritesA/B, phases $0C/$0D: generated batches
; already installed all area-dependent OBJ rows. Keep the follower cleanup
; which vanilla performs at the end of the second routine.
org $80DAFB
    JML hook_MirrorBeforeObsoleteSpriteGraphicsA
assert pc() == $80DAFF

org $80DB5B
    JML hook_MirrorBeforeObsoleteSpriteGraphicsB
assert pc() == $80DB5F

; Credits_LoadOverworldScene_PrepGFX, module $1A while preparing an overworld
; vignette: retain common OBJ setup, then keep the generated marker active through
; the following background palette selectors.
org $828558
    JSL hook_CreditsInitializeTilesets
assert pc() == $82855C

; Credits_LoadOverworldScene_PrepGFX, after the destination palette selectors
; are known: install the current area's generated assets before palette cache.
org $828568
    JSL hook_CreditsAfterPaletteSelection
assert pc() == $82856C

; Credits_LoadCoolBackground, module $1A scrolling-background setup: keep its
; scripted OBJ setup while generated assets replace the static BG load.
org $8285D1
    JSL hook_CreditsCoolBackgroundInitializeTilesets
assert pc() == $8285D5

; Credits_LoadCoolBackground, after selecting its nonstandard BG2 palette:
; replace the direct vanilla palette write with generated asset key $FF.
org $8285EA
    JSL hook_CreditsCoolBackgroundAfterPaletteSelection
assert pc() == $8285EE

; Module19_03_PrepTileSetsPalette, Triforce room forced-blank setup: preserve
; OBJ loading and mark its following BG palette selection as generated.
org $829F6B
    JSL hook_TriforceInitializeTilesets
assert pc() == $829F6F

; Module19_03_PrepTileSetsPalette, after its palette selectors are established:
; install the generated $8A=$88 Triforce bundle before the special cache copy.
org $829F76
    JSL hook_TriforceAfterPaletteSelection
assert pc() == $829F7A

; OverworldMosaicTransition_RecoverDestinationPalettes, modules $09/$0B:
; retain selector and OBJ palette work without replacing generated BG rows.
org $82B01B
    JSL hook_MosaicDestinationPaletteSelection
assert pc() == $82B01F

; Module09_25, used directly by special-overworld entry phase $18 and as exit
; phase $25: install the resolved destination assets while forced blank is
; still active, then retain the shared sprite-slot initialization.
org $82AE2D
    JSL hook_Module09BeforeSpecialSpriteReload
assert pc() == $82AE31

; AnimateMirrorWarp_DoSpritesPalettes, mirror/portal and Agahnim transitions:
; bracket the destination sprite/palette routine so its BG writes are skipped.
org $80D8FB
    JSL hook_MirrorWarpBeforeSpritesAndColors
assert pc() == $80D8FF

; Module09_2E_07/08, whirlpool phases $07/$08: replace the scrolling-only
; shared calls with a destination full-reload sequence, one batch per frame.
org $82A375
return_Module09AfterSubmodule:

org $82B3DF
    JML hook_WhirlpoolBeforeGeneratedAssets
    NOP
assert pc() == $82B3E4

org $82B3E4
    JML hook_WhirlpoolBeforeNextGeneratedAssetBatch
assert pc() == $82B3E8

; Module09_2E_09_LoadPalettes, whirlpool phase $09: mark the complete retained
; sprite/HUD/effect palette block as generated before its first instruction.
org $82B3F3
    JML hook_WhirlpoolBeforePaletteWork
assert pc() == $82B3F7

; Module09_2E_09_LoadPalettes, after all palette selectors and background
; state are established: suppress obsolete area-OBJ conversion and clear the marker.
org $82B422
    JSL hook_WhirlpoolAfterPaletteWork
assert pc() == $82B426

; Module09_2E_09_LoadPalettes, whirlpool phase $09: the direct main-BG call
; does not pass through the shared generated-load guard, so suppress it here.
org $82B406
    JSL hook_WhirlpoolMainBackgroundPalette
assert pc() == $82B40A

; LoadLandingScreenPalettes, flute landing after destination palette selection:
; install generated assets before background color and palette caching.
org $82EA54
    JSL hook_FluteAfterPaletteSelection
assert pc() == $82EA58

; FluteMenu_LoadSelectedScreen, module $0E flute restoration: keep the marker
; active while the landing-palette routine chooses destination palette state.
org $8AB911
    JSL hook_FluteLoadLandingScreenPalettes
assert pc() == $8AB915

; FluteMenu_LoadSelectedScreen, after animated and fixed-color setup: retain
; only InitializeTilesets' common OBJ load because generated assets are resident.
org $8AB937
    JSL hook_FluteInitializeTilesets
assert pc() == $8AB93B

; WorldMap_ExitMap, forced-blank world-map restoration: retain only
; InitializeTilesets' common OBJ load after the generated asset reload.
org $8ABC79
    JSL hook_WorldMapInitializeTilesets
assert pc() == $8ABC7D

; Intro_InitializeDefaultGFX, intro initialization after startup or
; save-and-quit: restore the default layout before the routine clears and
; rebuilds VRAM.
org $8CC208
    JSL hook_IntroDefaultGraphicsBeforeTilemapClear
assert pc() == $8CC20C

; LoadUnderworldEntrance, shared underworld-entry routine during forced blank:
; restore and clear the default layout before the room and HUD are uploaded.
org $82D61A
    JSL hook_LoadUnderworldEntranceBeforeEnvironmentFlag
assert pc() == $82D61E

; DecompressAnimatedOverworldTiles: generated animation tracks load their
; frames directly from ROM, so the old overworld WRAM conversion is obsolete.
org $80D3D4
    RTL
assert pc() == $80D3D5

; PrepareOAMForTransfer, after dynamic OBJ sources are selected: replace only
; the overworld animated-background timer with the generated track scheduler.
org $8086DF
hook_PrepareOverworldAnimations:
    JML ScheduleOverworldAnimations
assert pc() == $8086E3

; DoNMIUpdates, after ordinary dynamic OBJ uploads: omit the old 1 KiB
; animated-background cache upload only while the overworld layout is active.
org $808B50
hook_NMIUploadOverworldAnimations:
    JML UploadOverworldAnimations
    NOP
    NOP
assert pc() == $808B56

; NMI_UploadTilemap, NMI request $17=$01: tilemap-upload index $22 is used by
; the item and bottle menus. Make its destination follow the active BG3
; tilemap; other indexes retain the vanilla table.
org $808CB0
    JML hook_NMITilemapUploadDestination
    NOP
    NOP
assert pc() == $808CB6

; First unused tilemap-upload selector: fixed destination for the relocated
; Module $15 BG3 page staged before $0219 switches to the overworld HUD.
org $8098AB
    db $3C

; NMI_UpdateBGChar3and4, NMI request $17=$09: rebase the first half of the
; overworld transition BG-character upload. It has a separate entry because
; it does not use the later common DMA setup.
org $808EE7
    JML hook_NMIUpdateBGChar3And4
    NOP
assert pc() == $808EEC

; NMI_UpdateBGChar5and6, NMI request $17=$0A: rebase the second half of the
; overworld transition BG-character upload. It has a separate entry because
; it does not use the later common DMA setup.
org $808F16
    JML hook_NMIUpdateBGChar5And6
    NOP
assert pc() == $808F1B

; NMI_RunTilemapUpdateDMA, shared by NMI requests $17=$0E-$11 and OBJ requests
; $13-$14: rebase destinations below $4000 for the overworld layout while
; leaving OBJ destinations unchanged.
org $808FC9
    JML hook_NMIBGCharacterDMASetup
    NOP
    NOP
assert pc() == $808FCF

; ParseText_SetWindowPosition, message rendering with no fixed game module:
; interpret its window position as an offset within the active BG3 tilemap
; instead of the fixed vanilla $6000 tilemap.
org $8EF0ED
    JSL hook_RenderTextPosition
    NOP
    NOP
assert pc() == $8EF0F3

; RenderText_SetDefaultWindowPosition, message rendering with no fixed game
; module: apply the same active-BG3 selection to the default window position.
org $8EFBDF
    JSL hook_RenderTextPosition
    NOP
    NOP
assert pc() == $8EFBE5

; RenderText_PostDeathSaveOptions, game-over module $12 submodule $08: replace
; its hard-coded BG3 position with the equivalent position in the active
; gameplay layout.
org $8EEE34
    JSL hook_RenderTextPostDeathSaveOptionsPosition
    BRA $04
assert pc() == $8EEE3A

; NMI_UpdateOWScroll, NMI request $17=$03: the BG streamer moved vanilla
; overworld stripes to its $1100/$18 list, so replace this handler with the
; generated ROM-to-VRAM character-row queue. Module $09 can request it from
; pre-scroll phase $02, scrolling phase $06, or post-scroll phase $08.
org $808D0F
return_NMIAfterGeneratedAssetList:

org $808D13
    JML hook_NMIBeforeGeneratedAssetList
    NOP
assert pc() == $808D18

; HandleOverworldTransitions normal-transition path, module $09 submodule $00:
; select the generated directional schedule after the destination screen and
; direction are finalized. Retain the displaced destination sprite-palette
; selection while suppressing the obsolete BG palette writes.
org $82A9F3
    JSL hook_ScrollTransitionBeforeAssetPreparation
    RTS

; Module09_LoadAuxGFX, module $09 submodules $01/$0F/$1A/$26: retain destination
; state changes, but omit obsolete BG/OBJ staging and uploads. Mosaic phase $0F
; installs the destination bundle while forced blank remains active.
; Continue directly into the following patched submodule in the same frame.
org $82AAAF
    JSL hook_Module09AuxGraphicsPreparation
    INC.b $11
    JMP.w $AABB                 ; Tail-dispatch submodule $02 in the same frame.
assert pc() == $82AAB8

; Module09_LoadNewMapAndGFX, ordinary scrolling phase $04: the generated
; transition schedule replaces conversion of the old area-OBJ staging buffer.
org $82AAD4
hook_Module09BeforeObsoleteSpriteConversion:
    BRA $02
    NOP
    NOP
assert pc() == $82AAD8

; Module09_TriggerTilemapUpdate, module $09 submodule $02: replace the vanilla
; $17=$0A BG-character upload with the pre-scroll generated batches, processing
; one batch per frame and holding this phase until its terminator.
org $82AABB
    JML hook_Module09BeforePreScrollAssetBatch
assert pc() == $82AABF

; RunScrollOverworldTransition, module $09 submodules $06/$14: ordinary phase
; $06 queues generated BG/OBJ batches; mosaic phase $14 already completed a
; forced-blank full reload. Neither path uses the old incremental OBJ upload.
org $82AADD
    JSL hook_RunScrollBeforeScrollStep
assert pc() == $82AAE1

; StartOverworldScrollTransition, module $09 ordinary scrolling phases $04/$05:
; vanilla calls this once from phase $04 and again as phase $05 so vertical
; transitions can prepare two initial BG2 stripes. Those stripes are obsolete,
; so advance directly from $04 to $06 while retaining the shared routine's
; direction setup. Mosaic phases $12/$13 keep their existing timing.
org $82AB26
    JSL hook_StartOverworldScrollTransition
    NOP
assert pc() == $82AB2B

; OverworldMosaicTransition_LoadSpriteGraphicsAndSetMosaic and
; OverworldMosaicTransition_FilterAndLoadGraphics, modules $09/$0B: generated
; forced-blank loads replace both the old conversion and incremental upload.
org $82B097
hook_MosaicBeforeObsoleteSpriteConversion:
    BRA $02
    NOP
    NOP
assert pc() == $82B09B

org $82B0BB
hook_MosaicBeforeObsoleteSpriteUpload:
    BRA $02
    NOP
    NOP
assert pc() == $82B0BF

; Module09_2E_0B, whirlpool phase $0B: generated batches have already loaded
; the destination OBJ rows, so retain only the palette-filter work.
org $82B39B
hook_WhirlpoolBeforeObsoleteSpriteUpload:
    BRA $02
    NOP
    NOP
assert pc() == $82B39F

; Module09_Overworld dispatch table, module $09 submodule $08: replace the
; Overworld_FinalizeEntryOntoScreen vector with a wrapper that delays vanilla
; finalization until all post-scroll generated batches have been consumed.
org $82A314
    dw hook_Module09BeforeFinalizingScrollTransition

org !free_space_bank_82_start
LoadModule15Overworld:
    JSL HideModule15Backgrounds
    JSL SelectOverworldVRAM
    JSR.w $E207                 ; Run hi-jacked instruction
    RTS

hook_Module09BeforeFinalizingScrollTransition:
    JSL ProcessNextPostScrollAssetBatch
    BCC .wait

    JMP.w $C17A                 ; Overworld_FinalizeEntryOntoScreen

.wait
    RTS
assert pc() <= !free_space_bank_82_end

org !free_space_bank_any_start_1

hook_FileLoadAfterSpriteCacheSeed:
    STA.l $7EC2FF               ; Run hi-jacked instruction

    LDA.b #!SpriteSeedAssetKey
    JSR SynchronousLoadOverworldAssets
    RTL

hook_IntroDefaultGraphicsBeforeTilemapClear:
    JSL $80893D                 ; Run hi-jacked instruction
    JSR SelectDefaultVRAM
    RTL

hook_LoadUnderworldEntranceBeforeEnvironmentFlag:
    JSR SelectDefaultVRAM

    ; Remove overworld tilemap data before the underworld room rebuild.
    JSL $80834B                 ; EraseTilemaps_normal

    LDA.b #$01
    STA.b $16                   ; Upload the HUD into the restored BG3 block.

    ; Run hi-jacked instructions:
    LDA.b #$01
    STA.b $1B                   ; Mark the active environment as indoors.

    RTL

hook_Module08LoadProperties:
    JSR SelectAndClearOverworldVRAM

    ; Run hi-jacked instructions:
    LDA.b #$82
    STA.b $99

    RTL

hook_Module08InitializeTilesets:
    LDA.b #$01
    STA.l !GeneratedAssetMarker

    ; Keep InitializeTilesets' common-sprite load. Its shared early hook returns
    ; before the generated area-dependent OBJ and static BG graphics.
    JSL $80E1DB                 ; Run hi-jacked instruction
    RTL

; Install generated source palettes and static BG characters after the
; vanilla palette selectors have established the area's palette state. Clear
; the marker before resuming the normal background-color/cache setup.
hook_Module08AfterBGPaletteSelection:
    JSR SynchronousLoadCurrentOverworldAssets
    LDA.b #$00
    STA.l !GeneratedAssetMarker

    JSL $8CFF91                 ; Run hi-jacked instruction
    RTL

; OverworldLoadScreensPaletteSet still loads sprite, equipment, Link, and HUD
; palettes. Suppress only its final main-background palette call while the
; generated loader owns BG palette rows 2-7.
hook_OverworldScreenPalettesAfterHUD:
    LDA.l !GeneratedAssetMarker
    BNE .skip

    JSL $9BEEC7                 ; Run hi-jacked instruction
.skip
    RTL

; OverworldPalettesLoader must still resolve palette IDs and load its sprite
; palettes. Suppress only the three auxiliary BG palette writes.
hook_OverworldPalettesAfterSetSelection:
    LDA.l !GeneratedAssetMarker
    BNE .skip

    ; Run hi-jacked instructions:
    JSL $9BEEE8                 ; PaletteLoad_OWBG1
    JSL $9BEF0C                 ; PaletteLoad_OWBG2
    JSL $9BEEA8                 ; PaletteLoad_OWBG3

.skip
    RTL

hook_WorldMapOverworldRestore:
    JSL $80893D                 ; Run hi-jacked instruction
    JSR SelectAndClearOverworldVRAM
    JSR SynchronousLoadCurrentOverworldAssets
    RTL

hook_TriforceRoomBeforeSpecialOverworldLoad:
    STZ.b $16                   ; Prevent NMI from restoring the HUD after the clear.
    JSR SelectAndClearOverworldVRAM
    RTL

hook_CreditsOverworldTilemapClear:
    JSR SelectAndClearCreditsOverworldVRAM
    RTL

hook_CreditsBeforeEndingTextBlankFill:
    LDA.w $0219
    XBA
    AND.w #$00FF               ; Convert $3C40/$6040 to $003C/$0060.
    STA.w $1002                ; Run hi-jacked instruction
    RTL

hook_CreditsBeforeEndingTextStripeCopy:
    PHA
    LDA.w $0219
    CMP.w #!OverworldHUD
    BNE .default_layout

    PLA
    SEC
    SBC.w #$0024               ; Rebase encoded $62xx/$63xx to $3Exx/$3Fxx.
    BRA .store

.default_layout
    PLA

.store
    ; Run hi-jacked instructions:
    STA.w $1008,X
    INY

    RTL

; Select the final layout and clear only its three tilemap regions. Callers
; are forced blank; no active-display path may call this routine.
SelectAndClearOverworldVRAM:
    PHP
    REP #$30
    PHX
    PHY

    JSL SelectOverworldVRAM

    LDY.w #BlankBG12Tile
    LDA.w #$6000
    LDX.w #$1000
    JSR ClearVRAMRegion

    LDY.w #BlankBG3Tile
    LDA.w #$3C00
    LDX.w #$0400
    JSR ClearVRAMRegion

    PLY
    PLX
    PLP
    RTS

SelectAndClearCreditsOverworldVRAM:
    PHP
    REP #$30
    PHX
    PHY

    JSL SelectOverworldVRAM

    LDY.w #BlankCreditsBG12Tile
    LDA.w #$6000
    LDX.w #$1000
    JSR ClearVRAMRegion

    LDY.w #BlankCreditsBG3Tile
    LDA.w #$3C00
    LDX.w #$0400
    JSR ClearVRAMRegion

    PLY
    PLX
    PLP
    RTS

; Select the final registers and layout-dependent NMI destinations without
; clearing VRAM. The Agahnim transition uses this while its destination is
; hidden, then fully replaces each region.
SelectOverworldVRAM:
    PHP
    SEP #$20
    LDA.b #$69                  ; BG1: $6800, 64x32
    STA.w $2107
    LDA.b #$61                  ; BG2: $6000, 64x32
    STA.w $2108
    LDA.b #$3C                  ; BG3: $3C00, 32x32
    STA.w $2109
    STZ.w $210B                 ; BG1/BG2 characters begin at $0000.

    REP #$20
    LDA.w #$1C00                ; Periodic animated BG characters.
    STA.w $0134
    LDA.w #!OverworldHUD
    STA.w $0219
    PLP
    RTL

; Queue the relocated BG3 tilemap immediately before the layout switch. Start
; with transparent tiles, then overlay the current HUD at its new $3C40
; destination so BG3 can remain visible across the switch.
PrepareModule15OverworldTilemaps:
    PHP
    REP #$30
    PHX

    LDA.w #$207F
    LDX.w #$07FE
.next_word
    STA.w $1000,X
    DEX
    DEX
    BPL .next_word

    LDX.w #$0148
.copy_hud
    LDA.l $7EC700,X
    STA.w $1080,X
    DEX
    DEX
    BPL .copy_hud

    SEP #$20
    LDA.b #$23
    STA.w $0116
    LDA.b #$01
    STA.b $17

    PLX
    PLP
    RTL

; Select the shared non-overworld layout. BG3 uses the reduced 32x32 tilemap
; while BG1/BG2 retain their vanilla underworld and presentation arrangement.
SelectDefaultVRAM:
    PHP
    SEP #$20
    LDA.b #$13                  ; BG1: $1000, 64x64
    STA.w $2107
    LDA.b #$03                  ; BG2: $0000, 64x64
    STA.w $2108
    LDA.b #$60                  ; BG3: $6000, 32x32
    STA.w $2109
    LDA.b #$22                  ; BG1/BG2 characters begin at $2000.
    STA.w $210B
    LDA.b #$00
    STA.l !GeneratedAssetMarker

    REP #$20
    LDA.w #!DefaultHUD
    STA.w $0219
    PLP
    RTS

; Clear X VRAM words beginning at A. Y points to a two-byte fill word in this
; bank. As in vanilla EraseTilemaps, separate fixed-source DMAs fill the low
; and high byte of every tilemap word.
ClearVRAMRegion:
    PHA

    STZ.w $2115
    STA.w $2116

    LDA.w #$1808
    STA.w $4310
    STY.w $4312
    STX.w $4315

    SEP #$20
    LDA.b #!free_space_bank_any_start_1>>16
    STA.w $4314
    LDA.b #$02
    STA.w $420B

    REP #$20
    PLA
    STA.w $2116
    LDA.w #$1908
    STA.w $4310
    INY
    STY.w $4312
    STX.w $4315

    SEP #$20
    LDA.b #$80
    STA.w $2115
    LDA.b #$02
    STA.w $420B

    REP #$20
    RTS

BlankBG12Tile:
    dw $01EC
BlankBG3Tile:
    dw $007F
BlankCreditsBG12Tile:
    dw $007F
BlankCreditsBG3Tile:
    dw $0188

; Early generated-overworld exit from InitializeTilesets after common sprites.
hook_InitializeTilesetsBeforeAreaGraphics:
    SEP #$20
    LDA.l !GeneratedAssetMarker
    BNE .generated

    ; Run hi-jacked instructions:
    REP #$30
    LDA.w $0AA3
    AND.w #$00FF

    JML $80E1F6

.generated
    SEP #$30
    PLB                         ; Balance InitializeTilesets' PHB.
    RTL

; Tail of InitializeTilesets' displaced background-VRAM setup.
hook_InitializeTilesetsBeforeBGSetup:
    REP #$30                    ; Run hi-jacked instruction

    LDA.w $0219
    CMP.w #!OverworldHUD
    BNE .default

    LDA.w #$0000
    BRA .store

.default
    LDA.w #$2000                ; Run hi-jacked instruction

.store
    JML $80E25E

; Load the asset record selected by A and synchronously process every batch
; in its full-reload sequence. This routine is for loading during forced blank,
; where there is no need to schedule batches onto individual frames.
; See plans/asset_loading.md for details.
;
; Direct-page scratch:
;   $00-$02: area asset-record pointer, then current batch pointer
;   $03-$05: full-load batch-sequence pointer
;   $06-$07: 16-bit copy of the 8-bit asset key
; DMA channel 1 is scratch during this forced-blank load. Flags, X, and Y are
; preserved for the surrounding module code.
SynchronousLoadCurrentOverworldAssets:
    PHP
    LDA.b $8A
    BRA SynchronousLoadOverworldAssetsStart

SynchronousLoadOverworldAssets:
    PHP
SynchronousLoadOverworldAssetsStart:
    REP #$10                    ; Use 16-bit X/Y for metadata offsets and cursors.
    PHX
    PHY

    JSR ResolveOverworldAssetRecord
    JSR ActivateAnimationTracks

    ; $03:$05 <- 24-bit pointer to full-load batch sequence
    LDY.w #$0000                ; Start at the full-load pointer in the record.
    LDA.b [$00],Y               ; Read it through the asset-record pointer.
    STA.b $03                   ; Batch-sequence address low.
    INY
    LDA.b [$00],Y
    STA.b $04                   ; Batch-sequence address high.
    INY
    LDA.b [$00],Y
    STA.b $05                   ; Batch-sequence bank.
    BEQ .done                  ; A zero bank means there is no full-load list.

    ; A sequence is [bank, address low, address high] repeated, followed by a
    ; single zero bank. Keep Y as its cursor while each batch uses its own Y.
    LDY.w #$0000                ; Begin at the first batch pointer.
.next_batch
    LDA.b [$03],Y               ; Read the bank first so zero can terminate.
    BEQ .done                   ; End after the last batch in the sequence.
    STA.b $02                   ; Current batch bank.
    INY                         ; Advance to the address low byte.
    LDA.b [$03],Y
    STA.b $00                   ; Current batch address low.
    INY                         ; Advance to the address high byte.
    LDA.b [$03],Y
    STA.b $01                   ; Current batch address high.
    INY                         ; Leave Y at the next sequence entry.

    ; SynchronousProcessAssetBatch consumes Y, so preserve the sequence cursor.
    PHY                         ; Save the next batch-pointer position.
    JSR SynchronousProcessAssetBatch
    PLY                         ; Resume walking the batch sequence.
    BRA .next_batch

.done
    PLY                         ; Restore the caller's registers and flags.
    PLX
    PLP
    RTS

; Resolve the asset record for key A and the current game state. Each pointer-
; table entry leads to ascending inclusive maximum-state entries:
;   maximum state, record address low, record address high, record bank
; A final maximum of $FF is mandatory, so the scan needs no fallback branch.
; Input: 8-bit A asset key; 16-bit X/Y. Output: $00-$02 record pointer.
ResolveOverworldAssetRecord:
    STA.b $06
    STZ.b $07                   ; Zero-extend the key for 16-bit arithmetic.

    REP #$20
    LDA.b $06
    ASL A
    CLC
    ADC.b $06
    TAX                         ; X = 3 * asset key.

    LDA.l !OverworldAssetBundlePointers,X
    STA.b $03
    SEP #$20
    LDA.l !OverworldAssetBundlePointers+2,X
    STA.b $05                   ; $03-$05 = variant-list pointer.

    LDY.w #$0000
.next_variant
    LDA.b [$03],Y
    CMP.l $7EF3C5
    BCS .found

    INY
    INY
    INY
    INY
    BRA .next_variant

.found
    INY
    LDA.b [$03],Y
    STA.b $00
    INY
    LDA.b [$03],Y
    STA.b $01
    INY
    LDA.b [$03],Y
    STA.b $02
    RTS

; Process one batch containing a null-terminated palette list followed by a
; null-terminated character list. Each descriptor is:
;   source bank, source address low, source address high, destination row
; Palette rows are 32 bytes; character rows are 16 4bpp tiles (512 bytes).
; Input: $00-$02 = batch pointer. Clobbers A, Y, and DMA channel 1.
SynchronousProcessAssetBatch:
    LDY.w #$0000

.next_palette
    ; Load the ROM source directly into DMA channel 1's A-bus address.
    LDA.b [$00],Y
    BEQ .palette_done
    STA.w $4314
    INY
    LDA.b [$00],Y
    STA.w $4312
    INY
    LDA.b [$00],Y
    STA.w $4313
    INY
    LDA.b [$00],Y
    INY

    ; Convert destination row n to $7EC300 + n*$20. WMDATA makes the ROM-to-
    ; WRAM transfer direct; vanilla code later derives $7EC500 and schedules
    ; the CGRAM upload appropriate to this module.
    REP #$20
    AND.w #$00FF
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    CLC
    ADC.w #$C300
    STA.w $2181
    LDA.w #$8000              ; DMA mode 0 to WMDATA.
    STA.w $4310
    LDA.w #$0020              ; One complete 16-color palette row.
    STA.w $4315

    SEP #$20
    LDA.b #$7E
    STA.w $2183               ; Complete the $7EC300 WRAM destination.
    LDA.b #$02
    STA.w $420B               ; Run DMA channel 1 while display is blank.
    BRA .next_palette

.palette_done
    ; Vanilla BG palette loaders preserve color 0 of palette row 2 because
    ; palette effects use it as a protected copy of the backdrop color.
    REP #$20
    LDA.l $7EC300
    STA.l $7EC340
    SEP #$20

    INY                       ; Skip the palette list's one-byte terminator.
    LDA.b #$80
    STA.w $2115               ; Increment VRAM after writes to $2119.

.next_character
    ; Parse the character descriptor's ROM source into the same DMA channel.
    LDA.b [$00],Y
    BEQ .done
    STA.w $4314
    INY
    LDA.b [$00],Y
    STA.w $4312
    INY
    LDA.b [$00],Y
    STA.w $4313
    INY
    LDA.b [$00],Y
    INY

    ; Character destination row n begins at VRAM word n*$100. The low byte is
    ; therefore zero and the descriptor row is the VMADDR high byte.
    STZ.w $2116
    STA.w $2117

    REP #$20
    LDA.w #$1801              ; DMA mode 1 to VMDATA.
    STA.w $4310
    LDA.w #$0200              ; Sixteen 32-byte 4bpp characters.
    STA.w $4315

    SEP #$20
    LDA.b #$02
    STA.w $420B               ; Run DMA channel 1 while display is blank.
    BRA .next_character

.done
    RTS

; Tail of NMI_UploadTilemap's displaced destination lookup.
hook_NMITilemapUploadDestination:
    LDX.w $0116                 ; Run hi-jacked instruction

    CPX.b #$22
    BNE .table

    LDA.w $021A                ; High byte of $3C40 or $6040.
    JML $808CB6

.table
    LDA.w $9888,X               ; Run hi-jacked instruction
    JML $808CB6

hook_NMIUpdateBGChar3And4:
    ; Run hi-jacked instructions:
    REP #$20
    LDA.w #$2C00

    JSR RebaseBGCharacterVRAM
    JML $808EEC

hook_NMIUpdateBGChar5And6:
    ; Run hi-jacked instructions:
    REP #$20
    LDA.w #$3400

    JSR RebaseBGCharacterVRAM
    JML $808F1B

hook_NMIBGCharacterDMASetup:
    JSR RebaseBGCharacterVRAM

    ; Run hi-jacked instructions:
    STA.w $2116
    LDA.w #$0000               ; Low word of source $7F0000.

    JML $808FCF

; A.w: VRAM destination. Move shared BG character writes down by $2000 only
; while the final overworld layout is active. OBJ and BG3 destinations are at
; or above $4000 and remain unchanged.
RebaseBGCharacterVRAM:
    PHA
    LDA.w $0219
    CMP.w #!OverworldHUD
    BNE .unchanged

    PLA
    CMP.w #$4000
    BCS .done

    SEC
    SBC.w #$2000
.done
    RTS

.unchanged
    PLA
    RTS

; RenderText_TextPosition contains the two vanilla positions relative to the
; default $6040 HUD destination. Preserve those offsets and add the active
; gameplay destination, defaulting safely when no save has initialized $0219.
hook_RenderTextPosition:
    LDA.l $0EFD3E,X             ; Run hi-jacked instruction
    SEC
    SBC.w #!DefaultHUD
    STA.w $1CD2                 ; Run hi-jacked instruction

    LDA.w $0219
    CMP.w #!OverworldHUD
    BEQ .add_offset
    LDA.w #!DefaultHUD

.add_offset
    CLC
    ADC.w $1CD2
    STA.w $1CD2
    RTL

hook_RenderTextPostDeathSaveOptionsPosition:
    REP #$20
    LDA.w $0219                 ; Active BG3 tilemap plus the vanilla offset.
    CLC
    ADC.w #$01A8
    STA.w $1CD2
    SEP #$20
    RTL

assert pc() <= !free_space_bank_any_end_1

org !free_space_bank_any_start_2

PrepareModule15HUDBeforeLayoutSwitch:
    LDA.w $06BA
    CMP.b #$1F
    BNE .return

    JSL PrepareModule15OverworldTilemaps

    REP #$20
    LDA.w #$1C00
    STA.w $0134
    SEP #$20

.return
    STZ.w $0200                 ; Run hi-jacked instruction
    RTL

; Module $15 enters its long overworld loader during vertical blank. Hide BG1
; and BG2 in both the mirrors and PPU before switching layouts; BG3 was staged
; earlier and remains visible.
HideModule15Backgrounds:
    PHP
    REP #$20

    LDA.b $1C
    STA.l !Module15LayerEnable
    AND.w #$FCFC
    STA.b $1C
    STA.w $212C

    PLP
    RTL

SetMirrorWarpPyramidTargetColor:
    SEP #$20
    LDA.b $10
    CMP.b #$15
    REP #$20
    BNE .black

    LDA.w #$7FFF
    BRA .store

.black
    LDA.w #$0000

.store
    ; Run hi-jacked instructions:
    STA.l $7EC500
    STA.l $7EC540
    JML return_MirrorWarpSetPyramidTargetColor

; ReloadPreviouslyLoadedSheets has already established DB=$80. Generated
; records replaced all eight static BG and four area-dependent OBJ sheets.
hook_ReloadPreviouslyLoadedSheetsBeforeGraphics:
    PLB                         ; Balance ReloadPreviouslyLoadedSheets' PHB.
    RTL

; Run InitializeTilesets with generated area graphics active. Its shared hook
; retains only the common-sprite load.
InitializeGeneratedOverworldCommonSprites:
    LDA.b #$01
    STA.l !GeneratedAssetMarker
    JSL $80E1DB                 ; Run hi-jacked instruction
    LDA.b #$00
    STA.l !GeneratedAssetMarker
    RTL

; Run OverworldPalettesLoader while retaining selector and OBJ-palette work
; but suppressing its three generated BG rows. Input A is its palette set.
LoadGeneratedOverworldSpritePalettes:
    PHA
    LDA.b #$01
    STA.l !GeneratedAssetMarker
    PLA
    JSL $8CFF18                 ; Run hi-jacked instruction
    LDA.b #$00
    STA.l !GeneratedAssetMarker
    RTL

hook_CreditsInitializeTilesets:
    LDA.b #$01
    STA.l !GeneratedAssetMarker
    JSL $80E1DB                 ; Run hi-jacked instruction
    RTL

hook_CreditsAfterPaletteSelection:
    JSL $8CFF18                 ; Run hi-jacked instruction
    LDA.b $11
    LSR A
    CLC
    ADC.b #!CreditsFirstAssetKey
    JSR SynchronousLoadOverworldAssets
    LDA.b #$00
    STA.l !GeneratedAssetMarker
    RTL

hook_CreditsCoolBackgroundInitializeTilesets:
    LDA.b #$01
    STA.l !GeneratedAssetMarker
    JSL $80E1DB                 ; Run hi-jacked instruction
    RTL

hook_CreditsCoolBackgroundAfterPaletteSelection:
    LDA.b #!CreditsCoolBackgroundAssetKey
    JSR SynchronousLoadOverworldAssets
    LDA.b #$00
    STA.l !GeneratedAssetMarker
    RTL

hook_TriforceInitializeTilesets:
    LDA.b #$01
    STA.l !GeneratedAssetMarker
    JSL $80E1DB                 ; Run hi-jacked instruction
    RTL

hook_TriforceAfterPaletteSelection:
    JSL $8CFF18                 ; Run hi-jacked instruction
    JSR SynchronousLoadCurrentOverworldAssets
    LDA.b #$00
    STA.l !GeneratedAssetMarker
    RTL

hook_MosaicDestinationPaletteSelection:
    JML LoadGeneratedOverworldSpritePalettes

hook_Module09BeforeSpecialSpriteReload:
    LDA.b $11
    CMP.b #$18
    BNE .check_exit

    LDA.b $A0                   ; Special entry uses the destination special-map ID.
    JSR SynchronousLoadOverworldAssets
    BRA .reload_sprites

.check_exit
    CMP.b #$25
    BNE .reload_sprites

    JSR SynchronousLoadCurrentOverworldAssets

.reload_sprites
    JSL $89AFD6                 ; Run hi-jacked instruction
    RTL

hook_Module09AuxGraphicsPreparation:
    LDA.b $11
    CMP.b #$0F
    BNE .return

    JSR SynchronousLoadCurrentOverworldAssets

.return
    RTL

hook_FluteLoadLandingScreenPalettes:
    LDA.b #$01
    STA.l !GeneratedAssetMarker
    JSL $82EA41                 ; Run hi-jacked instruction
    LDA.b #$00
    STA.l !GeneratedAssetMarker
    RTL

hook_FluteAfterPaletteSelection:
    JSR SynchronousLoadCurrentOverworldAssets
    LDA.b #$00
    STA.l !GeneratedAssetMarker
    JSL $8CFF91                 ; Run hi-jacked instruction
    RTL

hook_FluteInitializeTilesets:
    JML InitializeGeneratedOverworldCommonSprites

hook_WorldMapInitializeTilesets:
    JML InitializeGeneratedOverworldCommonSprites

; Mirror phase 1 has resolved the BG sheet IDs. Skip the obsolete OBJ-cache
; lookup, balance its saved X and DB, then submit the first generated batch.
hook_MirrorBeforeSpriteSheetResolution:
    SEP #$10
    PLX

    JSR InitializeFullAssetSchedule
    JSR ProcessNextFullAssetBatch
    JSR AdvanceMirrorGeneratedAssetPhase
    PLB
    RTL

hook_MirrorBeforeObsoleteSpriteGraphicsA:
    RTL

hook_MirrorBeforeObsoleteSpriteGraphicsB:
    JSL $87AA8B                 ; HandleFollowersAfterMirroring

    LDA.b $10
    CMP.b #$15
    BNE .return

    REP #$20
    LDA.l !Module15LayerEnable
    STA.b $1C
    SEP #$20

.return
    RTL

; Mirror phases 2-4 no longer need their vanilla BG decompression bodies.
hook_MirrorNextGeneratedAssetBatch:
    JSR ProcessNextFullAssetBatch
    JSR AdvanceMirrorGeneratedAssetPhase
    RTL

; Carry from ProcessNextFullAssetBatch says whether the submitted batch was
; final. Hold phase 4 when a longer generated sequence still has work.
AdvanceMirrorGeneratedAssetPhase:
    BCS .done
    LDA.w $0200
    CMP.b #$05
    BNE .return
    DEC.w $0200
    RTS

.done
    LDA.b #$05
    STA.w $0200
.return
    RTS

hook_MirrorWarpBeforeSpritesAndColors:
    LDA.b #$01
    STA.l !GeneratedAssetMarker
    JSL $82B27E                 ; Run hi-jacked instruction
    LDA.b #$00
    STA.l !GeneratedAssetMarker
    RTL

; Start a destination full reload in whirlpool phase $07. A one-batch sequence
; may skip phase $08 entirely.
hook_WhirlpoolBeforeGeneratedAssets:
    JSR InitializeFullAssetSchedule
    JSR ProcessNextFullAssetBatch
    BCC .wait

    LDA.b #$0F
    STA.b $13
    LDA.b #$09
    STA.b $B0
    BRA .return

.wait
    INC.b $B0
.return
    BRA ReturnFromWhirlpoolGeneratedAssetPhase

; Hold whirlpool phase $08 until the last full-reload batch is submitted.
hook_WhirlpoolBeforeNextGeneratedAssetBatch:
    JSR ProcessNextFullAssetBatch
    BCC .wait

    LDA.b #$0F
    STA.b $13
    INC.b $B0
.wait

ReturnFromWhirlpoolGeneratedAssetPhase:
    REP #$20
    PLA                         ; Discard Module09's local JSR return address.
    JML return_Module09AfterSubmodule

hook_WhirlpoolBeforePaletteWork:
    LDA.b #$01
    STA.l !GeneratedAssetMarker
    STZ.w $0AA9                 ; Run hi-jacked instruction
    JML $82B3F6

hook_WhirlpoolMainBackgroundPalette:
    RTL

hook_WhirlpoolAfterPaletteWork:
    LDA.b #$00
    STA.l !GeneratedAssetMarker
    RTL

; Select the full-reload sequence at record offset zero for the current $8A.
InitializeFullAssetSchedule:
    PHP
    REP #$10
    PHX
    PHY

    SEP #$20
    LDA.b $8A
    JSR ResolveOverworldAssetRecord
    JSR ActivateAnimationTracks
    LDA.b #$01
    STA.l !AnimationSuspended

    LDY.w #$0000
    LDA.b [$00],Y
    STA.l !OverworldAssetSchedule
    INY
    LDA.b [$00],Y
    STA.l !OverworldAssetSchedule+1
    INY
    LDA.b [$00],Y
    STA.l !OverworldAssetSchedule+2

    PLY
    PLX
    PLP
    RTS

; Submit one batch from a full-reload sequence without presenting its palette.
; Carry is set when the submitted batch is final (or the sequence is empty).
ProcessNextFullAssetBatch:
    REP #$10
    PHY

    JSR LoadAssetScheduleCursor
    LDY.w #$0000
    LDA.b [$35],Y
    BEQ .done
    STA.b $02
    INY
    LDA.b [$35],Y
    STA.b $00
    INY
    LDA.b [$35],Y
    STA.b $01
    INY
    JSR AdvanceAssetScheduleByY

    ; Peek before queuing because the queue replaces $35-$37 with its VRAM
    ; list pointer for NMI.
    JSR LoadAssetScheduleCursor
    LDY.w #$0000
    LDA.b [$35],Y
    PHA
    JSR QueueGeneratedAssetBatchSourceOnly
    PLA
    BEQ .done

    PLY
    SEP #$10
    CLC
    RTS

.done
    LDA.b #$00
    STA.l !AnimationSuspended
    PLY
    SEP #$10
    SEC
    RTS

; Retain the vanilla destination-dependent sprite palette selection, while
; the existing marker hooks suppress its obsolete background palette writes.
; Then select the generated directional asset schedule.
hook_ScrollTransitionBeforeAssetPreparation:
    ; Run hi-jacked instructions:
    LDX.b $8A
    LDA.l $7EFD40,X
    STA.b $00

    LDA.b #$01
    STA.l !GeneratedAssetMarker

    LDA.l $8CFE6C,X             ; Run hi-jacked instruction
    JSL $8CFF18                 ; Run hi-jacked instruction

    LDA.b #$00
    STA.l !GeneratedAssetMarker
    JSR InitializeScrollingAssetSchedule
    RTL

; Store the directional schedule selected by destination $8A and direction
; index $0418. Record offsets are south, north, east, west for indexes 0-3.
InitializeScrollingAssetSchedule:
    PHP
    REP #$10
    PHX
    PHY

    SEP #$20
    LDA.b $8A
    JSR ResolveOverworldAssetRecord
    JSR ActivateAnimationTracks
    LDA.b #$01
    STA.l !AnimationSuspended

    SEP #$10
    LDX.w $0418
    LDA.l .record_offsets,X
    TAY
    LDA.b [$00],Y
    STA.l !OverworldAssetSchedule
    INY
    LDA.b [$00],Y
    STA.l !OverworldAssetSchedule+1
    INY
    LDA.b [$00],Y
    STA.l !OverworldAssetSchedule+2

    REP #$10
    PLY
    PLX
    PLP
    RTS

.record_offsets
    db 12, 9, 6, 3

; Replace the obsolete mode-$0A preparation submodule. Ordinary scrolling
; transitions consume their initialized pre-scroll schedule one batch per
; frame. Mosaic phases $10/$1B/$27 reach the same code after their assets were
; synchronously loaded during forced blank, so advance them directly to map
; rebuilding or mosaic recovery.
hook_Module09BeforePreScrollAssetBatch:
    LDA.b $11
    CMP.b #$10
    BEQ .advance
    CMP.b #$1B
    BEQ .advance
    CMP.b #$27
    BNE .scheduled

.advance
    INC.b $11
    BRA .return

.scheduled
    JSL ProcessNextScheduledAssetBatch
    BCC .wait

    INC.b $11

.wait
.return
    JML $82AAC4                 ; Return through the vanilla RTS.

; Consume the optional generated batch for the zero-based scroll frame in
; $0126 only during ordinary phase $06. Mosaic phase $14 already loaded its
; assets synchronously.
hook_RunScrollBeforeScrollStep:
    LDA.b $11
    CMP.b #$14
    BEQ .return

    JSR ProcessScheduledScrollAssetBatch

.return
    RTL

hook_StartOverworldScrollTransition:
    ; Run hi-jacked instructions:
    INC.b $11
    LDX.w $0410

    LDA.b $11
    CMP.b #$05
    BNE .return

    INC.b $11
.return
    RTL

; Consume one pointer from a pre- or post-scroll section.
; Carry set: section terminator consumed. Carry clear: one batch submitted.
ProcessNextScheduledAssetBatch:
    REP #$10
    PHY

    JSR LoadAssetScheduleCursor
    LDY.w #$0000
    LDA.b [$35],Y
    BEQ .done
    STA.b $02
    INY
    LDA.b [$35],Y
    STA.b $00
    INY
    LDA.b [$35],Y
    STA.b $01
    INY
    JSR AdvanceAssetScheduleByY
    JSR QueueGeneratedAssetBatch

    PLY
    SEP #$10
    CLC
    RTL

.done
    INY                         ; Advance past the bank-$00 terminator.
    JSR AdvanceAssetScheduleByY

    PLY
    SEP #$10
    SEC
    RTL

; Post-scroll finalization can run for several frames while Link walks into
; the destination. Leave its terminating zero in place so every later frame
; continues to see a completed schedule instead of reading past its end.
ProcessNextPostScrollAssetBatch:
    REP #$10
    PHY

    JSR LoadAssetScheduleCursor
    LDY.w #$0000
    LDA.b [$35],Y
    BNE .batch

    LDA.b #$00
    STA.l !AnimationSuspended
    PLY
    SEP #$10
    SEC
    RTL

.batch
    PLY
    SEP #$10
    JML ProcessNextScheduledAssetBatch

; Consume the batch assigned to the scrolling frame which is about to run.
; The $FF terminator advances the shared cursor to the post-scroll section.
ProcessScheduledScrollAssetBatch:
    REP #$10
    PHY

    JSR LoadAssetScheduleCursor
    LDY.w #$0000
    LDA.b [$35],Y
    CMP.b #$FF
    BEQ .done
    CMP.w $0126
    BNE .return

    INY
    LDA.b [$35],Y
    STA.b $02
    INY
    LDA.b [$35],Y
    STA.b $00
    INY
    LDA.b [$35],Y
    STA.b $01
    INY
    JSR AdvanceAssetScheduleByY
    JSR QueueGeneratedScrollingAssetBatch
    BRA .return

.done
    INY                         ; Advance past the frame-$FF terminator.
    JSR AdvanceAssetScheduleByY

.return
    PLY
    SEP #$10
    RTS

; Copy the persistent ROM cursor into the direct-page long pointer used while
; parsing either a schedule or its current batch.
LoadAssetScheduleCursor:
    REP #$20
    LDA.l !OverworldAssetSchedule
    STA.b $35
    SEP #$20
    LDA.l !OverworldAssetSchedule+2
    STA.b $37
    RTS

; Advance the within-bank schedule address by the 16-bit byte count in Y.
; Generated schedules are guaranteed not to cross a metadata-bank boundary.
AdvanceAssetScheduleByY:
    REP #$20
    TYA
    CLC
    ADC.l !OverworldAssetSchedule
    STA.l !OverworldAssetSchedule
    SEP #$20
    RTS

; Process one generated batch during active display. Palette payloads go to
; the source mirror at $7EC300. Once all rows are installed, copy the complete
; source palette to $7EC500 so the retained sprite palette changes and the new
; BG rows become visible together. A nonempty VRAM list is queued for NMI.
;
; Input: $00-$02 = batch pointer. Clobbers A, X, Y, $03-$07, and DMA channel 1.
QueueGeneratedAssetBatch:
    LDA.b #$01
    STA.b $07                   ; Present palette changes after copying sources.
    BRA QueueGeneratedAssetBatchCommon

; Scrolling batches suppress dynamic graphics and HUD work but retain OAM so
; Link's position continues to follow the moving camera.
QueueGeneratedScrollingAssetBatch:
    LDA.b #$02
    STA.b $07                   ; Present palettes and retain OAM.
    BRA QueueGeneratedAssetBatchCommon

; Effect-covered full reloads update only the source palette. Their retained
; mirror/whirlpool filters later derive $7EC500 and CGRAM from $7EC300.
QueueGeneratedAssetBatchSourceOnly:
    STZ.b $07

QueueGeneratedAssetBatchCommon:
    LDY.w #$0000
    STZ.b $06                   ; Palette-dirty flag.

.next_palette
    LDA.b [$00],Y
    BEQ .palette_done
    STA.w $4314
    INY
    LDA.b [$00],Y
    STA.w $4312
    INY
    LDA.b [$00],Y
    STA.w $4313
    INY
    LDA.b [$00],Y
    INY

    REP #$20
    AND.w #$00FF
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    CLC
    ADC.w #$C300
    STA.w $2181                ; Destination row in WRAM bank $7E.
    LDA.w #$8000              ; DMA mode 0 to WMDATA.
    STA.w $4310
    LDA.w #$0020
    STA.w $4315                ; One complete palette row.

    SEP #$20
    LDA.b #$7E
    STA.w $2183
    LDA.b #$02
    STA.w $420B
    INC.b $06
    BRA .next_palette

.palette_done
    INY                         ; Skip the palette-list terminator.
    LDA.b $06
    BEQ .queue_vram

    ; Whole-row DMA overwrites the color-0 slot skipped by vanilla's loaders.
    ; Restore the protected backdrop copy before presenting the new palette.
    REP #$20
    LDA.l $7EC300
    STA.l $7EC340
    SEP #$20

    LDA.b $07
    BEQ .queue_vram

    REP #$30
    LDX.w #$0000
.copy_palette
    LDA.l $7EC300,X
    STA.l $7EC500,X
    INX
    INX
    CPX.w #$0200
    BNE .copy_palette

    SEP #$20
    INC.b $15                  ; Upload the complete synchronized palette.

.queue_vram
    REP #$20
    TYA
    CLC
    ADC.b $00
    STA.b $35                  ; $35-$37 = VRAM-list pointer.
    SEP #$20
    LDA.b $02
    STA.b $37

    LDY.w #$0000
    LDA.b [$35],Y
    BEQ .return

    LDA.b #$03
    STA.b $17

.return
    LDA.b #$01
    STA.w $0710                ; Suppress normal graphics and HUD DMA for this NMI.

    LDA.b $07
    CMP.b #$02
    BEQ .done

    LDA.b #$01
    STA.w $0702                ; The stationary/covered frame may omit OAM too.
.done
    RTS

; NMI mode $03: transfer every character-row descriptor directly from its ROM
; payload to VRAM. Descriptor destinations are VRAM rows, so they can be
; written directly to VMADDH with VMADDL fixed at zero.
hook_NMIBeforeGeneratedAssetList:
    REP #$10
    LDY.w #$0000

    LDA.b #$80
    STA.w $2115
    LDA.b #$01
    STA.w $4310                ; DMA mode 1.
    LDA.b #$18
    STA.w $4311                ; VMDATA port.

.next_descriptor
    LDA.b [$35],Y
    BEQ .done
    STA.w $4314
    INY
    LDA.b [$35],Y
    STA.w $4312
    INY
    LDA.b [$35],Y
    STA.w $4313
    INY
    STZ.w $2116
    LDA.b [$35],Y
    STA.w $2117
    INY

    STZ.w $4315
    LDA.b #$02
    STA.w $4316                ; One 512-byte character row.
    STA.w $420B                ; Run DMA channel 1.
    BRA .next_descriptor

.done
    SEP #$30
    JML return_NMIAfterGeneratedAssetList

assert pc() <= !free_space_bank_any_end_2

org !free_space_bank_any_start_3

; Copy the animation list at record offset 15 into compact runtime state.
; Each state entry is definition pointer low/high/bank, frame, countdown.
; Input: $00-$02 = selected asset-record pointer.
ActivateAnimationTracks:
    PHP
    SEP #$20
    REP #$10
    PHA
    PHX
    PHY

    LDA.b #$00
    STA.l !AnimationTrackCount
    LDY.w #$000F
    LDA.b [$00],Y
    STA.b $03
    INY
    LDA.b [$00],Y
    STA.b $04
    INY
    LDA.b [$00],Y
    STA.b $05
    BEQ .done

    LDY.w #$0000
    LDX.w #$0000
.next_track
    LDA.b [$03],Y
    BEQ .done
    STA.l !AnimationTrackState+2,X
    STA.b $08
    INY
    LDA.b [$03],Y
    STA.l !AnimationTrackState,X
    STA.b $06
    INY
    LDA.b [$03],Y
    STA.l !AnimationTrackState+1,X
    STA.b $07
    INY

    PHY
    LDY.w #$0002
    LDA.b [$06],Y
    STA.l !AnimationTrackState+3,X
    INY
    LDA.b [$06],Y
    STA.l !AnimationTrackState+4,X
    PLY

    INX
    INX
    INX
    INX
    INX
    LDA.l !AnimationTrackCount
    INC A
    STA.l !AnimationTrackCount
    BRA .next_track

.done
    PLY
    PLX
    PLA
    PLP
    RTS

; Replace PrepareOAMForTransfer's overworld animation timer. Dungeon layouts
; retain the displaced vanilla load and continue through its existing timer.
ScheduleOverworldAnimations:
    LDA.w $0219
    CMP.w #!OverworldHUD
    BEQ .overworld

    LDA.l $7EC00D              ; Run hi-jacked instruction
    JML $8086E3

.overworld
    PHP
    PHX
    PHY
    SEP #$20
    REP #$10
    JSR BuildAnimationDescriptorList
    SEP #$10
    PLY
    PLX
    PLP
    JML $808719

; Tick every active track and combine due frame descriptors in one WRAM list.
; The list begins with the standard empty palette terminator; $35-$37 points
; one byte past it because NMI consumes only the character descriptors.
BuildAnimationDescriptorList:
    LDA.l !AnimationTrackCount
    BNE .active
    RTS
.active
    STA.b $0D
    LDA.l !AnimationSuspended
    ORA.b $17
    ORA.w $0710
    BEQ .build
    RTS
.build

    LDA.b #$00
    STA.l !AnimationDescriptorList
    REP #$20
    LDA.w #$0001
    STA.b $09
    STZ.b $0B
    SEP #$20

.next_track
    LDX.b $0B
    LDA.l !AnimationTrackState,X
    STA.b $03
    LDA.l !AnimationTrackState+1,X
    STA.b $04
    LDA.l !AnimationTrackState+2,X
    STA.b $05

    LDA.l !AnimationTrackState+4,X
    DEC A
    STA.l !AnimationTrackState+4,X
    BNE .advance_state

    LDY.w #$0001
    LDA.b [$03],Y
    STA.l !AnimationTrackState+4,X
    LDA.l !AnimationTrackState+3,X
    INC A
    LDY.w #$0000
    CMP.b [$03],Y
    BCC .store_frame
    LDA.b #$00
.store_frame
    STA.l !AnimationTrackState+3,X
    STA.b $0E
    STZ.b $0F

    REP #$20
    LDA.b $0E
    ASL A
    CLC
    ADC.b $0E
    CLC
    ADC.w #$0004
    TAY
    SEP #$20

    LDA.b [$03],Y
    STA.b $08
    INY
    LDA.b [$03],Y
    STA.b $06
    INY
    LDA.b [$03],Y
    STA.b $07
    LDY.w #$0001

.next_descriptor
    LDA.b [$06],Y
    BEQ .advance_state
    LDX.b $09
    STA.l !AnimationDescriptorList,X
    INX
    INY
    LDA.b [$06],Y
    STA.l !AnimationDescriptorList,X
    INX
    INY
    LDA.b [$06],Y
    STA.l !AnimationDescriptorList,X
    INX
    INY
    LDA.b [$06],Y
    STA.l !AnimationDescriptorList,X
    INX
    INY
    STX.b $09
    BRA .next_descriptor

.advance_state
    REP #$20
    LDA.b $0B
    CLC
    ADC.w #$0005
    STA.b $0B
    SEP #$20
    DEC.b $0D
    BEQ .tracks_done
    JMP .next_track

.tracks_done
    LDX.b $09
    LDA.b #$00
    STA.l !AnimationDescriptorList,X
    CPX.w #$0001
    BEQ .return

    REP #$20
    LDA.w #!AnimationDescriptorList+1
    STA.b $35
    SEP #$20
    LDA.b #!AnimationDescriptorList>>16
    STA.b $37
    LDA.b #$03
    STA.b $17
.return
    RTS

; Retain the complete vanilla animated-background upload in dungeons. The
; overworld path returns after the surrounding dynamic OBJ transfers instead.
UploadOverworldAnimations:
    LDA.w $021A
    CMP.b #!OverworldHUD>>8
    BEQ .skip

    ; Run hi-jacked instructions:
    LDX.w $0ADC
    STX.w $4302

    JML $808B56

.skip
    JML $808B67

assert pc() <= !free_space_bank_any_end_3
