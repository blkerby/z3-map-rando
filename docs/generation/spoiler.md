# Spoiler

Seed pages have a collapsed "Spoiler" section showing one row of item icons per sphere, with each item's location on hover.

## Approach

The spoiler is not stored with the seed. It is an exploration of the seed, defined by its world layout and item placements, against the current logic. The browser computes it on demand by replaying logical traversal through WebAssembly, using the current logic catalog and the seed's resolved settings. Seeds therefore need no spoiler data and no compatibility rules for it, and the spoiler is independent of the generator's placement algorithm.

Because current logic may differ from the logic that generated the seed, some placements may be unreachable in the replay, or reachable in other spheres than at generation. Unreachable locations are listed in a final group rather than treated as errors. A saved room, item location, or Small Key door missing from the current catalog is an error that stops the replay; the [seed vocabulary](../../crates/catalog_builder/tests/seed_vocabulary.rs) test keeps released identifiers from disappearing.

## Spheres

Spheres are classic: sphere 1 is everything collectible with the starting inventory, and sphere N is everything newly collectible with the items of all earlier spheres. Every item, including Small Keys, Big Keys, and prizes, waits for the next sphere to take effect. Non-item discoveries, such as flags, door openings with already-owned keys, Ocarina activation, and refills, are applied within a sphere until no more are found, since they need no item pickup.

[`generator::spoiler::replay_spoiler`](../../crates/generator/src/spoiler.rs) rebuilds the seed's `WorldLayout` and traversal graph, maps each saved `(room_id, item_id)` to a catalog item location, and reuses `compute_closure` (see [traversal](traversal.md)), which collects only locations that have a placement. Each call has only the earlier spheres' placements filled in, so it explores and applies non-item discoveries but collects nothing new. The newly reachable locations form the next sphere; their placements are filled in for the following call, which collects them at its start and explores further. The replay ends when a call discovers no new location. Traversal state carries over between calls, so each call revisits only the edges affected by newly collected items. The replay keeps a copy of the global state after each call that yields a sphere, for tracing routes.

## Routes

A route to a reachable location is traced on demand by `SpoilerReplay::trace_route`, as a fresh search rather than from the replay's incremental traversals, which can take roundabout paths. The search uses the saved global state of the location's sphere: the items of all earlier spheres, with every flag, door, Ocarina activation, and refill they allow. That state is a closure fixpoint, so the search applies no discoveries. Routes start at Link's House.

The search is Dijkstra's algorithm over pairs of a vertex and a local state, with each edge costing 1, so it finds a route with the fewest edges. Each entry records the entry and edge it was reached from. A per-vertex `LocalStateReducer` decides which states are worth expanding, as in normal traversal; the reducer is lossy and order-dependent, so a found route may not be strictly minimal, and in rare cases no route may be found for a location the replay reached. An entry whose edge collects the target location is queued even if its reducer rejects it, and popping it ends the search. Edges are evaluated one source state at a time through the ordinary `Interpreter`, so route tracing adds no bookkeeping to generation or the sphere replay.

## Browser and page

The browser package exports a `SpoilerSession` class, which replays the seed JSON against the logic catalog and keeps the replay in WebAssembly memory. Its `get_spheres` returns the spheres and unreachable locations as JSON. Each entry has the catalog location index, the location name, the item kind token, the name of the door a Small Key opens, and the dungeon name of a Map, Compass, or Big Key. Its `trace_route` takes a location index and returns the route as JSON rows, or null if none is found. Each row has the destination vertex's room and node names, the strat name or a name synthesized from the connection kind, the flags, doors, and Ocarina activation that the edge's requirement reads (including alternatives the route may not have used), and the local state on arrival. The spoiler adds about 72 KB gzipped to the package. A replay takes about 70–80 ms in the browser, and a route a few milliseconds.

The seed page fetches the logic catalog (about 55 KB) and runs the replay the first time the section is opened ([`SpoilerList.svelte`](../../frontend/src/lib/spoiler/SpoilerList.svelte)). Each sphere is a row of icons, one per item so that each stays individually hoverable, ordered by placement tier with prizes and dungeon items between the tiers; empty slots are omitted. Progressive items show the level each copy reaches, counting copies from the seed's starting equipment in sphere order and then catalog order. Items without an icon yet show a text badge until an icon named after the item is added to the item assets.

Rows start hidden as rows of question marks, and clicking a hidden row reveals it and every row above it; buttons reveal or hide all rows. Clicking a revealed item shows a table of its route in a dialog; the "State" column shows only the components that differ from the previous row, or on the first row from a state with nothing used, with icons for health, magic, bombs, arrows, and bottles. The page frees the session when the seed page is left.
