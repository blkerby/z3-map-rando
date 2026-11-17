; Suppress rain lightning and Ether medallion palette flashes.
lorom

; OverworldOverlay_HandleRain: half color math, subtraction on CG0, OBJ, BG2.
org $82A3FD
hook_set_rain_lightning_color_math:
    LDA.b #$72

; Ancilla18_EtherSpell: skip its palette flashing block.
org $88AACE
hook_update_ether_palettes:
    BRA return_ether_animation

org $88AAFE
return_ether_animation:
