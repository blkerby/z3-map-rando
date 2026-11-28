# Rain music

Outdoor music uses vanilla song `$03` (Rain) while `!EventFlags & $80` is clear, with bunny music taking priority. A dungeon boss defeat, completion of Zelda's Sanctuary sequence, opening Mire, or the flute kid starting to fade sets that saved bit. Music follows the global storm state even where an authored BG1 prevents visible rain. Cleared weather uses normal area-specific outdoor music.

Link's House spawns, including a new game, select the opening track and its indoor rain ambience only while the storm is active. Other spawn points retain their own music. Normal indoor entry keeps its existing music choices; the indoor rain-sound handler applies its floor and Sanctuary audibility rules when the requested track is Rain and the storm is active.

## Runtime

`patches/src/overworld_music.asm` resolves the pending song in one shared foreground routine. The main loop calls it after game and OAM work, immediately before clearing `$12` and returning to its NMI wait. The iris renderer calls the same routine before rebuilding its table: its scanline wait can cross vblank before control returns to the main loop. NMI remains unchanged.

The routine recognizes normal regional tracks while the overworld sound bank is loaded and the game is in an outdoor loading, play, travel, or interface state. It checks Link's bunny form before applying weather rules, so transition recovery requests also select bunny music while that form is active. It preserves indoor music, special event tracks, volume commands, and music owned by entrance cutscenes. An indoor exit can retain Rain while returning outdoors. House spawn selection and indoor rain ambience keep their own hooks because the indoor music bank and ambience must be selected before playback.

The SPC restarts a track when it receives that track's ID again. When Rain is already playing, the shared routine substitutes the maximum-volume command for another Rain request and consumes ordinary area-transition fades. Map and menu volume commands stay intact. Mirror travel retains its warp track and restores the appropriate regional track at the destination.

The generated Mire-opening cutscene replaces the Rain track with normal Dark World music when it clears the storm, retaining the bunny track without Moon Pearl. The flute kid's sequence clears the rain overlay as the sprite fade starts and fades out the Rain track halfway through. Normal Light World music starts when the fade finishes. Both sequences preserve an existing non-rain track. Boss defeat and Zelda's escape use the next outdoor load to select normal music. `EndRain` only saves weather state; presentation code owns the song request.
