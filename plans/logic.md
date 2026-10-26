# Logic catalog

The builder compiles `z3-json-data` into a compact binary catalog for generation. It does not place items or implement traversal. Use the catalog envelope described in [the architecture plan](README.md#catalog-format): uncompressed magic bytes and root schema hash, followed by a Zstd-compressed `bincode-next` payload encoded and decoded through its streaming Serde API. Shared types derive `Serialize` and `Deserialize` in [`logic_catalog`](../crates/logic_catalog/src/lib.rs). Build the catalog with:

```sh
mkdir -p build
cargo run -p catalog_builder -- logic
```

The builder reads source room, item, helper, tech, enemy, and connection definitions. It writes `Z3LOGIC\0`, the little-endian schema hash, and the compressed Serde bincode payload. Room indices follow source namespace and ID order; definitions retain source order. Compression is selected by `compression_level` in `catalog-build.toml`, with level `3` in the default template.

`compute_schema_hash` traces the root and each enum with `serde-reflection`, then hashes the ordered schema registry. Recursive requirements use named references; enum changes need neither explicit discriminants nor representation attributes.

## Graph

The initial catalog has three main flat lists:

- **Vertices:** authored room/node references, with separate Light/Dark World vertices for overworld nodes that exist in both worlds. Interiors have one ordinary vertex per node, with no world assigned. Boundary events add separate entry and exit vertices at affected endpoints. Catalog room IDs index the room list; source overworld and underworld identities are retained in each room.
- **Edges:** directed actions with a source, destination, conditions, and effects. Preserve parallel strats and self-loops.
- **Item locations:** catalog indices, names, and a single associated vertex. Retain the authored `(room_id, item_id)` pair as the stable identity for saved placements. These IDs must not be renumbered or reused; `item_id` is the item entry's ID, not its `itemLocation` node. Overworld items reference the vertex in their world. Collection actions reference the item-location index; reaching the associated vertex does not automatically collect its item. Fixed events such as flute activation are not randomized item locations.

Room graphs share one vertex space; inter-room connections are stored separately from their edges in `vanilla_connections`. Generation uses these pairings for the vanilla layout or supplies replacements for a randomized layout. Separate catalog lists retain entrances, screen boundaries, teleports, whirlpools, and flute spots, each referencing a vertex. Traversal starts at Light World Link's House; other spawn points are deferred.

Connection records resolve source references to catalog endpoint indices:

- Entrances pair an `EntranceIndex` with an interior `VertexIndex`. Doors are bidirectional; drops lead only into the interior. Each overworld entrance retains its authored room-local ID and an optional native `rom_entrance_id` from source `romEntranceId`. Native IDs are absent for special overworld transitions and may be shared by several ordinary entrances.
- Teleports lead from an interior vertex to a `TeleportIndex`. The destination determines the world; source `fromWorld` is ignored. Interiors have no world state.
- Whirlpools pair two `WhirlpoolIndex` endpoints for bidirectional traversal.
- Screen boundaries pair two `ScreenBoundaryIndex` endpoints for bidirectional traversal between adjacent overworld areas.

The builder matches overworld boundaries by opposite directions, the same world, and exactly equal spans in global tile coordinates. Area grid positions use 32 tiles per cell, while room sizes use 16 tiles per screen. Every boundary must have exactly one neighboring match; missing or ambiguous matches fail the build with room, node, world, direction, and span diagnostics. Each pair is stored once. `generator::graph::build_graph` expands the chosen connections into directed outgoing adjacency, including follower handling and matching event endpoints, without changing the catalog. It also includes the catalog's flute edges and adds a free save-and-quit edge from each vertex to the start.

The builder appends a synthetic `RoomKind::FluteHub` room with one ordinary node and vertex. `Room.room_id` is `Some(SourceRoomId)` for authored rooms and `None` for the hub, while every vertex retains mandatory room and node indices. `flute_hub_vertex_idx` identifies the hub. `ImplicitFlute` catalog edges connect every ordinary Light World overworld vertex to the hub with an active-flute requirement, then connect the hub to all eight destinations for free. Flute movement preserves the modeled followers and resources; moving through its synthetic room uses normal room-boundary obstacle resets.

Catalog indices use distinct newtypes, and fields holding them end in `_idx`. Each room stores node metadata in a vector indexed by `NodeIndex`, with authored IDs retained in `NodeMetadata::node_id`. The builder resolves source node IDs to room-local indices. Light/Dark overworld vertices and extra event vertices share the index of their authored node. Each room also stores strat names in source order and obstacles. Edge provenance lives in the catalog's `edge_metadata` vector, with one `EdgeMetadata` record per edge in the same order as `edges`, including for edges between rooms. Strat origins record both the owning room index and its room-local strat index. Obstacle indices refer to the edge's source room's obstacle list, and obstacle state resets on leaving that room. Vertices retain only their room index for room-local state. The parallel `vertex_metadata` vector holds each vertex's node index, optional overworld world, and origin: `Node`, `EventEntry`, or `EventExit`. Event origins include the event index. `vertex_metadata(VertexIndex)` accesses the matching record. Requirements and effects remain on edges, using boxed slices for fixed-length lists.

Each catalog item location also retains its vanilla item index, allowing generation to identify dungeon maps and compasses, shuffle pendants and crystals only among prize locations, and retain vanilla Small Key and Big Key locations while assigning Small Key door identities. This is generation metadata, not a receipt ID or ROM address.

Item-location identity is separate from item identity: generation decides which item, including which [door-specific key](keys.md), occupies it. Saved seeds use stable source identities rather than catalog indices. The current patch catalog maps those identities to patching instructions, so patching an old seed does not require its original logic catalog.

Names and strat provenance are retained for diagnostics. Tech definitions retain their stable source IDs for player settings and their supported proficiency tiers. Item definitions retain names as stable item identities. Locations retain the shared `ItemLocationId` authored room/item pair. Location ROM addresses, receipt IDs, and dungeon-prize patch bytes are stored only in the patch catalog. The patch builder extracts them directly from `z3-json-data`, without consuming the logic catalog. Both builders may share source-deserialization types.

Room-specific techniques are retained in `LogicCatalog::notables` with their owning room, stable room-local notable ID, and name. Source notable names resolve only in the containing room and compile to `Requirement::Notable(NotableIndex)`, using a dense catalog-wide index. Settings identify toggles by stable room identity and notable ID; names are informational. Generation resolves these settings once to `GenerationInputs::notable_enabled`, with unspecified notables disabled. The interpreter checks this array without changing local state or granting implicit tech, items, or resources.

## Boundary events

The builder compiles source `entranceState` and `exitState` into extra vertices listed in `event_entries` and `event_exits`. Each record identifies the event and physical endpoint: an overworld entrance index, or the interior vertex's room index and its metadata's node index. Extra vertices retain the ordinary endpoint's room, node, and world, but have distinct vertex indices. Their metadata origins distinguish them from ordinary nodes during endpoint discovery.

An entrance-state strat starts at its event-entry vertex; an exit-state strat ends at its event-exit vertex. A strat with both uses both substitutions. Its requirements and effects remain on the edge. Entry and exit vertices are distinct even when they refer to the same event and physical endpoint.

Generation connects an event-exit vertex to an event-entry vertex only when their physical endpoints are connected and their event IDs match. Only matching room-connection edges enter event-entry vertices or continue travel out of event-exit vertices; save-and-quit remains available from every vertex. Ordinary room connections remain separate. Graph construction appends `ResetObstacles(destination_room_idx)` after departure requirements on every room-crossing edge, including flute travel. This requirement replaces local obstacle state with an uncleared state sized for the destination room. Movement within a room preserves obstacle state, except save-and-quit also injects the reset when already in the starting room. No separate edge flag or movement handler resets obstacles.

For example, the Dam's exit strat checks the drained-floodgate obstacle before reaching its `DrainedFloodgate` exit vertex. The generated connection leads to Swamp Ruins' matching entry vertex, whose strat clears the overworld obstacle and reaches the ordinary node. Walking to the ordinary node cannot activate that entrance-state strat. Event identity is represented by graph position; the traverser does not carry a separate entrance/exit event state.

## Action composition

Support almost all source constructs. Requirements include composition, item and equipment checks, tech/proficiency checks, flags, prize counts, resource use and refills, damage, payment, followers, obstacles, and door checks. Resolve referenced definitions during building; enemy attacks only need their relevant damage data.

`And` executes in source order; `Or` retains alternative successful local states. Resource actions are stateful, not Boolean predicates. Apply magic uses and damage hits individually so potions and Fairy revival can intervene between them. Apply strat-level effects after the requirement succeeds.

Magic requirements compile to item/equipment checks followed by `UseMagic { cost_per_use, num_uses }`. Either-rod use accepts either rod; medallions also require a sword. Blue Cane pays one 20-point startup use, followed by 4-point drain uses. Cape has no startup cost and drains one point per use. Costs remain in base normalized magic points.

Source `upgradeCapacity` requirements compile to `Never`, disabling vanilla ammunition-capacity purchases. Randomizer-specific replacement behavior remains deferred.

Keep a recursive `Requirement` on each edge, without additional expression or effect vertices. Door-specific keys remove the main need for permanent effects within a requirement. Any future support for such effects must preserve their incremental execution, including successful prefixes of incomplete strats.

Expand helpers and tech dependencies during building, retaining each tech's enabled-tech check and proficiency thresholds. Helpers use the selected source definitions rather than generation-time configuration.

## Progression semantics

- Boss discoveries affect global state only. Keep their strats as ordinary edges; no terminal vertices are needed. Ignore source `killExitNode` automatic exits.
- `notFlag` and `swordExact` become `Never`. Other conservative Armos strats remain available. `shieldExact` becomes the corresponding minimum-level shield requirement, keeping the fixed Waterfall exchange collectible with progressive shields.
- Follower requirements name a specific follower. Source `"None"` becomes `LoseFollowers` with all followers: Zelda, Old Man, Blind, Dwarf, Purple Chest, and Super Bomb. Followers can be discarded whenever needed. Ordinary door connections inject `LoseFollowers` for Old Man, Blind, Purple Chest, and Super Bomb, plus Dwarf when the overworld entrance's native ROM entrance ID is below `$3C`, allowing entry after discarding followers that cannot enter. Drops inject only Super Bomb loss. Ordinary door exits inject Blind loss. The shared `common_types::allows_frog_dwarf` rule is also used by the ROM importer and covers all 59 forbidden native door records, including non-dungeon caves and Link's House. Door-departure teleports (Houlihan's exit) inject Blind loss; the Agahnim event teleports preserve followers. Ordinary and matching event connections use the same requirements, without follower-specific traversal-edge metadata.
- Bomb use first applies `LoseFollowers([SuperBomb])`, since the Super Bomb can be discarded before using ordinary bombs. Source follower completion becomes `SetFollower(None)`.
- Save and quit injects `And([LoseFollowers([Blind, SuperBomb]), ResetHealthToFull, ResetObstacles(start_room_idx)])`; Zelda, Old Man, Dwarf, and Purple Chest persist. It includes implicit full healing at Link's House and retains current magic, bombs, arrows, and bottle use. Preparation discoveries update global usable capacities between traversal waves.
- The catalog targets a patched game where the Golden Sword damages Mothula. Treat `h_MothulaVulnerableToGoldSword` as `Free` before helper expansion.
- Shared inventory slots will be split to retain independent use of Shovel/Flute, Mushroom/Powder, and blue/red Boomerang. Equipment upgrades preserve abilities.
- Flute activation is a fixed event granting `OcarinaActive`, not a placement slot. Its `OcarinaInactive` requirement is harmless if the flute is already active: repeating the event provides no new ability.
- `isBunny: "no"` requires Moon Pearl in Dark World and adds no restriction in Light World. Omitted values use the source default of `"no"`; `"any"` adds no restriction, and `"yes"` becomes `Never`. Add implicit Mirror edges from eligible Dark World overworld nodes to their Light World copies, requiring Magic Mirror.

## Resource interpretation

The generator starts routes at full health, accounting for repeated healing at Link's House. Independent obtained-refill flags track preparation even when magic/ammo capacity is zero. Usable capacity is physical capacity when its source has been obtained, or zero otherwise; recompute it after discoveries and capacity changes. Environmental `Refill` requirements only report preparation for later waves; none restores resources during a route until source repeatability can be identified. Refill limits remain catalog data. Bottle sources establish prepared contents even before a bottle is owned, without replenishing the current route's bottle budget. Fairy-source discovery still requires the Bug-Catching Net. Actual potion use or Fairy revival requires an unused owned bottle.

Health is stored as `HealthState::Consumed(u)` relative to full capacity, or `Remaining(r)` after bottled Fairy revival. Damage updates the active representation, full heals restore `Consumed(0)`, and revived remaining health stays fixed across capacity increases. Health missing-resource checks require the consumption representation. For scoring only, `Remaining(r)` maps to usage `160 - r`, keeping scores independent of current capacity. Magic/ammo/bottle usage remains plain consumption with no unprepared sentinel. See [the resource preparation and incremental traversal plan](incremental-traversal.md).

## Traversal implications

Traversal uses a fixed global inventory and permanent flags for a wave, plus multiple local states at each vertex. Local state includes resources, obstacles, and followers. Successful collection and flag actions report global discoveries; guaranteed discoveries become available in the next wave. Continuations requiring an update wait for the updated state. Per-vertex reducers persist across waves and accepted placement candidates, with unchanged local values and metric scores as capabilities improve. A reverse index built over the expanded graph maps mutable global reads to dependent edge positions. Changes to inventory, discoveries, or preparation schedule only affected edges with retained source states, including resource-consuming edges whose source reducers have not improved. Nested requirements, implicit consumable dependencies, and global reads in effects are indexed. Applied discoveries directly supply dependency keys, and item collection returns a key only for an actual capability change. Closure looks up these keys without per-wave global clones or a scan of all dependencies. Initial work is deduplicated and sorted by graph order. Junk collection schedules no traversal. New complete attempts start from fresh inventory and reducers; rejected candidate clones discard both.

Door-specific keys behave as ordinary persistent items. Preliminary key assignment reuses normal traversal with full non-key inventory and other dungeons' key access. Both passes enforce player settings, resources, and temporary state; normal placement must additionally establish item progression. There is no key-alternatives DAG.

Identical local states can merge. Resource dominance only compares compatible temporary states; independent maxima must not create invented states. Cleared obstacles are not universally preferable because the source also tests uncleared obstacles.

The interpreter keeps the actual state winning each of 18 fixed cost metrics, bounding its intermediate and output local-state collections. Dropping logical routes sacrifices completeness; retained states must remain conservative summaries of achievable continuations. After capacity or Mail improves, a cached Fairy revival can summarize a route that instead survives the earlier hit and retains its Fairy for later damage. Metric choice and tie handling are traversal decisions, not catalog data.

## Deferred work

- Generation assigns unique key pickups to logical doors; the catalog retains door identities and lock types, not generated key assignments. Big Keys are scoped to the pickup’s dungeon; logical-door-to-ROM-door associations remain deferred.

The shared generation graph and normal traverser are implemented. `generator::traversal::traverse` first evaluates the supplied initial edge positions directly, queuing improved destination vertices. A FIFO vertex queue then expands all outgoing edges against one retained-state snapshot per expansion. One queued bit per vertex deduplicates work, and self-loop improvements can requeue their vertex. `compute_closure` extends retained reducers and applies discoveries and reachable placed items between waves, recording unfilled locations as reachable without collecting hypothetical items. Save-and-quit requirements restore full health and reset obstacles while retaining magic, ammunition, and bottle use. Resource preparation updates global usable capacities between waves, and local counters start at zero for a new attempt. `PlacementState` owns reducers and inventory at the existing candidate clone/commit boundary. Initial externally granted capability changes, applied discoveries, and item receipts seed dependent edges through direct reverse-index lookups, while newly reached vertices expand normally. Discovery filtering in collection effects is indexed through reachable-location records. Provenance tracking remains deferred. The generator sequences single placement candidates with bounded retries, rerandomizing keys on each complete attempt. Completion reads the accepted closure’s collected-location bits after filling all slots. ROM key patches remain later work. Repeatable shop shield receipts from `grantsItems` are deliberately ignored until fixed-level shop equipment grants are modeled.
