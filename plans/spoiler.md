# Spoiler

Status: in progress. Work items 1 through 4 are implemented.

The seed page gets a "Spoiler" section, below Generate and Customize, collapsed by default. Expanded, it lists one collapsible per sphere, each listing the items collectible in that sphere with their locations.

## Approach

The spoiler is not stored with the seed. It is an exploration of the seed, defined by its world layout and item placements, against the current logic. The browser computes it on demand by replaying logical traversal through WebAssembly, using the current logic catalog and the seed's resolved settings. Seeds therefore need no spoiler data and no compatibility rules for it, and the spoiler is independent of the generator's placement algorithm.

Because current logic may differ from the logic that generated the seed, some placements may be unreachable in the replay, or reachable in other spheres than at generation. The spoiler reports unreachable locations in a final group rather than treating them as errors. A saved room, item location, or Small Key door missing from the current catalog is an error that stops the replay; the [seed vocabulary](../crates/catalog_builder/tests/seed_vocabulary.rs) test keeps released identifiers from disappearing.

## Spheres

Spheres are classic: sphere 1 is everything collectible with the starting inventory, and sphere N is everything newly collectible with the items of all earlier spheres. Every item, including Small Keys, Big Keys, and prizes, waits for the next sphere to take effect.

Non-item discoveries, such as flags, door openings with already-owned keys, Ocarina activation, and refills, are applied within a sphere until no more are found, since they need no item pickup.

The replay reuses `compute_closure` (`crates/generator/src/traversal.rs`), which collects only locations that have a placement. Each sphere calls it with only the earlier spheres' placements filled in, so it explores and applies non-item discoveries but collects nothing new. The newly reachable locations form the sphere; their placements are filled in for the next call, which collects them at its start and explores further. The replay ends when a call discovers no new location. Traversal state carries over between calls, so each call revisits only the edges affected by newly collected items.

## Work

1. **Graph from a saved layout.** `select_borders` (`crates/generator/src/map.rs`) chooses border variants and builds the connections in one pass. Split it so the connections are built from a `WorldLayout` whose borders are already chosen, and use that in both generation and replay.
2. **Replay.** `generator::spoiler::replay_spheres` rebuilds the `WorldLayout` and graph from the seed, maps each saved `(room_id, item_id)` to an `ItemLocationIndex` through the logic catalog, and returns the spheres and unreachable locations as catalog indices. Small Keys keep their saved `DoorId`, which collection resolves to the catalog door.
3. **Browser entry point.** Implemented as `replay_spoiler`. `crates/browser` exports a function taking the seed JSON and the logic catalog, calling `replay_spheres`, and returning the spheres, each a list of item locations, plus the unreachable ones. It returns display names from the catalog: the location name, and for Small Keys the name of the door the key is assigned to.
4. **Item labels.** Each record of `data/items.json` has a `label`, carried into `settings_data.json` and used for `ITEM_LABELS` in `frontend/src/lib/generation/data.ts`.
5. **Seed page.** Add the collapsed Spoiler section and its nav link on seed pages only (not `/generate`). On first expansion, fetch the logic catalog (about 55 KB) and run the replay. Each sphere header shows its number and item count; each row shows the item and its location. Items use `ITEM_LABELS`, except that a Small Key is shown with its assigned door, e.g. "Small Key (<door name>)", and an empty slot (`nothing`, not an `items.json` record) is shown as "Nothing". All items are listed, in catalog order within a sphere.

## Later

- A filter for which items to list.
- Adjustable logic settings (techs, proficiencies) to explore how they change the spheres; the replay already takes the settings as input.
- Routes for each item, from the traversal of the sphere that reached it.
