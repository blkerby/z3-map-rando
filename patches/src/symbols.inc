; Interface shared by independently assembled patches.

; Define an ASM symbol and expose its numeric value to the Rust catalog builder.
macro export_symbol(internal_name, exported_name, value)
    !<internal_name> = <value>
    export_<exported_name> = <value>
endmacro

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
; indexed by the stable replacement-type order in theme_check.
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
