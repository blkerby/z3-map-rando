; Interface shared by independently assembled patches.

; Four 16-bit 8x8 tilemap words make up each Map16 definition. Each table
; stores one quadrant, indexed by twice the Map16 ID.
!Map16TopLeft = $A18000
!Map16TopRight = $A28000
!Map16BottomLeft = $A38000
!Map16BottomRight = $A48000

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
!SelectAndClearOverworldVRAM = $A0AC4B
