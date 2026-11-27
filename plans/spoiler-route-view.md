# Spoiler route view

The spoiler's route dialog (see [spoiler](../docs/generation/spoiler.md#browser-and-page)) shows a table with the world map below it, so following the route on the map means scrolling back and forth. This plan replaces it with a full-viewport view: the route on the left, narrowed by grouping steps by room, and the map fixed in place on the right.

## Layout

- The route stays in a `<dialog>` opened with `showModal()`, sized to fill the viewport, keeping focus trapping, Esc to close, and the spoiler list's state underneath.
- A header row holds the target's icon, title, subtitle, and close button.
- Below the header, a two-column grid fills the remaining height. The left column scrolls; the right column holds the map, scaled with pixelated rendering to `min(column width, remaining height)`, with the world toggle below it. The ROM button, rendering status, and map error appear in the same column.
- On narrow screens, the map stacks on top at a fixed share of the height (about 40%), and the route scrolls below it.

## Route structure

Steps are grouped by consecutive runs in the same room and world. Each group starts with a header naming the connection that entered it and the room, followed by one line per node reached:

```
Start at Link's House
  Entrance
  North Door                  ← links to the strat page of the step reaching it
    Hookshot Across           ← strat name, muted and indented; omitted if exactly "Base"
    · Opened: Link's House door

Cross West to Swamp Ruins     ← Dark World groups tinted
  East Side                   ← arrival node, unlinked
  Dam Area
    Swim Around
```

- Each line is a node reached, rather than the node a step leaves from. The node name carries the link to the strat page of the step that reached it, so a "Base" strat, whose name is omitted, stays reachable. Other strat names, including variants such as "Base - Already Bombed", are shown as muted, indented text under the node.
- Dependencies (with their route links) and the state changes from `describeState` go in the muted block under the node, after the strat name.
- The room name is the overworld room's Dark World name in the Dark World. The Dark World tint, currently on rows, moves to the whole group.
- Group headers name the destination room:
  - The route start: "Start at <room>", followed by the starting node.
  - Map boundaries: "Cross West to <room>", "Mirror West to <room>".
  - Entrances: "Enter <interior room>", "Drop into <interior room>", "Exit to <overworld room>". These name the room rather than the entrance, since the two can differ once entrances are shuffled.
  - Warps: "Whirlpool to <room>", "Teleport to <room>", "Flute to <room>", "Save and quit to <room>".
  - The implicit Mirror edge, which stays in the same overworld room but changes world: "Mirror to <Dark World name>".
- A "Leave North and return" crossing stays in the current group, as a node line with that text in place of a strat name.
- Event annotations on vertices (such as `[event]`) stay on the node name.

`SpoilerSession::trace_route` returns structured rows for this: the destination node's room name (world-specific), node name, world (or the interior's world inferred from its entrances), the connection kind and direction for edges that change room or world, the strat name and link, dependencies, state, and map points. The frontend does the grouping. The current `location`, `strat`, and `dark_world` fields go away.

## Follow-up

These are part of this plan but pending until the work above is done:

- Highlight the selected or hovered step's (or group's) part of the line on the map, and switch the map to that step's world.
- Clicking a point on the map scrolls the route to the step that crosses it.
- A Back button in the view: following a dependency pushes the current route onto a history stack instead of replacing it.

When done, move the lasting design into [spoiler](../docs/generation/spoiler.md) and remove this plan.
