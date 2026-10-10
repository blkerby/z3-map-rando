# Logic catalog

The builder compiles `z3-json-data` into a compact binary catalog for generation. It does not place items or implement traversal. Use the catalog envelope described in [the architecture plan](README.md#catalog-format): uncompressed magic bytes and root schema hash, followed by a Zstd-compressed `bincode-next` payload encoded and decoded through its streaming Serde API. Shared types derive `Serialize` and `Deserialize` in [`logic_catalog`](../crates/logic_catalog/src/lib.rs). Build the catalog with:

```sh
mkdir -p build
cargo run -p catalog_builder -- logic ../z3-json-data build/logic_catalog.bin
```

The builder reads source room, item, helper, tech, enemy, and connection definitions. It writes `Z3LOGIC\0`, the little-endian schema hash, and the compressed Serde bincode payload. Room indices follow source namespace and ID order; definitions retain source order. Compression defaults to Zstd level `3`, with a `--compression-level` override.

`compute_schema_hash` traces the root and each enum with `serde-reflection`, then hashes the ordered schema registry. Recursive requirements use named references; enum changes need neither explicit discriminants nor representation attributes.

## Graph

The initial catalog has three main flat lists:

- **Vertices:** authored room/node references, with separate Light/Dark World vertices for overworld nodes that exist in both worlds. Interiors have one ordinary vertex per node, with no world assigned. Boundary events add separate entry and exit vertices at affected endpoints. Catalog room IDs index the room list; source overworld and underworld identities are retained in each room.
- **Edges:** directed actions with a source, destination, conditions, and effects. Preserve parallel strats and self-loops.
- **Item locations:** catalog indices, names, and a single associated vertex. Retain the authored `(room_id, item_id)` pair as the stable identity for saved placements. These IDs must not be renumbered or reused; `item_id` is the item entry's ID, not its `itemLocation` node. Overworld items reference the vertex in their world. Collection actions reference the item-location index; reaching the associated vertex does not automatically collect its item. Fixed events such as flute activation are not randomized item locations.

Room graphs share one vertex space; inter-room connections are stored separately from their edges in `vanilla_connections`. Generation uses these pairings for the vanilla layout or supplies replacements for a randomized layout. Separate catalog lists retain entrances, screen boundaries, teleports, whirlpools, and flute spots, each referencing a vertex. Traversal starts at Light World Link's House; other spawn points are deferred.

Connection records resolve source references to catalog endpoint indices:

- Entrances pair an `EntranceIndex` with an interior `VertexIndex`. Doors are bidirectional; drops lead only into the interior.
- Teleports lead from an interior vertex to a `TeleportIndex`. The destination determines the world; source `fromWorld` is ignored. Interiors have no world state.
- Whirlpools pair two `WhirlpoolIndex` endpoints for bidirectional traversal.
- Screen boundaries pair two `ScreenBoundaryIndex` endpoints for bidirectional traversal between adjacent overworld areas.

The builder matches overworld boundaries by opposite directions, the same world, and exactly equal spans in global tile coordinates. Area grid positions use 32 tiles per cell, while room sizes use 16 tiles per screen. Every boundary must have exactly one neighboring match; missing or ambiguous matches fail the build with room, node, world, direction, and span diagnostics. Each pair is stored once. Generation expands the chosen connections into edges, including follower handling and matching event endpoints. Flute travel remains separate.

Catalog indices use distinct newtypes, and fields holding them end in `_idx`. Each room stores node metadata in a vector indexed by `NodeIndex`, with authored IDs retained in `NodeMetadata::node_id`. The builder resolves source node IDs to room-local indices. Light/Dark overworld vertices and extra event vertices share the index of their authored node. Each room also stores strat names in source order and obstacles. Edge provenance lives in the catalog's `edge_metadata` vector, with one `EdgeMetadata` record per edge in the same order as `edges`, including for edges between rooms. Strat origins record both the owning room index and its room-local strat index. Obstacle indices refer to the edge's source room's obstacle list, and obstacle state resets on leaving that room. Vertices retain only their room index for room-local state. The parallel `vertex_metadata` vector holds each vertex's node index, optional overworld world, and origin: `Node`, `EventEntry`, or `EventExit`. Event origins include the event index. `vertex_metadata(VertexIndex)` accesses the matching record. Requirements and effects remain on edges, using boxed slices for fixed-length lists.

Item-location identity is separate from item identity: generation decides which item, including which [door-specific key](keys.md), occupies it. Saved seeds use stable source identities rather than catalog indices. The current patch catalog maps those identities to patching instructions, so patching an old seed does not require its original logic catalog.

Names and strat provenance are retained for diagnostics. Tech definitions retain their stable source IDs for player settings and their supported proficiency tiers. Item definitions retain names as stable item identities. The current catalog also stores location ROM addresses, receipt IDs, and dungeon-prize patch bytes; remove these patching fields as part of the planned boundary change. The patch builder extracts them directly from `z3-json-data`, without consuming the logic catalog. Both builders may share source-deserialization types.

## Boundary events

The builder compiles source `entranceState` and `exitState` into extra vertices listed in `event_entries` and `event_exits`. Each record identifies the event and physical endpoint: an overworld entrance index, or the interior vertex's room index and its metadata's node index. Extra vertices retain the ordinary endpoint's room, node, and world, but have distinct vertex indices. Their metadata origins distinguish them from ordinary nodes during endpoint discovery.

An entrance-state strat starts at its event-entry vertex; an exit-state strat ends at its event-exit vertex. A strat with both uses both substitutions. Its requirements and effects remain on the edge. Entry and exit vertices are distinct even when they refer to the same event and physical endpoint.

Generation connects an event-exit vertex to an event-entry vertex only when their physical endpoints are connected and their event IDs match. Only room-connection edges may enter event-entry vertices or leave event-exit vertices. Ordinary room connections remain separate. Crossing a room boundary still resets room-local obstacle state.

For example, the Dam's exit strat checks the drained-floodgate obstacle before reaching its `DrainedFloodgate` exit vertex. The generated connection leads to Swamp Ruins' matching entry vertex, whose strat clears the overworld obstacle and reaches the ordinary node. Walking to the ordinary node cannot activate that entrance-state strat. Event identity is represented by graph position; the traverser does not carry a separate entrance/exit event state.

## Action composition

Support almost all source constructs. Requirements include composition, item and equipment checks, tech/proficiency checks, flags, prize counts, resource use and refills, damage, payment, followers, obstacles, and door checks. Resolve referenced definitions during building; enemy attacks only need their relevant damage data.

`And` executes in source order; `Or` retains alternative successful local states. Resource actions are stateful, not Boolean predicates. Apply magic uses and damage hits individually so potions and Fairy revival can intervene between them. Apply strat-level effects after the requirement succeeds.

Magic requirements compile to item/equipment checks followed by `UseMagic { cost_per_use, num_uses }`. Either-rod use accepts either rod; medallions also require a sword. Blue Cane pays one 20-point startup use, followed by 4-point drain uses. Cape has no startup cost and drains one point per use. Costs remain in base normalized magic points.

Keep a recursive `Requirement` on each edge, without additional expression or effect vertices. Door-specific keys remove the main need for permanent effects within a requirement. Any future support for such effects must preserve their incremental execution, including successful prefixes of incomplete strats.

Expand helpers and tech dependencies during building, retaining each tech's enabled-tech check and proficiency thresholds. Helpers use the selected source definitions rather than generation-time configuration.

## Progression semantics

- Boss rewards affect global state only. Keep their strats as ordinary edges; no terminal vertices are needed. Ignore source `killExitNode` automatic exits.
- `notFlag`, `swordExact`, and `shieldExact` become `Never`. Other conservative Armos strats remain available; the Waterfall shield exchange is unavailable under the current helper definitions.
- Follower requirements name a specific follower. Source `"None"` becomes `LoseFollowers` with all followers: Zelda, Old Man, Blind, Dwarf, Purple Chest, and Super Bomb. The randomizer will allow discarding any follower when needed.
- Bomb use first applies `LoseFollowers([SuperBomb])`, since the Super Bomb can be discarded before using ordinary bombs. Source follower completion becomes `SetFollower(None)`.
- The catalog targets a patched game where the Golden Sword damages Mothula. Treat `h_MothulaVulnerableToGoldSword` as `Free` before helper expansion.
- Shared inventory slots will be split to retain independent use of Shovel/Flute, Mushroom/Powder, and blue/red Boomerang. Equipment upgrades preserve abilities.
- Flute activation is a fixed event granting `OcarinaActive`, not a placement slot. Its `OcarinaInactive` requirement is harmless if the flute is already active: repeating the event provides no new ability.
- `isBunny: "no"` requires Moon Pearl in Dark World and adds no restriction in Light World. Omitted values use the source default of `"no"`; `"any"` adds no restriction, and `"yes"` becomes `Never`. Add implicit Mirror edges from eligible Dark World overworld nodes to their Light World copies, requiring Magic Mirror.

## Traversal implications

Traversal uses a fixed global inventory and permanent flags for a wave, plus multiple local states at each vertex. Local state includes resources, obstacles, and followers. Successful collection and flag actions report global discoveries; guaranteed discoveries become available in the next wave. Continuations requiring an update wait for the updated state.

Door-specific keys behave as ordinary persistent items. There is no key-alternatives DAG or key-specific all-tech/full-inventory possibility traversal. This does not remove local resource and obstacle trade-offs.

Identical local states can merge. Resource dominance only compares compatible temporary states; independent maxima must not create invented states. Cleared obstacles are not universally preferable because the source also tests uncleared obstacles.

One proposed approximation keeps the actual state winning each of a finite set of cost metrics, bounding the stored frontier by the number of metrics. Dropping logical routes sacrifices completeness; every accepted route must remain a valid witness. Metric choice and tie handling are traversal decisions, not catalog data.

## Deferred work

- Generation assigns unique key pickups to logical doors; the catalog retains door identities and lock types, not generated key assignments. Big-key scope and logical-door-to-ROM-door associations still need definition.

The traverser, metric implementation, item filler, ROM key patches, and generation connections are later work. The builder preserves the data they need without fixing unresolved behavior implicitly.
