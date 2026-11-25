# Spoiler

Seed pages have a collapsed "Spoiler" section listing one collapsible group per sphere, each with the items first collectible in that sphere and their locations.

## Approach

The spoiler is not stored with the seed. It is an exploration of the seed, defined by its world layout and item placements, against the current logic. The browser computes it on demand by replaying logical traversal through WebAssembly, using the current logic catalog and the seed's resolved settings. Seeds therefore need no spoiler data and no compatibility rules for it, and the spoiler is independent of the generator's placement algorithm.

Because current logic may differ from the logic that generated the seed, some placements may be unreachable in the replay, or reachable in other spheres than at generation. Unreachable locations are listed in a final group rather than treated as errors. A saved room, item location, or Small Key door missing from the current catalog is an error that stops the replay; the [seed vocabulary](../../crates/catalog_builder/tests/seed_vocabulary.rs) test keeps released identifiers from disappearing.

## Spheres

Spheres are classic: sphere 1 is everything collectible with the starting inventory, and sphere N is everything newly collectible with the items of all earlier spheres. Every item, including Small Keys, Big Keys, and prizes, waits for the next sphere to take effect. Non-item discoveries, such as flags, door openings with already-owned keys, Ocarina activation, and refills, are applied within a sphere until no more are found, since they need no item pickup.

[`generator::spoiler::replay_spheres`](../../crates/generator/src/spoiler.rs) rebuilds the seed's `WorldLayout` and traversal graph, maps each saved `(room_id, item_id)` to a catalog item location, and reuses `compute_closure` (see [traversal](traversal.md)), which collects only locations that have a placement. Each call has only the earlier spheres' placements filled in, so it explores and applies non-item discoveries but collects nothing new. The newly reachable locations form the next sphere; their placements are filled in for the following call, which collects them at its start and explores further. The replay ends when a call discovers no new location. Traversal state carries over between calls, so each call revisits only the edges affected by newly collected items.

## Browser and page

The browser package exports `replay_spoiler`, which takes the seed JSON and the logic catalog and returns the spheres and unreachable locations as JSON. Each entry has the location name, the item kind token, and for a Small Key the name of the door it opens. It adds about 72 KB gzipped to the package, and a replay takes about 70–80 ms in the browser.

The seed page fetches the logic catalog (about 55 KB) and runs the replay the first time the section is opened ([`SpoilerList.svelte`](../../frontend/src/lib/spoiler/SpoilerList.svelte)). Sphere groups start collapsed. Rows list items in catalog order, labeled with `ITEM_LABELS`, except that a Small Key is shown with its assigned door, e.g. "Small Key (<door name>)", and an empty slot is shown as "Nothing".
