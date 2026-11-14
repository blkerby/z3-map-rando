; Interface shared by independently assembled patches.

; Define an ASM symbol and expose its numeric value to the Rust catalog builder.
macro export_symbol(internal_name, exported_name, value)
    !<internal_name> = <value>
    export_<exported_name> = <value>
endmacro

; ALTTPR-compatible saved ownership, separate from the selected vanilla items.
; $80 blue boomerang, $40 red boomerang, $20 current mushroom, $10 powder,
; $08 past mushroom, $04 shovel, $02 inactive ocarina, $01 active ocarina.
!InventoryTracking = $7EF38C

; World of the loaded overworld sprite set, retained until its replacement.
!OverworldSpriteWorld = $7FF18B

; Items for the vanilla thrown-gift sprites, whose subtype remains unchanged.
%export_symbol(CatfishItem, catfish_item, $B08040)
%export_symbol(KingZoraItem, king_zora_item, $B08041)

; Generated flat Map16 maps and their screen, alternate-background, and rain
; pointers. The patcher allocates map payloads in banks C0-C9.
%export_symbol(ThemeFlatMap16Data, theme_flat_map16_data, $C08000)
%export_symbol(FlatMap16ScreenPointers, flat_map16_screen_pointers, $BFE000)
%export_symbol(BG1FlatMap16ScreenPointers, bg1_flat_map16_screen_pointers, $BFE200)
%export_symbol(Area80BG1FlatMap16Pointers, area_80_bg1_flat_map16_pointers, $BFE400)
%export_symbol(GeneratedBG1Enabled, generated_bg1_enabled, $BFE406)
%export_symbol(LostWoodsClearBG1FlatMap16Pointers, lost_woods_clear_bg1_flat_map16_pointers, $BFE410)
%export_symbol(RainContexts, rain_contexts, $BFE430)
%export_symbol(RainFlatMap16Pointers, rain_flat_map16_pointers, $BFE4D0)
%export_symbol(SlotToArea, slot_to_area, $82A4E3)
%export_symbol(AreaToAnchor, area_to_anchor, $BFE540)
%export_symbol(WorldMapGraphics, world_map_graphics, $CA8000)
%export_symbol(WorldMapTilemaps, world_map_tilemaps, $CB8000)
%export_symbol(WorldMapPalettes, world_map_palettes, $8ADB39)

; A/X are 16-bit; preserve X and resolve a normal logical ID to its physical anchor.
!GetPhysicalAreaAnchor = $A0F200

; Generated background colors and asset payloads. Asset records and descriptor
; lists are allocated by the patcher in banks D0-FF.
%export_symbol(ThemeBackgroundColors, theme_background_colors, $A09E00)
%export_symbol(OverworldAreaRecordVariantPointers, overworld_area_record_variant_pointers, $A78000)
%export_symbol(OverworldAssetData, overworld_asset_data, $D08000)

; Four 16-bit 8x8 tilemap words make up each Map16 definition. Each table
; stores one quadrant, indexed by twice the Map16 ID.
%export_symbol(Map16TopLeft, map16_top_left, $A18000)
%export_symbol(Map16TopRight, map16_top_right, $A28000)
%export_symbol(Map16BottomLeft, map16_bottom_left, $A38000)
%export_symbol(Map16BottomRight, map16_bottom_right, $A48000)

; One property byte per Map16 ID and quadrant. The independent-property patch
; populates these tables from the generated Map16 definitions.
%export_symbol(Map16PropertyTopLeft, map16_property_top_left, $A58000)
%export_symbol(Map16PropertyTopRight, map16_property_top_right, $A5C000)
%export_symbol(Map16PropertyBottomLeft, map16_property_bottom_left, $A68000)
%export_symbol(Map16PropertyBottomRight, map16_property_bottom_right, $A6C000)

; Selects the property for the Map16 ID in A using the contacted quadrant from
; $00/$02. Returns the property in A with 8-bit accumulator and index registers.
; Defined in overworld_map16_properties.asm.
!GetOverworldTileTypeIndependent = $80884E

; Twenty-two little-endian 24-bit pointers to generated dynamic-tile groups,
; indexed by the stable replacement-type order in the patcher's compiler.
%export_symbol(DynamicTileGroupPointers, dynamic_tile_group_pointers, $A78300)

; Five little-endian 24-bit pointers to generated dungeon-entrance cutscene
; scripts, indexed by the vanilla trigger value minus one.
%export_symbol(CutscenePointers, cutscene_pointers, $A78342)

; One little-endian 24-bit persistent event-overlay pointer per overworld area.
; A zero pointer means the area has no generated overlay.
%export_symbol(OverworldOverlayPointers, overworld_overlay_pointers, $A78351)

; Opens a generated wooden door at the contacted Map16 offset in X.
; Carry set means a door was opened and entrance handling should stop.
; Defined in overworld_dynamic_tiles.asm.
!OpenDynamicWoodenDoorAtEntrance = $A0AF00

; A nonzero value asks NMI to omit its next OAM upload. NMI clears the request
; after consuming it, so producers do not need to clear it themselves.
!NMISkipOAM = $0702

; The active HUD tilemap destination also identifies the overworld VRAM layout.
!OverworldHUD = $3C40

; Saved copies of the $1C/$1D main-screen and subscreen layer-enable mirrors.
; Module $15 uses them to restore the layers after temporarily hiding BG1/BG2.
!Module15LayerEnable = $7EC90A

; A nonzero value suspends ordinary animation scheduling while a transition
; owns the generated-asset queue.
!AnimationSuspended = $7EC90D

; A nonzero value marks generated overworld graphics and palette loading.
!GeneratedAssetMarker = $7EC909

; Selects the overworld area record for the 8-bit key in A and current game
; state. Requires 16-bit X/Y and returns its 24-bit pointer in $00-$02.
; Defined in overworld_assets.asm.
!ResolveOverworldAreaRecord = $A0A6C6

; Activates the animation list in the selected area record at $00-$02.
; Defined in overworld_animations.asm.
!ActivateOverworldAnimationTracks = $A09F40

; Selects the overworld VRAM layout and clears its tilemaps during forced blank.
; Defined in overworld_vram.asm.
!SelectAndClearOverworldVRAM = $A0AC53

; Rebuilds displayed OBJ palettes 0-3 for the current rain/lightning state.
; Defined in overworld_bg_tilemaps.asm.
!RebuildRainSpritePalettes = $A08900
!UpdateRainSpritePalettes = $A0890A

; Cached rain treatment for displayed OBJ palettes 0-3: 0 is bright; 1 and 2
; use light and dark rain; 3 and 4 also keep palette 1's lower half bright.
!RainSpritePaletteState = $7ECC82

; Pending cutscene and live-overlay BG2 rectangle in logical 8x8 coordinates.
; All fields are words; maxima identify the last changed Map16 cell's top-left tile.
!CutsceneRedrawPending = $7ECC83
!CutsceneRedrawMinX = $7ECC85
!CutsceneRedrawMaxX = $7ECC87
!CutsceneRedrawMinY = $7ECC89
!CutsceneRedrawMaxY = $7ECC8B

; Perpendicular target and remaining alignment delta, including the final snap.
!TransitionCameraTarget = $7ECC8D
!TransitionCameraSnap = $7ECC8F
!BackgroundDeltaY = $7ECC7B
!BackgroundDeltaX = $7ECC7C

; Preload the source BG2 strips for TransitionCameraSnap. X is the camera-axis
; offset (0 for X, 6 for Y); A/X/Y are 16-bit, X/Y are preserved.
!PreloadAlignedTransitionEdge = $A09800

; Raises bomb ($51 in Y) or arrow ($53 in Y) capacity and queues the same
; amount of ammo. Entry is A8/X8/Y8; preserves X/Y/DB.
; Defined in capacity_upgrades.asm.
!UpgradeCapacity = $A0FB00

; Starting bomb and arrow capacities, written by the patcher from the settings.
%export_symbol(StartingCapacities, starting_capacities, $A0FAFE)

; Saved spawn unlocks: bit 0 is Sanctuary entered; bit 1 is Light World mountain reached.
!SpawnVisitFlags = $7EF3D6

; Unified receipt tables, including room for all 256 receipt IDs.
%export_symbol(ItemReceiptTables, item_receipt_tables, $B1A000)
!ItemReceiptValues = !ItemReceiptTables+$700
%export_symbol(ItemGraphics, item_graphics, $B1AA00)
