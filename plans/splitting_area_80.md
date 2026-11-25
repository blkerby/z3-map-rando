# Splitting area $80

Status: not started. Decision: keep one area with per-part assets (option B below).

## Problem

Area `$80` is one 2x2-screen area containing two special rooms, Sacred Grove (room 102) and Under the Bridge (room 103). Each room takes its theme from its parent overworld room: Sacred Grove from Lost Woods (room 1), and Under the Bridge from Camper's Bridge (room 28). Their themes therefore differ whenever their parents' themes differ, but an area currently has one theme throughout: ALTTPRetiling has one source area, "80 Sacred Grove Under the Bridge", and the patcher, palette allocation, and generated asset records all work per area. Until this is resolved, both parts use the theme of Lost Woods.

At runtime the engine tells the parts apart by entrance: `$A0 == $0181` selects Under the Bridge and its `Bridge Shadow` BG1, and one BG2 map holds both parts, so the assets of both BG1 alternatives coexist in the area's one record (see [overworld backgrounds](../docs/engine/overworld_backgrounds.md#alternatives)).

The two parts are never on screen together. Each is reached only through a special-overworld transition, from Lost Woods or from Camper's Bridge, and the camera stays within the part that was entered.

## Options

### A: Split into two area IDs

Give Under the Bridge its own area ID, so that every area has one room and one theme.

- The data model stays uniform: themes, asset records, palette allocation, and the patcher's per-area theme lookup need no notion of parts.
- Every vanilla table indexed by `$8A` needs entries for the new area, including map data, overlays, sprite sets, music, and area size, as does code that checks for `$8A == $80`, such as the pedestal and fog handling. The bridge screens move into the new area's map data.
- ALTTPRetiling needs two source areas, through an upstream split or by the patcher slicing the source area.

### B: Keep one area, with a theme and asset record per part

Keep `$8A = $80` for gameplay. The patcher tiles each part from its own theme and allocates each part as its own asset unit, and each part's special-overworld transition selects that part's asset record through its special-area selector (see [overworld asset loading](../docs/engine/overworld_assets.md#area-record)).

- Engine changes are small, since asset keys already need not equal `$8A`, and vanilla per-area tables and code are untouched.
- The parts never share VRAM or palette space, since each record carries only its own theme.
- The patcher's area-based structures, such as retiling area preparation, theme filtering, and palette allocation, work per part rather than per area. This is one documented exception to one theme per area.
- ALTTPRetiling needs no change; the patcher slices the source area by screen.

## Comparison

| | A: split area | B: per-part assets |
| --- | --- | --- |
| Engine/ASM changes | Large: `$8A`-indexed tables and `$80` special cases | Small: transition selector chooses the asset key |
| Patcher changes | Moderate: new area, source slicing or upstream split | Moderate: per-part tiling and allocation |
| Upstream ALTTPRetiling | Split preferred | None |
| Data model | Uniform | One exception: parts within an area |
| Risk | High: vanilla `$80` behavior is spread across tables | Contained to project code |

Option B is chosen because its cost stays in project code rather than vanilla engine data, and it uses the asset-key mechanism that asset loading already has.

## Open questions

- Which screens of `$80` each part occupies. The patch catalog records rooms 102 and 103 with size `[1, 1]` in `$80`, without their positions; both options need this to slice the area.
- Whether Map16 definitions are shared across the whole area rather than per asset record. If so, per-part themes in option B must share those definitions.
