; Keep normal rain color math during lightning flashes.
lorom

; OverworldOverlay_HandleRain: half color math, subtraction on CG0, OBJ, BG2.
org $82A3FD
hook_set_rain_lightning_color_math:
    LDA.b #$72
