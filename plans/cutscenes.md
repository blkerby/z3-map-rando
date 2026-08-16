# Overworld cutscenes

Overworld cutscenes will be stored as JSON scripts created by the overworld
editor. Rust will compile the scripts into ROM data, and a small ASM interpreter
will execute them during gameplay.

The initial scope is dungeon-entrance-style cutscenes: timed changes to area
layers, accompanied by sounds, shaking, music, and palette changes. Scripts do
not need to reproduce unwanted vanilla display effects such as black-and-white
flashing.

## Script model

A script belongs to one area and contains a sequence of actions. Each area's
theme directory stores its scripts in `cutscenes.json`, allowing more scripts
to be added to the same file later. A `draw` action references a layer in that
area directly. The layer's existing BG1 or BG2 selection determines which
background it changes.

```json
{
  "cutscenes": [
    {
      "event": "example_complete",
      "actions": [
        { "action": "wait", "frames": 64 },
        { "action": "set_complete" },
        { "action": "play_sound", "channel": 2, "sound": 12 },
        { "action": "play_sound", "channel": 3, "sound": 7 },
        { "action": "draw", "layer": "Cutscene cracking" },
        { "action": "wait", "frames": 96 },
        { "action": "draw", "layer": "Cutscene open" },
        { "action": "play_sound", "channel": 3, "sound": 27 },
        { "action": "end" }
      ]
    }
  ]
}
```

Cutscene layers are sparse incremental patches. Drawing a layer adds its tiles
to the displayed cutscene state; cells absent from the layer are left unchanged,
and later writes replace earlier writes to the same cells. The compiler applies
every drawn layer in script order to derive the persistent appearance used when
loading an area whose event is complete. Keeping each layer to only its new tile
writes also limits the work requested from NMI.

Any area layer referenced by a cutscene `draw` action is excluded from the
area's ordinary initial rendering. The editor can therefore use its existing
layer tools to edit cutscene artwork without adding another tile format.

The interpreter executes actions until one waits, then resumes from that point
on a later frame. Effects such as screen shaking remain active across waits
until stopped.

## Actions

| Action | Purpose |
| --- | --- |
| `wait(frames)` | Pause the script for a number of frames. |
| `play_sound(channel, sound)` | Play a sound through one of the three vanilla sound-effect channels. |
| `play_music(song)` | Change the current music. |
| `draw(layer)` | Accumulate the tile writes in a named area layer. |
| `set_complete` | Mark the script's completion event complete at the required point in the sequence. |
| `start_shake(offsets)` | Begin a repeating pattern of horizontal and vertical screen offsets. |
| `stop_shake` | Stop shaking and clear the screen offsets. |
| `end` | End the cutscene, stop its effects, and restore normal gameplay processing. |

Shake offsets are `[x, y]` pixel displacements. The interpreter advances to the
next offset each frame and repeats the sequence.

## Vanilla scripts

These scripts transcribe the waits, terrain phases, sounds, music, and shaking
in [`EntranceCutscene`](../jpdasm/bank_1B.asm#L14694). Layer names describe what
the vanilla importer should create. Decimal sound and song numbers correspond
to the hexadecimal IDs in the disassembly.

This model is adequate for the terrain, BG2 artwork, timing, sound, music,
shaking, palette, and persistence used by the five sequences. Ganon's Tower's
orbiting crystals remain intact as a vanilla sprite sequence before the script
starts, so they do not require sprite control in the cutscene format.

The scripts intentionally omit Misery Mire's intermittent BG1 flashing and
Ganon's Tower's black-and-white flash. Turtle Rock replaces its BG1/BG2
cross-fade with a simple, faster fade to and from black.

### Palace of Darkness

Vanilla waits 64 frames before the first change and 32 frames between each
later phase. Each change plays SFX2 `$0C` and SFX3 `$07`; the sequence ends
with SFX3 `$1B`.

```json
{
  "cutscenes": [
  {
  "event": "palace_of_darkness_entrance_opened",
  "actions": [
    { "action": "wait", "frames": 64 },
    { "action": "set_complete" },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 1" },
    { "action": "wait", "frames": 32 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 2" },
    { "action": "wait", "frames": 32 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 3" },
    { "action": "wait", "frames": 32 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene open" },
    { "action": "wait", "frames": 32 },
    { "action": "play_sound", "channel": 3, "sound": 27 },
    { "action": "end" }
  ]
  }
  ]
}
```

### Skull Woods

The first fire change occurs after 4 frames. Four more phases follow at
12-frame intervals. Every phase plays SFX3 `$16`, and the last also leads into
SFX3 `$1B`.

```json
{
  "cutscenes": [
  {
  "event": "skull_woods_entrance_opened",
  "actions": [
    { "action": "wait", "frames": 4 },
    { "action": "set_complete" },
    { "action": "draw", "layer": "Cutscene phase 1" },
    { "action": "play_sound", "channel": 3, "sound": 22 },
    { "action": "wait", "frames": 12 },
    { "action": "draw", "layer": "Cutscene phase 2" },
    { "action": "play_sound", "channel": 3, "sound": 22 },
    { "action": "wait", "frames": 12 },
    { "action": "draw", "layer": "Cutscene phase 3" },
    { "action": "play_sound", "channel": 3, "sound": 22 },
    { "action": "wait", "frames": 12 },
    { "action": "draw", "layer": "Cutscene phase 4" },
    { "action": "play_sound", "channel": 3, "sound": 22 },
    { "action": "wait", "frames": 12 },
    { "action": "draw", "layer": "Cutscene open" },
    { "action": "play_sound", "channel": 3, "sound": 22 },
    { "action": "play_sound", "channel": 3, "sound": 27 },
    { "action": "end" }
  ]
  }
  ]
}
```

### Misery Mire

Vanilla spends 239 frames on its initial intermittent BG1 effect. The effect is
omitted here. Rumbling begins 16 frames later; shaking uses the vanilla
alternating offsets. The terrain phases occur after a further 56, 72, and 80
frames, followed by a final 128-frame wait.

```json
{
  "cutscenes": [
  {
  "event": "misery_mire_entrance_opened",
  "actions": [
    { "action": "wait", "frames": 16 },
    { "action": "play_sound", "channel": 1, "sound": 7 },
    { "action": "start_shake", "offsets": [ [ -1, 1 ], [ 1, -1 ] ] },
    { "action": "wait", "frames": 56 },
    { "action": "set_complete" },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 1" },
    { "action": "wait", "frames": 72 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 2" },
    { "action": "wait", "frames": 80 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene open" },
    { "action": "wait", "frames": 128 },
    { "action": "play_sound", "channel": 1, "sound": 5 },
    { "action": "play_sound", "channel": 3, "sound": 27 },
    { "action": "stop_shake" },
    { "action": "end" }
  ]
  }
  ]
}
```

### Turtle Rock

This simplified sequence skips the vanilla palette fade. It shakes for 16 frames,
reveals the final BG2 terrain at once, then shakes for 16 more frames. SFX3 `$02`
plays before and after the reveal; the sequence ends with SFX1 `$05` and SFX3
`$1B`.

```json
{
  "cutscenes": [
    {
      "event": "turtle_rock_entrance_opened",
      "actions": [
        { "action": "set_complete" },
        { "action": "start_shake", "offsets": [ [ -1, 1 ], [ 1, -1 ] ] },
        { "action": "play_sound", "channel": 3, "sound": 2 },
        { "action": "wait", "frames": 16 },
        { "action": "draw", "layer": "Cutscene open" },
        { "action": "play_sound", "channel": 3, "sound": 2 },
        { "action": "wait", "frames": 16 },
        { "action": "play_sound", "channel": 1, "sound": 5 },
        { "action": "play_sound", "channel": 3, "sound": 27 },
        { "action": "stop_shake" },
        { "action": "end" }
      ]
    }
  ]
}
```

### Ganon's Tower

This begins after the vanilla orbiting-crystal sequence, which remains part of
the trigger-side behavior. Only its following black-and-white flash is omitted.
Rumbling starts immediately. The nine terrain phases use vanilla waits of 48,
48, 52, then six lots of 32 frames. Every phase plays SFX2 `$0C` and SFX3
`$07`. After 72 more frames, vanilla plays SFX3 `$1B`, changes to song `$0D`,
and starts SFX1 `$09` wind.

```json
{
  "cutscenes": [
  {
  "event": "ganons_tower_entrance_opened",
  "actions": [
    { "action": "set_complete" },
    { "action": "play_sound", "channel": 1, "sound": 7 },
    { "action": "wait", "frames": 48 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 1" },
    { "action": "wait", "frames": 48 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 2" },
    { "action": "wait", "frames": 52 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 3" },
    { "action": "wait", "frames": 32 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 4" },
    { "action": "wait", "frames": 32 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 5" },
    { "action": "wait", "frames": 32 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 6" },
    { "action": "wait", "frames": 32 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 7" },
    { "action": "wait", "frames": 32 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene phase 8" },
    { "action": "wait", "frames": 32 },
    { "action": "play_sound", "channel": 1, "sound": 5 },
    { "action": "play_sound", "channel": 2, "sound": 12 },
    { "action": "play_sound", "channel": 3, "sound": 7 },
    { "action": "draw", "layer": "Cutscene open" },
    { "action": "wait", "frames": 72 },
    { "action": "play_sound", "channel": 3, "sound": 27 },
    { "action": "play_music", "song": 13 },
    { "action": "play_sound", "channel": 1, "sound": 9 },
    { "action": "end" }
  ]
  }
  ]
}
```

## Vanilla import

The editor's ROM importer should create these scripts only for the five vanilla
parent areas (`$5E`, `$40`, `$70`, `$47`, and `$43`). No general 65816 script
decoder is needed.

For each terrain phase, the importer should convert only that phase's known
vanilla Map16 writes through the same Map16-to-8x8 path used by the ordinary
area import and save them as a sparse cutscene layer. The layers accumulate in
script order, including later replacements at positions written by earlier
phases. Their accumulated result is the persistent opened state. Turtle Rock
imports only its final BG2 terrain writes; its temporary vanilla BG1 data is
not needed.

The importer should then write the corresponding script template above to the
area theme's `cutscenes.json`, with `draw` actions referencing those layer
names directly. This deliberately uses five small, known tables in the importer
instead of trying to infer arbitrary cutscene code from a ROM. Custom ROM
cutscene code is outside the vanilla import contract.

## Engine behavior

Starting a cutscene suspends normal player and sprite processing. Ending one
restores that processing and clears any active shake. Drawing actions
automatically request the necessary NMI uploads; scripts do not manage VRAM or
transfer queues directly.

Cutscene triggers are separate from scripts. An area event selects and starts a
script, while the script describes only what happens after it begins. The
format does not initially include branches, loops, arbitrary ASM calls,
arbitrary memory writes, or sprite choreography.
