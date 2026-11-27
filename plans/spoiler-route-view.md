# Spoiler route view follow-ups

Refinements to the spoiler's route dialog (see [spoiler](../docs/generation/spoiler.md#browser-and-page)), which shows the route beside a fixed map:

- Highlight the selected or hovered step's (or group's) part of the line on the map, and switch the map to that step's world.
- Clicking a point on the map scrolls the route to the step that crosses it.
- A Back button in the dialog: following a dependency pushes the current route onto a history stack instead of replacing it.

When done, move the lasting design into [spoiler](../docs/generation/spoiler.md) and remove this plan.
