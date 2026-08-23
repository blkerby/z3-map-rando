; Stream overworld BG1/BG2 into 64x32 tilemaps. BG2 is fully streamed;
; BG1 uses the same renderer for bulk and gameplay edge loads. It applies to
; all areas, small and large. It differs from the vanilla
; streaming renderer (used by vanilla for BG2 in large areas) in that it draws new
; rows/columns in units of 8x8 tiles rather than 16x16, and only draws them
; across the visible area:
; rows of 33 tiles and columns of 29 tiles, compared to vanilla
; which would draw a 64x2 block (128 tiles). However, unlike vanilla,
; when both a row and column update are needed on the same frame,
; we don't try to spread them out onto different frames. It's ok because the
; updates are so much smaller.
;
; The fact that we draw only the visible region with no margin creates
; some complication with the mirror warp, where the HDMA wave effects could
; bring tiles into view outside the drawn window. This is handled by drawing
; margin columns when using the mirror. We also fix some vanilla issues
; with the mirror: vanilla has NMI overruns causing flashes of black
; bands at the top of the screen while using the mirror. It also
; begins reloading graphics while the palette is not yet fully faded to white;
; it can get away with this because the Dark World and Light World tilesets
; are aligned with each other, something we don't assume for custom tilesets.
; So we delay all reloading to happen under the cover of the fully white
; screen. Because of the elimination of the Map32 system (with its slow
; decompression step) this doesn't result in an overall slower sequence;
; in fact it's a bit faster than vanilla.

lorom

incsrc "symbols.inc"

!free_space_bank_a0_start = $A08900
!free_space_bank_a0_end = $A09900
!free_space_bank_82_start = $82F6FD
!free_space_bank_82_end = $82F720

; Temporary parameters for the shared BG1/BG2 renderer. This documented free
; WRAM is only scratch during a renderer call; no state persists between frames.
!BGMap16SourceOffset = $7EC900
!BGVRAMBase = $7EC902
!BGLogicalMask = $7EC904
!GeneratedBG1Enabled = $BFE406
!RainContexts = $BFE430
!OverworldAreaRecordVariantPointers = $A78000
!OverworldScreenSize = $82F5F1
!BackgroundSettings = $7ECC72
!BackgroundPositionX = $7ECC77
!BackgroundPositionY = $7ECC79
!BackgroundDeltaY = $7ECC7B
!BackgroundDeltaX = $7ECC7C
!MirrorFixedColor = $7EC90E

; Overworld_OperateCameraScroll
;
; Keep independent signed-byte camera deltas for generated BG1. Vanilla's
; 16-bit Y store at $069E overlaps the X byte at $069F.
org $82BB34
hook_OverworldCameraVerticalDelta:
    JSL RecordCameraDeltaY
    NOP

org $82BBEB
hook_OverworldCameraHorizontalDelta:
    JSL RecordCameraDeltaX
    NOP

; OverworldScrollTransition writes one cardinal-axis delta in 8-bit mode.
org $82BF4C
hook_OverworldTransitionCameraDelta:
    JSL RecordTransitionCameraDelta
    NOP
    NOP

; Credits_HandleCameraScrollControl duplicates the ordinary camera stores.
org $8ED906
hook_CreditsCameraVerticalDelta:
    JSL RecordCameraDeltaY
    NOP

org $8ED979
hook_CreditsCameraHorizontalDelta:
    JSL RecordCameraDeltaX
    NOP

; OverworldOverlay_HandleRain
;
; Every fourth game frame, the rain animation advances to another quarter of
; the 512x256 BG1 tilemap: 128 pixels right and 64 pixels down.
org $82A3C4
hook_run_rain_effects:
    JML RunRainEffectsWhenSelected
assert pc() == $82A3C8

org $82A407
    REP #$20

    LDA.b $E0
    CLC
    ADC.w #$0080
    STA.b $E0

    LDA.b $E6
    CLC
    ADC.w #$0040
    STA.b $E6

    SEP #$20
    BRA +
org $82A423
+

; ReloadSubscreenOverlay
;
; Generated gameplay backgrounds replace vanilla's area-specific synthetic
; overlay selection. Retain only rain's game-state predicate; authored BG1
; takes precedence through the generated rain-context table. Presentation
; modules continue through the vanilla selector.
org $82AE77
hook_select_generated_background:
    JML SelectGeneratedBackground
    NOP
assert pc() == $82AE7C

; Overworld_SetFixedColAndScroll
;
; During an ordinary area transition, vanilla immediately moves the Castle
; and Pyramid BG1 overlay to fixed coordinates while every other overlay keeps
; its previous position. Skip that special setup so the old area's BG1 does
; not jump when either destination is randomized next to another overlay.
org $8BFF76
    BRA return_OverworldSetFixedColAndScrollAfterOverlaySpecialCase

org $8BFF9D
return_OverworldSetFixedColAndScrollAfterOverlaySpecialCase:

; OverworldScrollTransition
;
; Vanilla advances BG2 each frame but omits the matching BG1 store for areas
; $1B and $5B. Apply the ordinary store for every area so Castle and Pyramid
; BG1 scroll smoothly through the transition too.
org $82BF65
    BRA return_OverworldScrollTransitionBG1Store

org $82BF6F
return_OverworldScrollTransitionBG1Store:

; Module09_LoadNewMapAndGFX
;
; Vanilla contexts: modules $09/$0B, submodules $03 and $11, while preparing
; the destination map for an area transition.
;
; Vanilla increments $0710 until its initial mode-$03 stripe upload clears it.
; That upload is disabled, so clear any reservation inherited from pre-scroll
; asset loading. The new edge preload sets $0710 again if it queues $18 work.
org $82AACE
    STZ.w $0710

; Module09_21
;
; Vanilla context: modules $09/$0B, submodule $21, after submodule $20
; restores the BG1 overlay on return from the world map.
;
; This forced-blank reload stage follows the BG1 overlay upload in submodule
; $20. Replace its obsolete small/large-area BG2 builder with the bulk
; renderer, then advance to Module09_22 to fade the screen back in.
org $82EA89
    JSL BG2BulkRender
    SEP #$20
    INC.b $11
    STZ.b $13
    RTS

; MirrorWarp_LoadSpritesAndColors
;
; Vanilla contexts: modules $09/$0B, submodules $23 and $2C, with $B0=$04
; while completing a mirror warp.
;
; The first half of this routine saves and adjusts the vanilla BG2 renderer
; cursors, calls BuildOverworldFromMap16, and restores the cursors. Skip
; directly to the sprite-palette work that follows.
org $82B283
    JMP.w $B2CE

; MirrorWarp_LoadSpritesAndColors has just selected the destination fixed
; color. Cache it before the remaining sprite work can run across an NMI;
; Module $15 keeps the PPU white until the normal fade resumes.
org $82B2EB
    JML CaptureMirrorWarpFixedColor
assert pc() == $82B2EF

; ReloadSubscreenOverlay sets the destination layer configuration before its
; relocated tilemaps are ready. Defer it during the active-display Agahnim
; transition so the saved configuration can be restored after the reload.
org $82AF5B
hook_ReloadSubscreenOverlaySetLayers:
    JML SetOverworldSubscreenLayers
assert pc() == $82AF5F

org $82AF63
return_ReloadSubscreenOverlaySetLayers:

; MirrorWarp_HandleCastlePyramidSubscreen enables the pyramid subscreen after
; the logical overlay load. Keep it hidden during the Agahnim transition.
org $82B22B
hook_MirrorWarpEnableSubscreenAfterOverlayLoad:
    JSL EnableMirrorWarpSubscreenUnlessModule15
assert pc() == $82B22F

; Module09_2E_Whirlpool
;
; The logical overlay loads remain, but the new streamer replaces the vanilla
; $0C/$0D BG1 half-tilemap uploads.
;
; $B0=$03, Module09_2E_03_FindDestination. ReloadSubscreenOverlayAndAdvance
; has just loaded the destination overlay and queued its BG1 bulk window.
; Preserve the displaced $15 clear, omit OAM for that NMI, and continue with
; the vanilla blue-screen setup.
org $82B3C7
    STZ.b $15
    INC.w !NMISkipOAM
    BRA return_WhirlpoolBeforeBlueRemoval
    NOP

; $B0=$04 or $06, Module09_2E_04. Vanilla queued one BG1 half through
; $17=$0D. That upload is obsolete, so advance directly to the next state.
org $82B3CF
    BRA return_WhirlpoolBeforeStateAdvance
    NOP
    NOP
    NOP
    NOP

; $B0=$05, Module09_2E_05_LoadDestinationMap. Overworld_LoadOverlayAndMap
; has just queued the BG2 bulk window. Replace the following $17=$0C upload
; with the OAM omission needed by the BG2 NMI, then enable the blue screen.
org $82B3D9
    INC.w !NMISkipOAM
    BRA return_WhirlpoolBeforeBlueScreenEnable
    NOP

org $82B426
return_WhirlpoolBeforeBlueRemoval:

org $82B42A
return_WhirlpoolBeforeBlueScreenEnable:

org $82B42E
return_WhirlpoolBeforeStateAdvance:

; MirrorWarp_Initialize
;
; Before changing $8A to the destination world, fill the source-world columns
; that the horizontal-scroll HDMA can expose outside the normal 33-tile
; windows.
org $82B158
    JSL hook_MirrorWarpBeforeWorldChange

; MirrorWarp_BuildWavingHDMATable
;
; Keep HDMA off on mirror frames that queue a large NMI transfer. The screen
; is white during these loading steps. While the destination tilemap is being
; staged, keep it off until the final margin repair has also been transferred.
org $80FE64
    JML hook_MirrorWarpWavingFrameAfterAnimation

; AnimateMirrorWarp_DrawDestinationScreen
;
; Unlike the ordinary full-load paths, this calls
; DrawOverworldQuadrantsAndOverlays as a subroutine and returns without
; falling through to OverworldBuildMapAndTrigger. Replace its final
; "INC $0710 : RTL" with a mirror wrapper around the common bulk renderer.
org $80D8F7
    JML RenderMirrorWarpBG2

; AnimateMirrorWarp step 5, AnimateMirrorWarp_TriggerOverlayA_2.
; MirrorWarp_HandleCastlePyramidSubscreen has just selected and loaded the
; logical overlay. Skip the following vanilla $17=$0C BG1 half upload and
; return through the routine's existing RTL at $80D8F2.
org $80D8EB
    BRA return_MirrorWarpAfterOverlayAUpload

org $80D8F2
return_MirrorWarpAfterOverlayAUpload:

; AnimateMirrorWarp step 8, AnimateMirrorWarp_DoSpritesPalettes.
; MirrorWarp_LoadSpritesAndColors has just calculated the destination BG1 base
; scrolls, but the common module tail has not copied them to the finalized
; scrolls yet. Finalize them before replacing the vanilla $17=$0C half upload
; with our BG1 bulk window.
org $80D8FF
hook_MirrorWarpBeforeBG1BulkRender:
    JML RenderMirrorWarpBG1AndHideBackgrounds

; The now-unreachable remainder of step 8 is reused as a bank-00 trampoline
; for the step-10 mirror margins.
org $80D903
    JML hook_MirrorWarpMarginPhase

; AnimateMirrorWarp_TriggerOverlayB is used by both steps 6 and 9. Its only
; purpose was the vanilla $17=$0D BG1 half upload, so both steps now return.
org $80D907
    RTL

; AnimateMirrorWarp_LoadSubscreen, step 11: preserve the destination subscreen
; state without exposing BG1 before its following character upload completes.
org $80DACD
hook_MirrorWarpSetDestinationSubscreen:
    JSL SetMirrorWarpDestinationSubscreen
assert pc() == $80DAD1

; AnimateMirrorWarp vector entry 10
;
; The common bulk renderer already installs the complete destination tilemap
; during step 7. Keep step 9 pointing at the no-op routine above, and redirect
; step 10 to the $80D903 trampoline so it adds margins after its normal
; animated-tile decompression.
org $80D880
    db $07, $03

; OverworldBuildMapAndTrigger
;
; This is the common full-overworld BG2 build path. Its vanilla entry paths
; are:
; - $08/$0A:$02:
;     Module08_02_LoadAndAdvance -> LoadAndBuildOverworldScreen
;     -> DrawOverworldQuadrantsAndOverlays -> this routine.
; - $09/$0B:$19:
;     Module09_19 -> Module08_02_LoadAndAdvance
;     -> LoadAndBuildOverworldScreen -> DrawOverworldQuadrantsAndOverlays
;     -> this routine.
; - $09/$0B:$21:
;     Module09_21 -> this routine directly.
; - $09/$0B:$28:
;     overworld submodule table -> LoadAndBuildOverworldScreen
;     -> DrawOverworldQuadrantsAndOverlays -> this routine.
; - $09/$0B:$2E with $B0=$05:
;     Module09_2E_05_LoadDestinationMap -> Overworld_LoadOverlayAndMap
;     -> LoadAndBuildOverworldScreen_long -> LoadAndBuildOverworldScreen
;     -> DrawOverworldQuadrantsAndOverlays -> this routine.
; - $0E:$0A with $0200=$08:
;     Module0E_0A_FluteMenu -> Overworld_LoadOverlayAndMap
;     -> the same LoadAndBuildOverworldScreen path.
; - Top-level $18 with $0200=$04:
;     Module18_04 -> Overworld_LoadOverlayAndMap
;     -> the same LoadAndBuildOverworldScreen path.
; - Top-level $19 with $B0=$04:
;     Module19_04_LoadAndSongAndAdvance -> Module08_02_LoadAndAdvance
;     -> the same LoadAndBuildOverworldScreen path.
; - Top-level $1A credits, with $11=$00,$04,$06,$08,$0A,$0C,$0E,$10,$12,
;   $18,$1A,$1C, or $1E and $B0=$02:
;     Credits_LoadNextScene_Overworld -> Credits_LoadOverworldScene
;     -> Credits_LoadOverworldScene_LoadMap -> LoadAndBuildOverworldScreen
;     -> DrawOverworldQuadrantsAndOverlays -> this routine.
;
; Except for $09/$0B:$21, these paths fall through from
; DrawOverworldQuadrantsAndOverlays; they do not call this label explicitly.
;
; Skip the BG2-only cursor setup and BuildOverworldFromMap16 call. Resume at
; the existing stack-restoration tail, then replace its BG2 mode $04 request
; with the new bulk renderer.
org $82EAF4
    BRA +
org $82EB04
+

org $82EB11
    JSL BG2BulkRender
    NOP
    NOP
    NOP

; DrawOverworldQuadrantsAndOverlays
;
; Vanilla replaces its first logical Map16 row at $7E4000-$7E407E with blank
; tile $0DBE. In vanilla, nothing appears to consume that cleared row, so it
; has no clear purpose: the VRAM tilemap is already populated before the clear.
; It would, however, mess up our streaming renderer, so we skip it.
org $82EC2E
    BRA return_DrawOverworldAfterFirstRowClear

org $82EC4C
return_DrawOverworldAfterFirstRowClear:

; SomeTilemapChange
;
; Module09_LoadNewMapAndGFX performs the same clear during scrolling
; area transitions. Again we skip it.
org $82ED51
    BRA return_ScrollingTransitionAfterFirstRowClear

org $82ED6B
return_ScrollingTransitionAfterFirstRowClear:

; LoadOverworldOverlay
;
; Playable-overworld paths retain the logical flat-overlay load but skip
; BuildBGOverlayFromMap16 and its $17=$04 full upload. Presentation modules
; retain the vanilla build and upload.
org $82FA7D
    JSR hook_LoadOverworldOverlayBeforeUploadRequest
    BEQ return_LoadOverworldOverlayAfterUploadRequest
    STA.b $17
    STA.w $0710

org $82FA87
return_LoadOverworldOverlayAfterUploadRequest:

; Overworld_LoadSubscreenAndSilenceSFX1
;
; Hook the overlay loading to set up the PPU for a 64x32 BG1 tilemap.
; Initial normal/special overworld loads reach this under forced
; blank, allowing the PPU to be configured. For transitions that have
; an activate display (mirror, whirlpool) the PPU setup is assumed to
; already be correct.
org $82AE3B
    JSL hook_LoadOverworldSubscreenBeforeSFX
    NOP

; LoadOverworldOverlay is shared by gameplay loads and self-contained scenes.
; Every caller keeps the logical overlay at $7E4000 and uses the 64x32 BG1
; ring. Module $15 queues its window later in the mirror sequence.
;
; The caller stores a nonzero return value in $17 as the NMI update mode:
;   $00: queue no BG1 upload (disabled or streamed overlay);
;   $04: upload the final credits' static shared BG1/BG2 tilemap;
;   $0D: upload rain's 64x32 tilemap from the former 4 KiB staging half.
; This routine uses free space left by the obsolete overlay loader.
org $82F544
hook_LoadOverworldOverlayBeforeUploadRequest:
    LDA.b $10
    CMP.b #$1A                  ; Final credits retain one static full tilemap.
    BNE .not_final_credits

    LDA.b $11
    CMP.b #$20
    BNE .not_final_credits

    REP #$20
    LDA.w #$6000                ; Rebase vanilla's shared map from $1000.
    STA.b $CC
    SEP #$20

    JSR.w $FA8A                 ; BuildBGOverlayFromMap16
    LDA.b #$04
    RTS

.not_final_credits
    LDA.b $10
    CMP.b #$15                  ; Mirror step 8 queues its streamed BG1.
    BEQ .done
    CMP.b #$19                  ; Triforce-room scrolls finalize one phase later.
    BEQ .done

    ; Rain is a complete static 64x32 tilemap. Build all 16 Map16 rows and
    ; return vanilla's 4 KiB former-half upload instead of a visible window.
    LDA.b $8C
    CMP.b #$9F
    BNE .streamed_window

    REP #$20
    LDA.w #$6800                ; Final overworld BG1 tilemap base.
    STA.b $CC
    SEP #$20

    JSR BuildRainTilemap
    LDA.b #$0D
    RTS

.streamed_window
    ; $11 selects the outer Module09/0B submodule. Values $23 and $2C both
    ; run the mirror animation state machine; this routine is reached during
    ; its inner $0200 step 5, before BG1 scrolls are finalized. Return without
    ; uploading here, because the step-8 hook uploads BG1 after
    ; Overworld_SetFixedColAndScroll.
    LDA.b $11
    CMP.b #$23                  ; Module09/0B:$23, normal mirror warp.
    BEQ .done
    CMP.b #$2C                  ; Module09/0B:$2C, return-portal mirror warp.
    BEQ .done

.render
    ; Other paths use the same earlier phase in which vanilla uploaded BG1.
    ; Their callers build BG2 in a later phase.
    ; The hobo overlay selects its lower logical half by adding $0100 to BG1
    ; after this call. Render from that final scroll now; vanilla will update
    ; the base scroll immediately afterward.
    LDA.b $8C
    CMP.b #$94
    BNE .scroll_ready

    REP #$20
    LDA.b $E6
    ORA.w #$0100
    STA.w $0124
    SEP #$20

.scroll_ready
    JSL BG1BulkRender

.done
    LDA.b #$00
    RTS

; Select the overworld's 64x32 BG1 layout, then reproduce the SFX write
; displaced at Overworld_LoadSubscreenAndSilenceSFX1.
hook_LoadOverworldSubscreenBeforeSFX:
    LDA.b #$69                  ; base=$6800, width=64, height=32
    STA.w $2107                 ; BG1SC

    ; Run hi-jacked instructions:
    LDA.b #$05                  ; SFX1.05
    STA.w $012D

    RTL

; Build rain's complete 64x32 tilemap in vanilla's former-half staging buffer.
; The shared builder walks backward, two Map16 rows per iteration, so adding
; $0800 starts at logical row 15 and eight iterations finish at row 0.
org $82F6B1
BuildRainTilemap:
    PHB

    LDA.b #$7F                  ; Short staging-buffer accesses use bank $7F.
    PHA
    PLB

    REP #$30

    LDA.w #$4000
    STA.b $04                   ; Low word of the 24-bit Map16 source pointer.

    LDA.w #$007E
    STA.b $06                   ; Pointer bank: $04-$06 now contains $7E4000.

    LDA.b $84                   ; Vanilla's cursor into the logical Map16 map.
    CLC
    ADC.w #$0800                ; 16 Map16 rows * $80 bytes per WRAM row.
    STA.b $84                   ; Begin the backward build at source row 15.

    STZ.b $0A                   ; Destination-address table cursor.
    STZ.b $0E                   ; Expanded tile-data cursor.

    LDA.w #$0008                ; Eight iterations * two rows = 16 rows.
    STA.b $08

    JMP.w $FABD                 ; Shared loop restores DB and returns.

; Convert the builder's logical row into a physical tilemap offset. Rain wraps
; through one 32-tile-high BG1 screen; every other vanilla builder retains its
; 64-tile-high upper/lower-screen selection.
BGBuilderCalculateRowOffset:
    LDA.b $88                   ; Logical Map16 row being placed in the tilemap.
    AND.w #$000F
    ASL A                       ; Map16 row * 64 VRAM words.
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    STA.b $00                   ; Physical row offset before adding the BG base.

    LDA.b $CC                   ; Destination base: $0000 for BG2, $1000 for BG1.
    BEQ .use_lower_screen       ; BG2 retains its vertical screen block.

    LDA.b $8C
    CMP.w #$009F
    BEQ .done                   ; Rain has no lower vertical screen block.

.use_lower_screen
    LDA.b $88
    AND.w #$0010
    BEQ .done

    LDA.b $00                   ; Select the lower pair of 32x32 screen blocks.
    ORA.w #$0800
    STA.b $00

.done
    RTS

assert pc() <= $82F6FD

org !free_space_bank_82_start

hook_Module19AfterPaletteCacheCopy:
    JSR.w $C44F                 ; Run hi-jacked instruction

    REP #$20
    LDA.w #$0100                ; Phase 4 will expose BG1's right-hand half.
    STA.w $0120                 ; Render that forthcoming viewport now.
    SEP #$20

    JSL BG1BulkRender
    INC.b $B0                   ; Run hi-jacked instruction
    RTS

hook_CreditsCoolBackgroundAfterOverlayLoad:
    REP #$20

    ; Run hi-jacked instructions:
    STZ.b $E0
    STZ.b $E6

    STZ.w $0120
    STZ.w $0124
    JMP.w $AE40                 ; Run hi-jacked instruction

assert pc() <= !free_space_bank_82_end

; CreateInitialNewScreenMapToScroll
;
; Vanilla contexts: modules $09/$0B, submodules $03 and $11, after the
; destination area's logical Map16 data has been loaded.
;
; This routine and its direction-specific callees only initialize the
; vanilla BG2 streaming cursors and build the first mode $03 stripe batch.
; Replace it with the new streamer's destination-edge preload (only needed
; for eastward transitions).
org $82ED95
    JSL hook_CreateInitialNewScreenMap
    RTS

; OverworldTransitionScrollAndLoadMap
;
; Vanilla contexts: modules $09/$0B during scrolling area transitions,
; including the final easing frames.
;
; The caller owns transition control and clears $0416 after this call. This
; routine's only contribution is another vanilla BG2 mode $03 stripe batch.
org $82EF72
    SEP #$30
    RTS

; OverworldHandleMapScroll
;
; Vanilla contexts: ordinary overworld camera movement and several scripted
; overworld movement sequences.
;
; This routine only consumes $0416 to build vanilla BG2 stripes and uses
; $0418 to serialize diagonal stripes across frames. Skip the whole consumer.
; Do not clear either variable here: $0418 also records the direction of
; transitions for movement code.
org $82EFD7
    RTS

; Credits_OperateScrollingAndTilemap
;
; Credits use the same overworld rings but update their scrolls outside the
; normal overworld module. Stream their finalized BG2 and BG1 edges each frame.
org $8285B9
    JML hook_CreditsAfterScrollUpdate
    NOP
    NOP
    NOP
    NOP
    NOP
assert pc() == $8285C2

; Credits_LoadCoolBackground resets its overlay scroll after loading it in
; vanilla. Reset both live and finalized scrolls first so the ring is built at
; the position the scene will display.
org $8285F1
    JSR hook_CreditsCoolBackgroundAfterOverlayLoad
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
    NOP
assert pc() == $8285FC

; Module19_03_PrepTileSetsPalette
;
; The Triforce room advances its BG1 horizontal scroll to $0100 in the next
; phase. Render that forthcoming viewport now, before BG2 is built.
org $829F7A
    JML hook_Module19AfterPaletteCacheCopy
    NOP
    NOP
assert pc() == $829F80

; CopyMap16ToBuffer
;
; Replace the vanilla 64-row physical destination calculation with the shared
; calculation above. Only the dedicated rain build omits the lower screen.
org $82FB28
    JSR BGBuilderCalculateRowOffset
    BRA +
org $82FB48
+

; OverworldCameraBoundaryCheck
;
; Ordinary overworld camera movement records every crossed 16-pixel boundary
; in $0416 solely to request work from OverworldHandleMapScroll. The new
; streamer compares the finalized 8-pixel scroll positions instead, so skip
; this obsolete producer.
org $82BCE4
    BRA +
org $82BCED
+

; Module09_Overworld / Module0B_OverworldSpecial
;
; These modules add the current shake offset to both layers after their
; submodule has run. Each replacement routine compares and stores its own
; finalized axis. The four calls append to one $1100 list in this order:
; BG2 horizontal, BG2 vertical, BG1 horizontal, BG1 vertical. The last call
; terminates and requests the combined list.
;
; The first five replaced bytes are:
;   STA $E2
;   STA $011E
org $82A37D
    JSL BG2StreamHorizontal
    NOP
;
; The second five replaced bytes are:
;   STA $E8
;   STA $0122
org $82A389
    JSL BG2StreamVertical
    NOP
;
; The third five replaced bytes are:
;   STA $E0
;   STA $0120
org $82A395
    JSL BG1StreamHorizontal
    NOP
;
; The fourth five replaced bytes are:
;   STA $E6
;   STA $0124
org $82A3A1
    JSL BG1StreamVertical
    NOP

; Module0E_Interface
;
; The item menu scrolls only BG3, and the world-map fade initially leaves the
; overworld visible. Leave generated BG1's finalized position untouched in
; those phases; actual Mode 7 phases retain vanilla's $E0/$E6 finalization.
org $80F861
hook_Module0EFinalizeBG1Scroll:
    JML FinalizeInterfaceBG1Scroll
assert pc() == $80F865

org $80F873
return_Module0EFinalizeBG1Scroll:

; DrawMap16Anywhere and AlterMap16Hardcore append immediate Map16 changes to
; the existing $1000/$14 stripe list. Keep those producers, but replace their
; duplicated vanilla 64x64 destination calculation with the 64x32 BG2 ring.
;
; AlterMap16Hardcore has already saved A, written the logical Map16 value, and
; pushed X. Its stripe emitter begins at $9BCA23 after this replacement.
org $9BC9E9
    STX.b $00
    JSR.w BG2FindMap16VRAMAddress
    BRA +
org $9BCA23
+

; Input: $00 = byte offset of the Map16 tile in the 64x64 WRAM map.
; Output: $02 = top-left BG2 VRAM word in the 64x32 tilemap.
;
; Each Map16 tile is two 8x8 tiles wide and high. Map16 column bit 4 selects
; the second 32x32 screen block; Map16 row bits 0-3 wrap through its 16
; physical Map16 rows.
org $9BCA69
BG2FindMap16VRAMAddress:
    LDA.b $00
    AND.w #$0020
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    STA.b $02                   ; $0000 or right-hand block $0400.

    LDA.b $00
    AND.w #$001F
    ORA.b $02
    STA.b $02                   ; Even 8x8 X within the screen block.

    LDA.b $00
    AND.w #$0780
    LSR A
    ORA.b $02
    CLC
    ADC.w #$6000
    STA.b $02                   ; Physical row * 32 words plus BG2 base.
    RTS

assert pc() <= $9BCA9F

;---------------------------------------------------------------------------------------------------
; Build the complete visible BG2 window as an arbitrary DMA list at $1100.
;
; The logical window starts at the 8x8 tile containing the viewport's
; top-left pixel and is always 33 tiles wide by 29 tiles high. BG2 is a
; 64x32 tilemap based at VRAM word $6000.
;
; Each 33-tile row crosses exactly one discontinuity in the two horizontal
; 32x32 screen blocks. Emit two $18 entries per row, writing tile words
; directly into their final positions in $1100-$1963:
;
;   29 * (66 data bytes + 8 header bytes) + 2-byte terminator = 2148 bytes
;
; Direct-page scratch:
;   $00: logical X coordinate of the window's left edge (in units of 8x8 tiles from area left)
;   $02: logical Y coordinate of the row being emitted (in units of 8x8 tiles from area top)
;   $04: bulk rows remaining, or runtime entering-edge offset
;   $06: logical X coordinate of the tile being emitted
;   $08: tiles remaining in the current segment
;   $0A: address scratch, or previous finalized tile before comparison
;   $0C: segment scratch
;   $0E: column destination X, signed tile delta, or runtime rows remaining
;
; Y is the byte offset into the $1100 list.
;---------------------------------------------------------------------------------------------------

org !free_space_bank_a0_start

RunRainEffectsWhenSelected:
    LDA.l !GeneratedBG1Enabled
    BEQ .run
    LDA.b $8C
    CMP.b #$9F
    BNE .done

.run
    LDA.b $8A
    CMP.b #$70
    JML $82A3C8

.done
    RTL

SelectGeneratedBackground:
    LDA.l !GeneratedBG1Enabled
    AND.w #$00FF
    BEQ .vanilla

    LDA.b $10
    AND.w #$00FF
    CMP.w #$0015
    BEQ .playable
    CMP.w #$0008
    BCC .vanilla
    CMP.w #$000C
    BCS .vanilla

.playable
    LDA.l $7EC213
    AND.w #$00FF
    TAX

    SEP #$20
    LDA.l !RainContexts,X
    REP #$20
    AND.w #$00FF
    BEQ .no_rain

    TXA
    CMP.w #$0070
    BEQ .mire

    LDA.l $7EF3C5
    AND.w #$00FF
    CMP.w #$0002
    BCS .no_rain
    BRA .rain

.mire
    LDA.l $7EF2F0
    AND.w #$0020
    BNE .no_rain

.rain
    LDX.w #$009F
    BRA .selected

.no_rain
    LDX.w #$0000

.selected
    LDY.w #$0390
    JML $82AF2D

.vanilla
    ; Run hi-jacked instructions:
    LDY.w #$0390
    LDA.b $8A

    JML $82AE7C

RecordCameraDeltaY:
    ; Run hi-jacked instructions:
    LDA.b $04
    STA.w $069E

    SEP #$20
    STA.l !BackgroundDeltaY
    REP #$20
    RTL

RecordCameraDeltaX:
    ; Run hi-jacked instructions:
    LDA.b $04
    STA.w $069F

    SEP #$20
    STA.l !BackgroundDeltaX
    REP #$20
    RTL

RecordTransitionCameraDelta:
    ; Run hi-jacked instructions:
    LDA.w $BDF2,Y
    STA.w $069E,X

    STA.l !BackgroundDeltaY,X
    RTL

; Read the five-byte background tail from the selected generated area record,
; initialize fixed-point BG1 camera positions, and configure the PPU caches.
; Returns carry set when generated presentation replaces vanilla.
ConfigureGeneratedBackground:
    SEP #$30
    LDA.l !GeneratedBG1Enabled
    BNE .enabled
    JMP .vanilla

.enabled
    LDA.b $10
    CMP.b #$15
    BEQ .playable
    CMP.b #$08
    BCS .at_least_overworld
    JMP .presentation

.at_least_overworld
    CMP.b #$0C
    BCC .playable
    JMP .presentation

.playable
    LDA.b $8C
    CMP.b #$9F
    BNE .resolve
    JMP .presentation

.resolve
    LDA.l $7EC213
    STA.b $06
    STZ.b $07

    REP #$30
    LDA.b $06
    ASL A
    CLC
    ADC.b $06
    TAX

    LDA.l !OverworldAreaRecordVariantPointers,X
    STA.b $03
    SEP #$20
    LDA.l !OverworldAreaRecordVariantPointers+2,X
    STA.b $05

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

    LDY.w #$0018
    LDX.w #$0000
.copy_settings
    LDA.b [$00],Y
    STA.l !BackgroundSettings,X
    INY
    INX
    CPX.w #$0005
    BNE .copy_settings

    REP #$20
    LDA.l $7EC213
    CMP.w #$0080
    BNE .initialize_camera

    LDA.b $A0
    CMP.w #$0181
    BNE .initialize_camera

    ; Area $80 shares one settings record with its fog variant. Keep Bridge
    ; Shadow in lockstep with BG2 and disable the fog drift.
    LDA.w #$0008
    STA.l !BackgroundSettings+1
    STA.l !BackgroundSettings+3

.initialize_camera
    REP #$30
    JSR BG1InitializeCamera
    SEP #$30

    LDA.l !BackgroundSettings
    BEQ .none
    CMP.b #$01
    BEQ .half_add

    ; Backdrop: BG2 is main, BG1 is sub, and only a black backdrop receives
    ; BG1 through color math where BG2 has a transparent pixel.
    REP #$20
    LDA.w #$0000
    STA.l $7EC300
    STA.l $7EC340
    STA.l $7EC500
    STA.l $7EC540
    SEP #$20
    INC.b $15

    LDA.b #$16
    LDX.b #$01
    LDY.b #$20
    BRA .store_layers

.half_add
    LDA.b #$16
    LDX.b #$01
    LDY.b #$72
    BRA .store_layers

.none
    LDA.b #$16
    LDX.b #$00
    LDY.b #$20

.store_layers
    STY.b $9A

    PHA
    LDA.l !Module15LayerEnable
    BMI .defer_active

    LDA.b $10
    CMP.b #$15
    PLA
    BEQ .defer_layers

    STA.b $1C
    STX.b $1D
    SEC
    RTS

.defer_active
    PLA

.defer_layers
    ORA.b #$80                 ; Retain the persistent Module $15 marker.
    STA.l !Module15LayerEnable
    TXA
    STA.l !Module15LayerEnable+1
    SEC
    RTS

.presentation
    LDA.b #$FF
    STA.l !BackgroundSettings

.vanilla
    CLC
    RTS

SetOverworldSubscreenLayers:
    JSR ConfigureGeneratedBackground
    BCS .generated

    LDA.l !Module15LayerEnable
    BMI .defer

    LDA.b $10
    CMP.b #$15
    BEQ .defer

    ; Run hi-jacked instructions:
    LDA.b #$16
    STA.b $1C
    LDA.b #$01
    STA.b $1D
    JML return_ReloadSubscreenOverlaySetLayers

.defer
    LDA.b #$96                 ; $16 plus the persistent Module $15 marker.
    STA.l !Module15LayerEnable
    LDA.b #$01
    STA.l !Module15LayerEnable+1
    JML return_ReloadSubscreenOverlaySetLayers

.generated
    JML $82AFAD

EnableMirrorWarpSubscreenUnlessModule15:
    ; Keep BG1 hidden until step 11, after its bulk tilemap render.
    LDA.b #$01                 ; Run hi-jacked instruction without storing it.
    RTL

SetMirrorWarpDestinationSubscreen:
    LDA.b $10
    CMP.b #$15
    BNE .enable

    LDA.b #$01
    STA.l !Module15LayerEnable+1
    RTL

.enable
    ; Run hi-jacked instructions:
    LDA.b #$01
    STA.b $1D
    RTL

hook_CreditsAfterScrollUpdate:
    REP #$20

    LDA.b $E2
    JSL BG2StreamHorizontal
    LDA.b $E8
    JSL BG2StreamVertical
    LDA.b $E0
    JSL BG1StreamHorizontal
    LDA.b $E6
    JSL BG1StreamVertical

    SEP #$20
    RTL

BG2BulkRender:
    PHP                         ; Preserve the caller's register-width flags.

    REP #$30                    ; Use 16-bit A, X, and Y for offsets and words.

    PHA                         ; The load-state tail does not expect us to
    PHX                         ; change any registers, so save all three.
    PHY

    SEP #$20
    LDA.b #$61                  ; base=$6000, width=64, height=32
    STA.w $2108                 ; BG2SC
    REP #$20

    JSR BG2SelectRenderer

    LDA.b $10
    AND.w #$00FF
    CMP.w #$0019
    BNE .calculate_world_origin

    ; The Triforce room's flat special map starts at logical (0,0); its $88
    ; area ID is not an ordinary overworld screen-grid coordinate.
    STZ.b $00
    STZ.b $02
    BRA .render

.calculate_world_origin
    JSR BG2CalculateLogicalWindowOrigin
    ; $00 = logical X tile,  $02 = logical Y tile

.render
    JSR BGBulkRender

    PLY                         ; Restore registers in reverse push order.
    PLX
    PLA

    PLP                         ; Restore the caller's original widths.
    RTL

; Select the layer parameters consumed by the shared renderer.
BG2SelectRenderer:
    LDA.w #$0000
    STA.l !BGMap16SourceOffset  ; BG2 Map16 begins at $7E2000.

    LDA.w #$6000
    STA.l !BGVRAMBase           ; BG2 tilemap begins at VRAM word $6000.

    LDA.w #$007F
    STA.l !BGLogicalMask        ; BG2 logical coordinates are 0..127.
    RTS

; Build the visible BG1 overlay in the loading phase before BG2. The compact
; PPU layout was already selected at the common overworld BG1-load entry.
; Disabled overlays do not normally reach this routine, but checking $1D keeps
; the ownership boundary explicit. Rain ($8C=$9F) uses its separate static path.
BG1BulkRender:
    PHP                         ; Preserve the caller's register-width flags.

    REP #$30                    ; Save complete 16-bit register values.
    PHA
    PHX
    PHY

    SEP #$20                    ; Overlay controls are byte-wide.

    LDA.b $10
    CMP.b #$15
    BNE .read_layer_mirror

    LDA.l !Module15LayerEnable+1 ; Its live mirror stays hidden during loading.
    BRA .layer_ready

.read_layer_mirror
    LDA.b $1D                   ; Zero means BG1 is disabled for this area.

.layer_ready
    BEQ .skip

    LDA.b $8C                   ; Synthetic overlay ID selected by the loader.
    CMP.b #$9F                  ; Rain uses a dedicated static 64x32 tilemap.
    BEQ .skip

    REP #$20

    JSR BG1SelectRenderer
    JSR BG1CalculateLogicalWindowOrigin
    ; $00 = logical X tile,  $02 = logical Y tile

    JSR BGBulkRender

.skip
    REP #$20                    ; Pull the 16-bit values saved above.
    PLY
    PLX
    PLA

    PLP
    RTL

; Persistent destination overlays may have queued immediate $1000/$14 stripes
; while building the logical map. The complete BG2 render below includes those
; changes, so discard the redundant request and reset its append cursor exactly
; as NMI would after consuming request $14=$01.
RenderMirrorWarpBG2:
    LDA.b $10
    CMP.b #$15
    BNE .render

    ; BG2 is about to replace the old HUD page at $6000. Switch to the staged
    ; $3C00 BG3 page first while only its relocated HUD is enabled.
    PHP
    SEP #$20
    LDA.b #$69
    STA.w $2107                 ; BG1: $6800, 64x32.
    LDA.b #$61
    STA.w $2108                 ; BG2: $6000, 64x32.
    LDA.b #$3C
    STA.w $2109                 ; BG3: $3C00, 32x32.
    STZ.w $210B                 ; BG1/BG2 characters begin at $0000.
    PLP

.render
    STZ.b $14
    STZ.w $1000
    STZ.w $1001
    JML BG2BulkRender

; Make the mirror bulk renderer use the destination scroll which the common
; module tail will otherwise finalize only after this loading step returns.
; Include shake exactly as that tail does, render BG1 while its destination
; enable is still live, then hide BG1/BG2 until the final restore.
RenderMirrorWarpBG1AndHideBackgrounds:
    PHP
    REP #$30
    PHX
    PHY

    JSR BG1UseGeneratedCamera
    BCC .position_ready
    JSR BG1InitializeCamera

.position_ready

    LDA.b $E0
    CLC
    ADC.w $011A
    STA.w $0120

    LDA.b $E6
    CLC
    ADC.w $011C
    STA.w $0124

    PLY
    PLX
    PLP
    JSL BG1BulkRender

    LDA.b #$00
    RTL

; Initialize BG1 positions from the camera's local coordinate in signed
; eighth-pixel fixed point. Drift phase starts at zero on each full load.
BG1InitializeCamera:
    LDA.l $7EC213
    AND.w #$0007
    XBA
    ASL A
    STA.b $0C

    LDA.b $E2
    SEC
    SBC.b $0C
    LDX.w #$0001
    JSR BG1MultiplyByFollow
    STA.l !BackgroundPositionX
    LSR A
    LSR A
    LSR A
    STA.b $E0
    STA.w $0120

    LDA.l $7EC213
    AND.w #$0038
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    STA.b $0C

    LDA.b $A0
    CMP.w #$0181
    BNE .use_local_y

    LDA.b $E8
    ORA.w #$0100              ; Match vanilla's Bridge Shadow vertical offset.
    BRA .multiply_y

.use_local_y
    LDA.b $E8
    SEC
    SBC.b $0C

.multiply_y
    LDX.w #$0003
    JSR BG1MultiplyByFollow
    STA.l !BackgroundPositionY
    LSR A
    LSR A
    LSR A
    STA.b $E6
    STA.w $0124
    RTS

; A: signed pixel delta; X: follow setting offset. Returns eighth-pixel delta.
BG1MultiplyByFollow:
    STA.b $0A
    LDA.l !BackgroundSettings,X
    AND.w #$00FF
    TAY
    LDA.w #$0000

.next
    CPY.w #$0000
    BEQ .done
    CLC
    ADC.b $0A
    DEY
    BRA .next

.done
    RTS

; Sign-extend the byte in A for fixed-point camera arithmetic.
BG1SignExtendByte:
    AND.w #$00FF
    CMP.w #$0080
    BCC .done
    ORA.w #$FF00

.done
    RTS

; A: signed camera delta; X: follow setting offset (1 for X, 3 for Y).
; Returns the new integer BG1 scroll before screen shake.
BG1AdvanceDataPosition:
    PHY
    JSR BG1MultiplyByFollow
    STA.b $0E

    INX                         ; Select this axis's signed drift byte.
    LDA.l !BackgroundSettings,X
    JSR BG1SignExtendByte

    CLC
    ADC.b $0E
    CPX.w #$0002
    BNE .vertical

    CLC
    ADC.l !BackgroundPositionX
    STA.l !BackgroundPositionX
    BRA .integer

.vertical
    CLC
    ADC.l !BackgroundPositionY
    STA.l !BackgroundPositionY

.integer
    LSR A
    LSR A
    LSR A
    PLY
    RTS

; Return carry set when gameplay should use generated camera behavior.
BG1UseGeneratedCamera:
    SEP #$20
    LDA.l !GeneratedBG1Enabled
    BEQ .vanilla
    LDA.l !BackgroundSettings
    CMP.b #$FF
    BEQ .vanilla
    LDA.b $8C
    CMP.b #$9F
    BEQ .vanilla

    REP #$20
    SEC
    RTS

.vanilla
    REP #$20
    CLC
    RTS

; Leave generated BG1 unchanged during the item menu and the initial fade into
; either world map. Also retain it when an interface has handed control back
; to another module during the current frame. Every other interface phase
; retains vanilla behavior.
FinalizeInterfaceBG1Scroll:
    SEP #$20
    LDA.b $1B
    BNE .vanilla_8bit

    LDA.b $10
    CMP.b #$0E
    BNE .check_generated

    LDA.b $11
    CMP.b #$01
    BEQ .check_generated
    CMP.b #$07
    BEQ .world_map
    CMP.b #$0A
    BNE .vanilla_8bit

.world_map
    LDA.w $0200
    BNE .vanilla_8bit

.check_generated
    REP #$20
    JSR BG1UseGeneratedCamera
    BCS .done
    BRA .vanilla

.vanilla_8bit
    REP #$20

.vanilla
    ; Run hi-jacked instructions:
    LDA.b $E0
    CLC
    ADC.w $011A
    STA.w $0120

    LDA.b $E6
    CLC
    ADC.w $011C
    STA.w $0124

.done
    JML return_Module0EFinalizeBG1Scroll

; Select the BG1 overlay source and VRAM destination for the shared renderer.
BG1SelectRenderer:
    LDA.w #$2000
    STA.l !BGMap16SourceOffset  ; BG1 Map16 begins at $7E4000.

    LDA.w #$6800
    STA.l !BGVRAMBase           ; BG1 tilemap begins at VRAM word $6800.

    LDA.w #$003F
    STA.l !BGLogicalMask

    JSR BG1UseGeneratedCamera
    BCC .done

    LDA.l $7EC213
    AND.w #$00FF
    TAX
    LDA.l !OverworldScreenSize,X
    AND.w #$00FF
    BNE .done

    LDA.w #$007F
    STA.l !BGLogicalMask        ; Large BG1 areas use coordinates 0..127.

.done
    RTS

; Convert finalized BG1 scrolls directly to the logical 8x8 tile at the
; viewport's top-left pixel. Unlike BG2, the overlay is always loaded at
; logical (0,0), so no world-space screen origin is subtracted.
BG1CalculateLogicalWindowOrigin:
    LDA.w $0120                 ; Final BG1 horizontal scroll in pixels.
    LSR A
    LSR A
    LSR A
    AND.l !BGLogicalMask
    STA.b $00

    LDA.w $0124                 ; Final BG1 vertical scroll in pixels.
    LSR A
    LSR A
    LSR A
    AND.l !BGLogicalMask
    STA.b $02
    RTS

; Build and request the complete 33x29 window for the selected layer.
BGBulkRender:
    LDA.w #$001D
    STA.b $04                   ; Initialize remaining rows to 29.

; Build and request a $1100/$18 list containing $04 rows beginning at $02.
BGBuildRowList:
    LDY.w #$0000

.next_row
    JSR BGEmitRow

    LDA.b $02
    INC A
    AND.l !BGLogicalMask
    STA.b $02

    DEC.b $04
    BNE .next_row

    ; Reserve NMI time for this tilemap transfer. The $18 handler clears both
    ; controls after consuming the complete list.
    LDA.w #$0001
    STA.w $0710

    JSR BGFinishList

    RTS

; Terminate and request a nonempty $1100 list assembled by the selected layer.
BGFinishList:
    LDA.w #$FFFF
    STA.w $1100,Y

    SEP #$20
    LDA.b #$01
    STA.b $18
    REP #$20
    RTS

;---------------------------------------------------------------------------------------------------
; Preload the offscreen destination edge for east transitions.
;
; The resident window includes one extra column on the right and one extra
; row on the bottom. Loading an eastern destination invalidates the extra
; column without moving the camera, so refresh it before the first 8-pixel
; scroll step. South needs no seed: its $x11E->$x120 alignment correction
; crosses a tile boundary and invokes the normal vertical streamer.
;---------------------------------------------------------------------------------------------------

hook_CreateInitialNewScreenMap:
    PHP
    REP #$30

    PHA
    PHX
    PHY

    LDA.w $0410                 ; Transition direction bit.
    AND.w #$00FF
    CMP.w #$0001                ; East.
    BNE .done                   ; Other directions expose or stream their edge.

    JSR BG2SelectRenderer
    JSR BG2CalculateLogicalWindowOrigin

    LDA.b $00
    CLC
    ADC.w #$0020                ; Preload the current offscreen right column.
    AND.w #$007F
    STA.b $06
    STA.b $0E                   ; Source and destination X are the same.

    LDY.w #$0000
    JSR BGEmitColumn

    JSR BGFinishList            ; Transfer the seed edge before scrolling starts.

.done
    PLY
    PLX
    PLA

    PLP
    RTL

;---------------------------------------------------------------------------------------------------
; Store finalized horizontal scroll and append an entering column.
;
; Input:
;   A: new BG2 horizontal scroll in pixels, including shake
;
; Output:
;   Y: byte offset after the optional column in the $1100 list
;
; The column uses the previous finalized vertical tile. If vertical movement
; exposed corner tiles, BG2StreamVertical appends their rows afterward and
; overwrites those intersections with the same final data.
;---------------------------------------------------------------------------------------------------

BG2StreamHorizontal:
    STA.b $E2                   ; Run hi-jacked instruction

    PHP                         ; The caller has 16-bit A but 8-bit X/Y.
    REP #$30

    PHA                         ; Preserve registers other than the Y result.
    PHX

    LDY.w #$0000                ; Start this frame's gameplay list at $1100.

    LDA.w $011E
    LSR A                       ; Convert the old scroll to its 8x8 tile column.
    LSR A
    LSR A
    STA.b $0A

    LDA.b $E2

    STA.w $011E                 ; Run hi-jacked instruction
    LSR A
    LSR A
    LSR A
    SEC
    SBC.b $0A                   ; Signed horizontal tile delta.
    STA.b $0E
    BEQ .done                   ; Most frames expose no horizontal edge.

    ; The bulk renderer may already own $1100 this frame.
    LDA.b $18
    AND.w #$00FF
    BNE .done

    LDA.b $0E
    CMP.w #$0001
    BEQ .edge_ready

    CMP.w #$FFFF
    BNE .done                   ; No boundary crossed, or this was a larger jump.

.edge_ready
    JSR BG2SelectRenderer
    JSR BG2CalculateLogicalWindowOrigin
    JSR BGStreamHorizontalEdge

.done
    PLX
    PLA

    PLP
    RTL

;---------------------------------------------------------------------------------------------------
; Store finalized vertical scroll and append entering rows.
;
; Input:
;   A: new BG2 vertical scroll in pixels, including shake
;   Y: byte offset after any column emitted by BG2StreamHorizontal
;
; BG1 follows this call, so BG1StreamVertical finishes the combined list.
;---------------------------------------------------------------------------------------------------

BG2StreamVertical:
    STA.b $E8                   ; Run hi-jacked instruction

    PHP                         ; The caller has 16-bit A but 8-bit X/Y.
    REP #$30                    ; Use 16-bit coordinates, offsets, and tiles.

    PHA                         ; Preserve registers other than the Y cursor.
    PHX

    LDA.w $0122                 ; Convert the previous finalized Y scroll to its
    LSR A                       ; world-space 8x8 tile row.
    LSR A
    LSR A
    STA.b $0A

    LDA.b $E8

    STA.w $0122                 ; Run hi-jacked instruction
    LSR A                       ; Convert the new scroll to its tile row.
    LSR A
    LSR A
    SEC
    SBC.b $0A                   ; Signed vertical tile delta.
    STA.b $0E
    BEQ .done                   ; Most frames expose no vertical edge.

    ; The bulk renderer may already have prepared $1100 earlier this frame.
    ; Keep its complete-window upload instead of replacing it with edges.
    LDA.b $18
    AND.w #$00FF
    BNE .done

    ; Keep only the vertical deltas supported by gameplay. A normal frame
    ; exposes one row; the final north-transition frame can expose two above.
    LDA.b $0E
    CMP.w #$0001
    BEQ .vertical_delta_ready

    CMP.w #$FFFF
    BEQ .vertical_delta_ready

    CMP.w #$FFFE
    BEQ .vertical_delta_ready

    BRA .done                   ; Ignore larger jumps; bulk loading owns those.

.vertical_delta_ready
    JSR BG2SelectRenderer
    JSR BG2CalculateLogicalWindowOrigin
    JSR BGStreamVerticalEdge

.done
    PLX
    PLA

    PLP                         ; Restore the caller's original register widths.
    RTL

;---------------------------------------------------------------------------------------------------
; Store finalized BG1 scrolls and append overlay edges to the gameplay list.
;
; BG1 uses its area-sized logical map directly, so these wrappers differ from
; BG2 only in the scroll addresses, enable checks, and renderer selection.
; Rain remains on its separate static path and never reaches the emitters.
;---------------------------------------------------------------------------------------------------

BG1StreamHorizontal:
    PHP                         ; The caller has 16-bit A but 8-bit X/Y.
    REP #$30

    PHA                         ; Preserve registers other than the Y cursor.
    PHX
    STA.b $0C

    JSR BG1UseGeneratedCamera
    BCC .scroll_ready

    LDA.l !BackgroundDeltaX
    JSR BG1SignExtendByte
    LDX.w #$0001
    JSR BG1AdvanceDataPosition
    CLC
    ADC.w $011A                ; Apply shake after unshaken camera movement.
    BRA .store_scroll

.scroll_ready
    LDA.b $0C

.store_scroll
    STA.b $E0                   ; Run hi-jacked instruction

    LDA.w $0120
    LSR A                       ; Convert the old scroll to its 8x8 tile column.
    LSR A
    LSR A
    STA.b $0A

    LDA.b $E0

    STA.w $0120                 ; Run hi-jacked instruction
    LSR A
    LSR A
    LSR A
    SEC
    SBC.b $0A                   ; Signed horizontal tile delta.
    STA.b $0E
    BEQ .save_cursor            ; Most frames expose no horizontal edge.

    LDA.b $18                   ; A bulk renderer may already own $1100.
    AND.w #$00FF
    BNE .save_cursor

    JSR BG1CheckStreamingEnabled
    BEQ .save_cursor

    LDA.b $0E
    CMP.w #$0001
    BEQ .edge_ready

    CMP.w #$FFFF
    BNE .save_cursor            ; Larger jumps are owned by bulk loading.

.edge_ready
    JSR BG1SelectRenderer
    JSR BG1CalculateLogicalWindowOrigin
    JSR BGStreamHorizontalEdge

.save_cursor
    ; A BG2 two-row update plus this column can pass offset $00FF. Preserve
    ; the complete cursor before returning to the caller's 8-bit index mode.
    STY.b $0C

    PLX
    PLA

    PLP
    RTL

BG1StreamVertical:
    PHP                         ; The caller has 16-bit A but 8-bit X/Y.
    REP #$30

    PHA
    PHX

    LDY.b $0C                   ; Restore the full cursor saved above.
    STA.b $08

    JSR BG1UseGeneratedCamera
    BCC .scroll_ready

    LDA.l !BackgroundDeltaY
    JSR BG1SignExtendByte
    LDX.w #$0003
    JSR BG1AdvanceDataPosition
    CLC
    ADC.w $011C                ; Apply shake after unshaken camera movement.
    BRA .store_scroll

.scroll_ready
    LDA.b $08

.store_scroll
    STA.b $E6                   ; Run hi-jacked instruction

    LDA.w $0124
    LSR A                       ; Convert the old scroll to its 8x8 tile row.
    LSR A
    LSR A
    STA.b $0A

    LDA.b $E6

    STA.w $0124                 ; Run hi-jacked instruction
    LSR A
    LSR A
    LSR A
    SEC
    SBC.b $0A                   ; Signed vertical tile delta.
    STA.b $0E
    BEQ .finish_list            ; Finish any earlier edge work.

    LDA.b $18                   ; Keep an earlier bulk list intact.
    AND.w #$00FF
    BNE .done

    JSR BG1CheckStreamingEnabled
    BEQ .finish_list

    ; BG1's observed gameplay movement exposes one row at a time.
    LDA.b $0E
    CMP.w #$0001
    BEQ .edge_ready

    CMP.w #$FFFF
    BNE .finish_list

.edge_ready
    JSR BG1SelectRenderer
    JSR BG1CalculateLogicalWindowOrigin
    JSR BGStreamVerticalEdge

.finish_list
    CPY.w #$0000                ; All four stages produced no work.
    BEQ .done

    JSR BGFinishList            ; Request the combined BG2/BG1 edge list.

.done
    LDA.w #$0000
    STA.l !BackgroundDeltaY      ; Consume both generated camera deltas.

    PLX
    PLA

    PLP
    RTL

; Return Z clear when a streamed overlay is active. Disabled overlays and
; rain return Z set without changing the caller's accumulator width.
BG1CheckStreamingEnabled:
    SEP #$20

    LDA.b $1D
    BEQ .done

    LDA.b $8C
    CMP.b #$9F

.done
    REP #$20
    RTS

;---------------------------------------------------------------------------------------------------
; Append one entering edge for the selected layer.
;
; Inputs:
;   $00/$02: logical window origin
;   $0E: signed tile delta
;   Y: current $1100 list cursor
;---------------------------------------------------------------------------------------------------

BGStreamHorizontalEdge:
    LDA.b $0E
    INC A                       ; -1 becomes 0; +1 becomes 2.
    ASL A
    ASL A
    ASL A
    ASL A                       ; Select left+0 or left+32.
    STA.b $04

    LDA.b $00
    CLC
    ADC.b $04
    AND.l !BGLogicalMask
    STA.b $06                   ; Logical X of the entering column.
    STA.b $0E                   ; Source and destination X are the same.

    JSR BGEmitColumn
    RTS

BGStreamVerticalEdge:
    LDA.b $0E
    BMI .moving_up

    ; Downward movement exposes row top+28.
    LDA.b $02
    CLC
    ADC.w #$001C
    AND.l !BGLogicalMask
    STA.b $02
    BRA .next_row

.moving_up
    EOR.w #$FFFF                ; Absolute value of -1 or -2.
    INC A
    STA.b $0E                   ; Start at top and emit one or two rows.

.next_row
    JSR BGEmitRow

    DEC.b $0E
    BEQ .done

    LDA.b $02                   ; The second exposed row follows the first.
    INC A
    AND.l !BGLogicalMask
    STA.b $02
    BRA .next_row

.done
    RTS

;---------------------------------------------------------------------------------------------------
; Convert the finalized BG2 scroll to the logical tile at the window's
; top-left corner.
;
; Output:
;   $00: logical X coordinate, 0..127
;   $02: logical Y coordinate, 0..127
;
; The flat-map loader places screen $8A at logical Map16 coordinate (0,0),
; regardless of that screen's world position. Subtract its world-space
; origin before dividing the scroll coordinates by eight. Large areas use a
; top-left screen ID and naturally continue into the other three quadrants.
;---------------------------------------------------------------------------------------------------

BG2CalculateLogicalWindowOrigin:
    LDA.b $8A                   ; Current top-left overworld screen ID.
    AND.w #$0007                ; Screen-grid column, 0..7.
    XBA                         ; Multiply by $100.
    ASL A                       ; Multiply by $200 pixels per screen.
    STA.b $0A                   ; World-space X origin of loaded screen.

    LDA.w $011E                 ; Final BG2 horizontal scroll in pixels.
    SEC
    SBC.b $0A                   ; Make it relative to logical map column 0.
    LSR A                       ; Divide by 8 pixels per 8x8 tile.
    LSR A
    LSR A
    AND.w #$007F                ; Wrap within the 128-tile logical width.
    STA.b $00

    ; Each overworld screen-grid row is $200 pixels tall. Bits 3-5 of $8A
    ; select that row; world and special-area bits do not affect its origin.
    LDA.b $8A
    AND.w #$0038                ; Screen-grid row already multiplied by 8.
    ASL A                       ; Multiply by another $40:
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A                       ; row * 8 * $40 = row * $200 pixels.
    STA.b $0A                   ; World-space Y origin of loaded screen.

    LDA.w $0122                 ; Final BG2 vertical scroll in pixels.
    SEC
    SBC.b $0A                   ; Make it relative to logical map row 0.
    LSR A                       ; Divide by 8 pixels per 8x8 tile.
    LSR A
    LSR A
    AND.w #$007F                ; Wrap within the 128-tile logical height.
    STA.b $02

    RTS

;---------------------------------------------------------------------------------------------------
; Append one 33-tile horizontal row.
;
; $00: first logical X coordinate
; $02: logical Y coordinate
; Y:   current $1100 list offset
;
; A 33-tile row always crosses exactly one boundary between the two physical
; 32x32 screen blocks, so this appends two horizontal entries.
;---------------------------------------------------------------------------------------------------

BGEmitRow:
    LDA.b $00                   ; Every row begins at the window's logical X.
    STA.b $06                   ; Initialize the tile X currently being emitted.

    ; If physical X within its 32-tile block is r, the second segment holds
    ; r+1 tiles and the first holds the remaining 33-(r+1).
    AND.w #$001F                ; r = X within the current screen block.
    INC A
    STA.b $0C                   ; Tiles in the second list entry.

    LDA.w #$0021                ; Entire row is 33 tiles.
    SEC
    SBC.b $0C
    STA.b $08                   ; Tiles through the first block boundary.

    JSR BGEmitHorizontalSegment

    LDA.b $0C                   ; Continue at the X advanced by the first entry.
    STA.b $08

    JSR BGEmitHorizontalSegment
    RTS

;---------------------------------------------------------------------------------------------------
; Emit one horizontal $18 entry.
;
; $06: first logical X coordinate
; $02: logical Y coordinate
; $08: number of tiles
; Y:   current $1100 list offset
;
; Advances $06 and Y past the emitted tiles.
;---------------------------------------------------------------------------------------------------

BGEmitHorizontalSegment:
    LDX.b $06                   ; Horizontal source and destination X match.
    JSR BGCalculateVRAMAddress
    STA.w $1100,Y               ; Entry bytes 0-1: VRAM word destination.

    INY                         ; Advance to the VMAIN/length fields.
    INY

    ; Low byte: VMAIN $80, advancing one VRAM word after each tile.
    ; High byte: transfer length in bytes.
    LDA.b $08                   ; Segment length in tile words.
    ASL A                       ; Convert words to DMA bytes.
    XBA                         ; Put byte count in entry byte 3.
    ORA.w #$0080                ; Put VMAIN $80 in entry byte 2.
    STA.w $1100,Y

    INY                         ; Advance to this entry's tile payload.
    INY

    ; Calculate the first Map16 byte offset once. Every pair advances to the
    ; next Map16 ID, avoiding a coordinate calculation per tile.
    LDA.b $02
    AND.w #$007E
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A                       ; Map16 row * $80 bytes.
    STA.b $0A

    LDA.b $06
    AND.w #$007E                ; Map16 column * 2 bytes.
    CLC
    ADC.b $0A
    CLC
    ADC.l !BGMap16SourceOffset
    TAX                         ; X is the current Map16 map offset.

    LDA.b $08
    STA.b $0A                   ; Preserve the segment length for final X advance.

    LDA.b $02                   ; Select top or bottom tables once per segment.
    AND.w #$0001
    BNE .bottom

    ; If the segment starts on an odd X, emit that initial top-right tile
    ; alone so every complete pair that follows starts on a left quadrant.
    LDA.b $06
    AND.w #$0001
    BEQ .top_pairs

    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16TopRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    INX
    INX                         ; Advance to the next Map16 ID.

    DEC.b $08
    BNE .top_pairs
    JMP .done

.top_pairs
    ; After an optional initial right tile, X is even. Save whether one
    ; trailing left tile remains, then count complete left/right pairs.
    LDA.b $08
    LSR A
    STA.b $08
    PHP

    LDA.b $08
    BEQ .top_pairs_done

.top_pair_loop
    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16TopLeft,X
    STA.w $1100,Y

    INY
    INY

    LDA.l !Map16TopRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    INX
    INX

    DEC.b $08
    BNE .top_pair_loop

.top_pairs_done
    PLP
    BCC .done                   ; Even remainder has no trailing tile.

    LDA.l $7E2000,X
    ASL A
    TAX

    LDA.l !Map16TopLeft,X
    STA.w $1100,Y

    INY
    INY
    BRA .done

.bottom
    ; Bottom rows follow the same right-then-pairs pattern using the bottom
    ; definition tables.
    LDA.b $06
    AND.w #$0001
    BEQ .bottom_pairs

    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16BottomRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    INX
    INX

    DEC.b $08
    BEQ .done

.bottom_pairs
    LDA.b $08
    LSR A
    STA.b $08
    PHP                         ; Carry records a trailing bottom-left tile.

    LDA.b $08
    BEQ .bottom_pairs_done

.bottom_pair_loop
    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16BottomLeft,X
    STA.w $1100,Y

    INY
    INY

    LDA.l !Map16BottomRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    INX
    INX

    DEC.b $08
    BNE .bottom_pair_loop

.bottom_pairs_done
    PLP
    BCC .done

    LDA.l $7E2000,X
    ASL A
    TAX

    LDA.l !Map16BottomLeft,X
    STA.w $1100,Y

    INY
    INY

.done
    LDA.b $06                   ; Advance logical X by the original length.
    CLC
    ADC.b $0A
    AND.l !BGLogicalMask
    STA.b $06
    RTS

;---------------------------------------------------------------------------------------------------
; Append one 29-tile vertical column.
;
; $06: logical X coordinate
; $02: first logical Y coordinate
; Y:   current $1100 list offset
;
; VMAIN $81 advances by one physical 32-tile row. If the column crosses
; physical row 31, split it so the second entry wraps to row 0 in the same
; horizontal screen block. Restores $02 before returning.
;---------------------------------------------------------------------------------------------------

BGEmitColumn:
    LDA.b $02
    PHA                         ; Preserve the window's logical top row.

    AND.w #$001F                ; Physical row of the column's first tile.
    STA.b $0A

    LDA.w #$0020
    SEC
    SBC.b $0A                   ; Tiles available through physical row 31.
    CMP.w #$001D
    BCC .save_first_length

    LDA.w #$001D                ; The whole column fits before the wrap.

.save_first_length
    STA.b $08

    LDA.w #$001D                ; Save any tiles remaining after the wrap.
    SEC
    SBC.b $08
    STA.b $0C

    JSR BGEmitVerticalSegment

    LDA.b $0C
    BEQ .done

    STA.b $08
    JSR BGEmitVerticalSegment

.done
    PLA
    STA.b $02                   ; Restore the logical top for row emission.
    RTS

;---------------------------------------------------------------------------------------------------
; Emit one vertical $18 entry.
;
; $06: logical source X coordinate
; $0E: physical destination X coordinate
; $02: first logical Y coordinate
; $08: number of tiles
; Y:   current $1100 list offset
;
; Advances $02 and Y past the emitted tiles.
;---------------------------------------------------------------------------------------------------

BGEmitVerticalSegment:
    LDX.b $0E                   ; Column destination may differ from source X.
    JSR BGCalculateVRAMAddress
    STA.w $1100,Y               ; Entry bytes 0-1: VRAM word destination.

    INY
    INY

    ; VMAIN $81 advances 32 VRAM words after each complete tile word.
    LDA.b $08
    ASL A                       ; Convert tile words to DMA bytes.
    XBA                         ; Put byte count in entry byte 3.
    ORA.w #$0081                ; Put VMAIN $81 in entry byte 2.
    STA.w $1100,Y

    INY
    INY                         ; Advance to this entry's tile payload.

    ; Calculate the first Map16 byte offset once. Every pair advances one
    ; Map16 row, avoiding a coordinate calculation and ID load per tile.
    LDA.b $02
    AND.w #$007E
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A
    ASL A                       ; Map16 row * $80 bytes.
    STA.b $0A

    LDA.b $06
    AND.w #$007E                ; Map16 column * 2 bytes.
    CLC
    ADC.b $0A
    CLC
    ADC.l !BGMap16SourceOffset
    TAX                         ; X is the current Map16 map offset.

    LDA.b $08
    STA.b $0A                   ; Preserve the segment length for final Y advance.

    LDA.b $06                   ; Select left or right tables once per segment.
    AND.w #$0001
    BNE .right

    ; If the segment starts on an odd Y, emit that initial bottom-left tile
    ; alone so every complete pair that follows starts on a top quadrant.
    LDA.b $02
    AND.w #$0001
    BEQ .left_pairs

    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16BottomLeft,X
    STA.w $1100,Y

    INY
    INY

    PLX
    TXA
    CLC
    ADC.w #$0080
    TAX                         ; Advance to the next Map16 row.

    DEC.b $08
    BNE .left_pairs
    JMP .done

.left_pairs
    ; Save whether one trailing top tile remains, then count complete
    ; top/bottom pairs.
    LDA.b $08
    LSR A
    STA.b $08
    PHP

    LDA.b $08
    BEQ .left_pairs_done

.left_pair_loop
    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16TopLeft,X
    STA.w $1100,Y

    INY
    INY

    LDA.l !Map16BottomLeft,X
    STA.w $1100,Y

    INY
    INY

    PLX
    TXA
    CLC
    ADC.w #$0080
    TAX

    DEC.b $08
    BNE .left_pair_loop

.left_pairs_done
    PLP
    BCC .done

    LDA.l $7E2000,X
    ASL A
    TAX

    LDA.l !Map16TopLeft,X
    STA.w $1100,Y

    INY
    INY
    BRA .done

.right
    ; Right columns follow the same bottom-then-pairs pattern using the right
    ; definition tables.
    LDA.b $02
    AND.w #$0001
    BEQ .right_pairs

    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16BottomRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    TXA
    CLC
    ADC.w #$0080
    TAX

    DEC.b $08
    BEQ .done

.right_pairs
    LDA.b $08
    LSR A
    STA.b $08
    PHP                         ; Carry records a trailing top-right tile.

    LDA.b $08
    BEQ .right_pairs_done

.right_pair_loop
    LDA.l $7E2000,X
    ASL A
    PHX
    TAX

    LDA.l !Map16TopRight,X
    STA.w $1100,Y

    INY
    INY

    LDA.l !Map16BottomRight,X
    STA.w $1100,Y

    INY
    INY

    PLX
    TXA
    CLC
    ADC.w #$0080
    TAX

    DEC.b $08
    BNE .right_pair_loop

.right_pairs_done
    PLP
    BCC .done

    LDA.l $7E2000,X
    ASL A
    TAX

    LDA.l !Map16TopRight,X
    STA.w $1100,Y

    INY
    INY

.done
    LDA.b $02                   ; Advance logical Y by the original length.
    CLC
    ADC.b $0A
    AND.l !BGLogicalMask
    STA.b $02
    RTS

;---------------------------------------------------------------------------------------------------
; Convert logical (X, $02) to a VRAM word in the selected 64x32 tilemap.
;
; The physical layout is two horizontal 32x32 screen blocks. The configured
; base selects BG2 at $0000 or, later, BG1 at $1000. Returns the address in A
; and clobbers $0A.
;---------------------------------------------------------------------------------------------------

BGCalculateVRAMAddress:
    LDA.b $02                   ; Logical Y is already wrapped for this layer.
    AND.w #$001F                ; Physical tilemap row wraps every 32 rows.
    ASL A                       ; Multiply the physical row by 32 words.
    ASL A
    ASL A
    ASL A
    ASL A
    STA.b $0A                   ; Row offset within one 32x32 screen block.

    TXA
    AND.w #$001F                ; Column within its 32-tile screen block.
    CLC
    ADC.b $0A
    STA.b $0A

    TXA
    AND.w #$0020                ; Logical X bit 5 selects left or right block.
    BEQ .done

    LDA.b $0A
    CLC
    ADC.w #$0400                ; Skip one 32x32 block (1024 VRAM words).
    STA.b $0A

.done
    LDA.b $0A
    CLC
    ADC.l !BGVRAMBase
    RTS

;---------------------------------------------------------------------------------------------------
; Prepare the source-world margins before MirrorWarp_Initialize changes $8A.
;---------------------------------------------------------------------------------------------------

hook_MirrorWarpBeforeWorldChange:
    STZ.w $420C                 ; HDMAEN: stop hardware before rewriting its table.
    JSL $80FDEE                 ; Run hi-jacked instruction
    JSR BGMirrorBuildMargins
    STZ.b $9B                   ; Do not combine the margin DMA with HDMA startup.
    RTL

hook_MirrorWarpWavingFrameAfterAnimation:
    ; Choose the next NMI's HDMA state before running an animation step.
    ; Several loading steps take most of a frame; setting $9B afterward lets
    ; NMI observe the temporary enabled value in the middle of that work.
    LDA.w $0200
    CMP.b #$0E
    BCS .enable_hdma

    ; Keep the wave active until filter mode 2 reaches zero. Once the palette
    ; is white, hold both hardware and software HDMA at vanilla's flat table
    ; until every loading step is complete.
    LDA.l $7EC009
    BNE .enable_hdma

    STZ.w $420C
    STZ.b $9B
    BRA .hdma_ready

.enable_hdma
    LDA.b #$C0
    STA.b $9B                   ; Default: enable mirror channels after NMI.

    LDA.w $0200
    CMP.b #$0E
    BCC .hdma_ready

    ; Consume the Module $15 marker once propagation resumes on screen.
    LDA.b $10
    CMP.b #$15
    BNE .restore_fixed_color

    LDA.l !Module15LayerEnable
    BPL .restore_fixed_color
    AND.b #$7F
    STA.l !Module15LayerEnable ; Consume the persistent Module $15 marker.

.restore_fixed_color
    ; Destination loading is complete. Restore its cached fixed color before
    ; the palette starts fading back from white.
    JSR RestoreMirrorFixedColor

.hdma_ready
    ; Step 1 starts replacing BG character graphics. Finish fading to white
    ; before it, then hold white through step 13 so every loading transfer and
    ; its HDMA-disabled frame remain hidden.
    LDA.w $0200
    BEQ .run_animation          ; Step 0 is only the normal startup delay.

    CMP.b #$0E
    BCS .run_animation          ; Loading is done; allow the fade to resume.

    LDA.l $7EC009
    BNE .advance_filter         ; Mode 2 has not reached full white yet.

    ; The palette is paused, so its alternating frame would otherwise do no
    ; work. Advance one loading step on every white frame.
    JSL $80D8A4                 ; AnimateMirrorWarp, without changing palette.

    LDA.b #$01
    STA.w $06BB                 ; Resume with a palette step after loading.
    LDA.w $0200
    CMP.b #$0E
    BCS .animation_done         ; Build the first wave on the last white frame.

.keep_filter_white
    REP #$20
    LDA.w #$001F                ; Hold at the completed full-white step.
    STA.l $7EC007
    SEP #$20

    JSR .prepare_nmi
    RTL                          ; Freeze vanilla's wave oscillator and table.

.advance_filter
    DEC.w $06BB                 ; Preserve vanilla's one-filter-step cadence.
    BNE .animation_done

    LDA.b #$02
    STA.w $06BB
    JSL $80EEEC                 ; PaletteFilter_BlindingWhite
    INC.b $15                   ; Its terminal $1F branch omits this request.
    LDA.l $7EC009
    BEQ .keep_filter_white      ; Do not advance the freshly flattened table.
    BRA .animation_done

.run_animation
    JSL $80EEE2                 ; Run hi-jacked instruction

    ; Freeze the wave on every full-white loading frame, including step 0's
    ; delay and the frame on which it installs the destination scroll.
    LDA.l $7EC009
    BNE .animation_done
    LDA.w $0200
    CMP.b #$0E
    BCS .animation_done
    BRA .keep_filter_white

.animation_done
    JSR .prepare_nmi
    JML $80FE68

.prepare_nmi
    LDA.l $7EC009
    BNE .fixed_color_ready

    LDA.w $0200
    BEQ .fixed_color_ready
    CMP.b #$0E
    BCS .fixed_color_ready

    ; The palette filter does not touch fixed color. Make its three PPU mirror
    ; components white too, so transparent BG pixels cannot expose area color.
    LDA.b #$3F
    STA.b $9C                   ; Red component $1F, plus the red-select bit.
    LDA.b #$5F
    STA.b $9D                   ; Green component $1F, plus green-select.
    LDA.b #$9F
    STA.b $9E                   ; Blue component $1F, plus blue-select.

    ; Preserve the terminal fade step's CGRAM request when it is the only NMI
    ; work. Once CGRAM is white, omit later copies only on heavy loading NMIs.
    LDA.b $17
    ORA.b $18
    BEQ .keep_hdma
    STZ.b $15
    BRA .disable_hdma

.fixed_color_ready
    LDA.b $17                   ; Specialized NMI transfer queued?
    ORA.b $18                   ; Arbitrary DMA list queued?
    BEQ .keep_hdma

.disable_hdma
    LDA.b #$01
    STA.w !NMISkipOAM          ; This mirror transfer may also omit OAM once.
    STZ.w $420C                 ; Stop the active channels before table writes.
    STZ.b $9B                   ; Keep them off through the upcoming NMI.

.keep_hdma
    RTS

; Preserve the fixed color selected from destination-area data before the
; remainder of MirrorWarp_LoadSpritesAndColors can overrun into another NMI.
; The displaced compare resumes the vanilla Castle/Pyramid layer setup.
CaptureMirrorWarpFixedColor:
    JSR SaveMirrorFixedColor

    LDA.b $10
    CMP.b #$15
    BNE .resume

    LDA.b #$3F
    STA.b $9C
    STA.w $2132
    LDA.b #$5F
    STA.b $9D
    STA.w $2132
    LDA.b #$9F
    STA.b $9E
    STA.w $2132

.resume
    LDA.b $8A                   ; Run displaced instructions.
    CMP.b #$1B
    JML $82B2EF

; Cache the destination area's three $2132 bytes as one BGR555 word. Repacking
; them lets the persistent mirror state fit in the two free bytes at $7EC90E.
SaveMirrorFixedColor:
    PHP
    REP #$20
    LDA.b $9C                   ; Load red in the low byte, green in the high.
    PHA
    AND.w #$001F                ; Packed bits 0-4: red.
    STA.l !MirrorFixedColor
    PLA
    AND.w #$1F00
    LSR A
    LSR A
    LSR A
    ORA.l !MirrorFixedColor     ; Packed bits 5-9: green.
    STA.l !MirrorFixedColor

    SEP #$20
    LDA.b $9E                   ; Strip the blue-select bit from the third byte.
    AND.b #$1F
    REP #$20
    AND.w #$00FF
    XBA
    ASL A
    ASL A
    ORA.l !MirrorFixedColor     ; Packed bits 10-14: blue.
    STA.l !MirrorFixedColor
    PLP
    RTS

; Restore the destination fixed color after the white loading cover. Recreate
; the component-select bits required by one-byte writes to PPU register $2132.
RestoreMirrorFixedColor:
    PHP
    REP #$20
    LDA.l !MirrorFixedColor
    PHA
    AND.w #$001F                ; Unpack red from bits 0-4.
    ORA.w #$0020                ; Select red for the eventual $2132 write.
    SEP #$20
    STA.b $9C

    REP #$20
    PLA
    PHA
    LSR A
    LSR A
    LSR A
    LSR A
    LSR A
    AND.w #$001F                ; Unpack green from bits 5-9.
    ORA.w #$0040                ; Select green for the eventual $2132 write.
    SEP #$20
    STA.b $9D

    REP #$20
    PLA
    XBA
    LSR A
    LSR A
    AND.w #$001F                ; Unpack blue from bits 10-14.
    ORA.w #$0080                ; Select blue for the eventual $2132 write.
    SEP #$20
    STA.b $9E
    PLP
    RTS

; Step 10 retains its animated-tile work, then repairs both layers' margins.
hook_MirrorWarpMarginPhase:
    JSL $80D915                 ; Run hi-jacked instruction

    LDA.b $10
    CMP.b #$15
    BNE .build

    LDA.l !Module15LayerEnable+1
    STA.b $1D

.build
    JSR BGMirrorBuildMargins

    LDA.b $10
    CMP.b #$15
    BNE .return

    LDA.b $1D
    AND.b #$F8
    STA.b $1D

.return
    RTL

;---------------------------------------------------------------------------------------------------
; Build one mirror-margin $1100/$18 list.
;
; The HDMA oscillator reaches -9..+9 pixels. Across every possible subpixel
; camera position, the normal left..left+32 window therefore needs columns
; left-2, left-1, and left+33. BG2 clamps source data at world edges and also
; repairs an out-of-bounds left+32 column. BG1 wraps its logical 64x64 overlay.
;---------------------------------------------------------------------------------------------------

BGMirrorBuildMargins:
    PHP
    REP #$30

    PHX
    PHY

    JSR BG2SelectRenderer
    JSR BG2CalculateLogicalWindowOrigin

    LDY.w #$0000

    ; VRAM is a ring, so the two destinations wrap at X=0. Logical map data
    ; does not wrap there: repeat source column 0 outside the world edge.
    LDA.b $00
    SEC
    SBC.w #$0002
    AND.w #$007F
    STA.b $0E

    LDA.b $00
    CMP.w #$0002
    BCS .left_two_inside
    LDA.w #$0000
    BRA .left_two_ready

.left_two_inside
    SEC
    SBC.w #$0002

.left_two_ready
    STA.b $06
    JSR BGEmitColumn

    LDA.b $00
    DEC A
    AND.w #$007F
    STA.b $0E

    LDA.b $00
    BEQ .left_one_ready
    DEC A

.left_one_ready
    STA.b $06
    JSR BGEmitColumn

    ; Clamp right-side sources to the last logical column: 63 for a small
    ; area, 127 for a large area.
    LDA.w $0712
    AND.w #$00FF
    BEQ .small_area
    LDA.w #$007F
    BRA .have_right_edge

.small_area
    LDA.w #$003F

.have_right_edge
    STA.b $04

    ; The normal 33-tile window already wrote left+32. At the far-right area
    ; boundary that destination wrapped around the VRAM ring, but its source
    ; was outside the loaded Map16 map. Replace it with the edge column.
    LDA.b $00
    CLC
    ADC.w #$0020
    CMP.b $04
    BCC .right_margin
    BEQ .right_margin

    AND.w #$007F
    STA.b $0E

    LDA.b $04
    STA.b $06
    JSR BGEmitColumn

.right_margin
    ; left+33 is the additional column exposed by the mirror wave.
    LDA.b $00
    CLC
    ADC.w #$0021
    AND.w #$007F
    STA.b $0E

    LDA.b $00
    CLC
    ADC.w #$0021
    CMP.b $04
    BCC .right_ready
    BEQ .right_ready
    LDA.b $04

.right_ready
    STA.b $06
    JSR BGEmitColumn
    JMP BG1BuildMirrorMargins

BG1EmitMirrorColumn:
    STA.b $06
    STA.b $0E
    JSR BGEmitColumn
    RTS

BG1BuildMirrorMargins:
    JSR BG1CheckStreamingEnabled
    BEQ .finish

    JSR BG1SelectRenderer
    JSR BG1CalculateLogicalWindowOrigin

    ; BG1's logical overlay wraps with its tilemap, so source and destination
    ; use the same wrapped coordinate for all three exposed columns.
    LDA.b $00
    DEC A
    DEC A
    AND.w #$003F
    JSR BG1EmitMirrorColumn

    LDA.b $00
    DEC A
    AND.w #$003F
    JSR BG1EmitMirrorColumn

    LDA.b $00
    CLC
    ADC.w #$0021
    AND.w #$003F
    JSR BG1EmitMirrorColumn

.finish
    JSR BGFinishList

    PLY
    PLX

    PLP
    RTS

assert pc() <= !free_space_bank_a0_end
