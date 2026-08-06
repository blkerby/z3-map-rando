# Overworld cutscenes

Overworld cutscenes will be stored as JSON scripts created by the overworld
editor. Rust will compile the scripts into ROM data, and a small ASM interpreter
will execute them during gameplay.

The initial scope is dungeon-entrance-style cutscenes: timed changes to the
overworld, accompanied by sounds, shaking, and palette changes. Scripts do not
need to reproduce every display effect used by vanilla.

## Script model

A script is a sequence of actions. The interpreter executes actions until one
waits, then resumes from that point on a later frame. Effects such as screen
shaking remain active across waits until stopped.

Tile changes are stored as named frames. A frame contains one or more drawings
positioned in area-local Map16 coordinates, rather than WRAM offsets or VRAM
addresses. Each drawing uses the same parallel `palettes`, `tiles`, and `flips`
grids as the existing ALTTPRetiling area JSON. The grids describe 8x8 tiles, so
their width and height are even and each 2x2 group becomes one Map16 tile.

The compiler derives the persistent result by applying every `draw` action in
script order, with later writes replacing earlier writes at the same position.
That accumulated result is used when rebuilding an area whose event has already
been completed.

For example:

```json
{
  "event": "palace_of_darkness_entrance_opened",
  "frames": {
    "cracking": [
      {
        "position": [ 18, 12 ],
        "palettes": [ [ 5, 5 ], [ 5, 5 ] ],
        "tiles": [ [ 320, 321 ], [ 322, 323 ] ],
        "flips": [ [ 0, 0 ], [ 0, 0 ] ]
      }
    ],
    "open": [
      {
        "position": [ 18, 12 ],
        "palettes": [ [ 5, 5 ], [ 5, 5 ] ],
        "tiles": [ [ 324, 325 ], [ 326, 327 ] ],
        "flips": [ [ 0, 0 ], [ 0, 0 ] ]
      }
    ]
  },
  "actions": [
    { "action": "wait", "frames": 64 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "draw", "frame": "cracking" },
    { "action": "wait", "frames": 48 },
    { "action": "set_complete" },
    { "action": "draw", "frame": "open" },
    { "action": "play_sound", "channel": 3, "sound": 27 },
    { "action": "end" }
  ]
}
```

The palette and tile numbers above are illustrative. Tile numbers are indices
within the selected palette, as they are in an ALTTPRetiling area file.

## Actions

| Action | Purpose |
| --- | --- |
| `wait(frames)` | Pause the script for a number of frames. |
| `play_sound(channel, sound)` | Play a sound through one of the three vanilla sound-effect channels. |
| `play_music(song)` | Change the current music. |
| `draw(frame)` | Apply a named list of Map16 changes and update the visible tilemap. |
| `set_complete` | Mark the script's completion event complete at the required point in the sequence. |
| `start_shake(offsets)` | Begin a repeating pattern of horizontal and vertical screen offsets. |
| `stop_shake` | Stop shaking and clear the screen offsets. |
| `set_palette(changes)` | Immediately replace selected displayed palette colors. |
| `end` | End the cutscene, stop its effects, and restore normal gameplay processing. |

Here is an independent example of every action:

```json
[
  { "action": "wait", "frames": 48 },
  { "action": "play_sound", "channel": 2, "sound": 12 },
  { "action": "play_music", "song": 13 },
  { "action": "draw", "frame": "cracking" },
  { "action": "set_complete" },
  {
    "action": "start_shake",
    "offsets": [ [ 1, -1 ], [ -1, 1 ] ]
  },
  { "action": "stop_shake" },
  {
    "action": "set_palette",
    "changes": [
      {
        "palette": 5,
        "indices": [ 1, 2 ],
        "colors": [ [ 31, 31, 31 ], [ 0, 0, 0 ] ]
      }
    ]
  },
  { "action": "end" }
]
```

Shake offsets are `[x, y]` pixel displacements. The interpreter advances to the
next offset each frame and repeats the sequence; an offset can be repeated in
the list to hold it for multiple frames. Palette numbers are ALTTPRetiling
palette IDs, and color components range from 0 to 31. Each entry in `indices`
corresponds to the color at the same position in `colors`.

Fades and other gradual palette effects are represented as a sequence of
`set_palette` and `wait` actions. They do not require a separate transition
mechanism in the interpreter.

## Engine behavior

Starting a cutscene suspends normal player and sprite processing. Ending one
restores that processing and clears any active shake. Drawing and palette
actions automatically request the necessary NMI uploads; scripts do not manage
VRAM, CGRAM, or transfer queues directly.

Cutscene triggers are separate from scripts. An area event selects and starts a
script, while the script describes only what happens after it begins.

The script format will not initially include branches, loops, arbitrary ASM
calls, or arbitrary memory writes. Repeated script data may be compressed by
the compiler without exposing control flow in the editor.

## Vanilla adaptation

The Palace of Darkness and Skull Woods sequences can be represented directly
with waits, sounds, completion state, and Map16 frames. Misery Mire will keep
its terrain changes, sounds, and optional shaking, but omit the distracting
black-and-white BG1 flashing.

Turtle Rock and Ganon's Tower will retain the parts expressible through Map16,
sound, shaking, and palette actions. Their vanilla BG1, subscreen, and color
math effects are not compatibility requirements and may be redesigned by a
theme.

BG1 drawing and configuration are deliberately outside the script system.
Camera synchronization is also omitted because it only supported the vanilla
Turtle Rock BG1 construction.
