# Map randomization integration plan

Status: initial implementation present, with broader ROM gameplay verification outstanding. This document records the agreed behavior and verification scope. It builds on [engine milestones 10–12](../plans/engine.md#milestone-10-paired-world-area-topology-and-transitions), replacing the planned layout search with selection from the precomputed map pool. No Python runs during seed generation or ROM patching.

## Agreed behavior

- Add `map_layout` to `settings.json`, with exactly two values: `"random"` and `"vanilla"`. The bundled default is `"random"`.
- Support only the Desert theme initially. Do not silently substitute another theme or fall back to vanilla placement when random generation fails.
- Vanilla mode uses the existing `transitions` data for connectivity and the Desert `Main` layer directly for map tiling. It does not use `edgeVariants`, evaluate layer conditions, or composite additional layers onto that tiling.
- Random mode selects a layout from the map pool, then selects compatible variants for its touching edge segments. Favor open variants: exclude `closed` when the touching pair supports an open choice. Exterior segments use `closed`.
- Each Light World room and its Dark World counterpart remain one placement unit. Both worlds use the same room position and edge-variant selection; specialized variants describe their world-specific behavior.
- Before assigning any keys, check that every item location can be discovered from the normal starting node with full inventory and initially empty item-location discovery state. Use the actual selected graph and player logic settings, and require executable collection actions rather than merely reaching the associated vertices.
- Full inventory means the complete configured item pool, not maximum supported equipment independent of the settings. Include the fixed items and prizes, and assume all dungeon key access for this preliminary check.
- A failed reachability check causes another edge-variant selection on the same map. Allow 10 edge-variant attempts total per selected map, including the initial selection. If all 10 fail, the map consumes one failed generation attempt, just like a key-placement or item-placement failure.
- Use the existing quota of 1,000 complete generation attempts for all three failure stages. Each random-mode attempt selects a map; key-placement or item-placement failure moves to a new map on the next attempt. Preserve the existing inner key/candidate limits; do not introduce a separate map-selection budget or another 1,000-attempt placement loop per map.
- Configure the pool location with `map_pool = "maps/map_pool.bin"` in `catalog-build.toml` and its default template. Allow an explicit `--map-pool <PATH>` CLI override. `maps/map_pool.bin` in the randomizer repository is the default pool location.
- Save the resolved room positions and edge choices in the seed. Patching a saved seed must not require the pool or rerun random selection.
- Retain `$8A` and `$040A` as logical area identities with their existing loading/cache lifetimes. Resolve normal-overworld physical placement through generated ROM tables; no additional persistent RAM word is needed initially. Special overworld areas remain outside the shuffled grid, with their existing identities and internal coordinate systems.
- Graphics and palette allocation failures remain patch-time errors. Do not reject or resample a generated map because its assets exceed these budgets, and do not add allocation failures to the shared generation-attempt quota.

## Current implementation and input data

The randomizer's pool is at [`maps/map_pool.bin`](../maps/map_pool.bin), relative to the repository root. It was exported from `z3-map-gen`. Its `Z3MAPPL1` header describes 100,000 maps, 40 ordered source room IDs, and the source room-catalog SHA-256. Each map has 40 bytes, one top-left anchor per room encoded as `y * 8 + x`. The file occupies 4,000,124 bytes. The header hash matches the current `z3-map-gen/data/rooms.json` export. See the [pool format specification](../../z3-map-gen/docs/map_pool.md).

The pool guarantees complete, compatible room packing according to the map generator's catalog. It does not select border variants or prove reachability under randomizer logic. Those are generation-time responsibilities.

The integration started from these components:

| Component | Previous behavior | Integration change |
| --- | --- | --- |
| `seed_data::Settings` and `data/settings.json` | Item and logic settings, without a map choice | Add the two-value map setting and make random the bundled default |
| CLI and its bundled-catalog build | Load catalogs and pass decoded inputs into generation | Supply pool data for random generation and support the existing external-data/bundling workflow |
| `generator::generate` | Build a graph from `vanilla_connections` before item placement | Resolve placement and borders, check full-inventory reachability, then place keys and items |
| `generator::build_graph` | Accept an arbitrary selected connection slice | Expand selected variant endpoints, including specialized directed connections |
| `catalog_builder::logic` | Compile vanilla node transitions and spans | Also retain edge-variant support and endpoint references for random mode |
| `WorldLayout` | Persist area identities and themes, with vanilla placement implicit | Persist paired-room positions and selected border variants for random mode |
| Retiling builder and compiler | Retain layers but omit their `condition` fields | Use `Main` directly in vanilla mode; retain conditions and composite selected layers in random mode |
| Graphics/palette allocation | Use a hard-coded list of vanilla scrollable neighbor pairs | Derive coexistence constraints from resolved topology |
| ROM patcher and ASM | Use vanilla placement and many screen-ID assumptions | Generate topology data, separate identity from position, and relocate gameplay behavior |

The source logic already documents the variant semantics in [`edge-variants.md`](../z3-json-data/docs/edge-variants.md). Consume those guarantees; do not introduce a second variant definition or repeat the source validator's checks throughout generation. No changes to `z3-json-data`, `ALTTPRetiling`, or the editor are part of this implementation unless separately requested.

## 1. Settings, pool loading, and saved layout

Add a serde enum for the two map-layout choices, serialized as lowercase strings, and a `map_layout` field to `Settings`. Set `"map_layout": "random"` in the bundled settings. Update direct `Settings` construction sites and any documentation examples. Prototype data may migrate directly; backward compatibility with old settings or seed files is not a requirement.

Change the CLI's initial theme selection to Desert and reject unsupported themes clearly. Keep the existing theme input only insofar as it remains useful to existing tooling; do not add a second theme policy or automatic fallback. Both vanilla and random map modes use Desert during this phase.

Implement the small binary pool reader in the existing Rust code, without a new dependency or a Python bridge. At the file boundary, check the format identifier, declared length, and compatibility of source room identities with the generation catalog. Keep fixed-size records as bytes and decode the selected record directly. Resolve room references by the header's source IDs, not by assuming that pool indices equal logic-catalog indices.

The CLI owns filesystem access; generation receives pool data as an input. Add `map_pool = "maps/map_pool.bin"` to `catalog-build.toml` and `catalog-build.default.toml`, and add the corresponding path to `BuildConfig` and its existing repository-root path resolution. Add `--map-pool <PATH>` to the CLI. The randomizer-owned `maps/map_pool.bin` is the default; generation has no runtime dependency on a sibling `z3-map-gen` checkout.

Follow the existing catalog-loading pattern: an explicit CLI pool path overrides bundled data; bundled builds embed the pool selected by the build configuration; otherwise load the configured pool file. CLI override paths follow normal CLI path handling. Bundled builds must track the configured pool file as a build input so changing it updates the embedded bytes, and must remain runnable without the source pool or `catalog-build.toml` present. The pool is an existing binary input, not an artifact that the catalog builder regenerates with Python. Vanilla generation and patch-only operation do not require loading the pool.

Extend `WorldLayout` with stable source-room identities, their paired-world anchors, and selected variants for all perimeter segments in random mode. Vanilla mode has no selected edge variants: use its vanilla placement, existing connections, and `Main` tiling directly. Continue recording theme choices and retain support for special areas outside the normal shuffled board. Define one authoritative record for a shared border choice in random mode so opposite sides cannot independently select different variants. Store logical choices, not packed ASM records, ROM addresses, or catalog indices. Pool indices may be diagnostic information but cannot substitute for the saved positions.

Choose positions and variants using the caller's seeded RNG with a deterministic iteration order. Repatching consumes the saved layout without RNG. Do not persist redundant adjacency if it can be derived unambiguously from the saved anchors, variants, and catalog endpoint definitions.

## 2. Resolve geometry and edge variants

For random mode, compile the existing source `edgeVariants` into the logic catalog, retaining supported names and ordered local endpoint references. Join these records to source room IDs, dimensions, and their Light/Dark node vertices. Keep the source's special reference kinds, including `null` and `"return"`, explicit.

For a selected pool record, fill the 8×8 occupancy grid using the catalog room footprints. Enumerate each distinct pair of touching perimeter segments once, including each segment separately when two large rooms share an entire side. For every pair, intersect supported variants, remove `closed` if any open choice exists, and randomly select from the remaining choices. Within a candidate, use the same choice on both sides and in both worlds. Set board-exterior segments to `closed`. A catalog mismatch or impossible contact is an input compatibility error, not a reachability retry.

Initially, choose uniformly among the remaining compatible open variants; add no weights, connectivity heuristics, or search/backtracking. Favoring open borders does not ensure reachability, so the full-inventory gate remains necessary. Retrying may repeat a previous combination; avoid adding a history of attempted combinations unless measurements justify it.

Vanilla mode uses vanilla anchors and preserves the existing vanilla connection list built from `transitions`. Use the Desert `Main` layer directly for its map tiling, without evaluating its condition or compositing other layers. Do not select default edge variants or derive any vanilla connectivity from `edgeVariants`. No equivalence verification between vanilla transitions and default variants is required; the two map modes use their respective authored representations.

## 3. Build the selected traversal graph

Keep room-local strats, item-location identities, authored interior connections, and logical event identities independent of placement. Replace randomized overworld boundary connections with connections derived from selected variants, rather than trying to match their vanilla transition spans after moving rooms. Moving a room changes the physical location of its entrances, teleports, whirlpools, and flute spot, but does not itself shuffle their authored logical destination identities.

Use the existing `build_graph` expansion, including follower handling, destination obstacle resets, save-and-quit, event states, and dependency indexing. Extend the connection representation only where specialized border semantics cannot be expressed by the existing bidirectional screen-boundary connection.

| Variant behavior | Graph requirement |
| --- | --- |
| Ordinary land/water openings | Pair ordered opening references in map order, within the same world; preserve the source strats' movement requirements |
| Multiple openings sharing a node | Preserve each opening's endpoint pairing even when local node IDs repeat |
| `closed` | Add no border connection |
| `light-only` | Add the ordinary connection only in Light World |
| `light-and-mirror` | Connect ordinary Light World endpoints and add the documented directed Dark-to-Light Mirror route when its roles exist; do not invent the reverse route |
| Lake `"return"` | Permit entry and return through that opening, with the ordinary room-reset semantics, without exposing the destination room's other nodes |
| Carried-object and forced-exit states | Expand transitions only through endpoints present in the selected variants and preserve existing state compatibility rules |

Audit the existing implicit Mirror edges against specialized variant routes. The same selected endpoints must determine ordinary traversal and event-state traversal; retaining an unused vanilla endpoint would create a route that the ROM cannot provide. Distinguish necessary temporary graph vertices for a return-only opening from persistent source node identities.

## 4. Full-inventory reachability gate and retries

Run this gate after the candidate's graph is built and before `KeyPlacementState::initialize_dungeons` or randomized key assignment. Reuse the normal interpreter, local-state reducers, and discovery closure rather than writing a second reachability engine.

Construct a temporary full-inventory state using the existing preliminary key pass as the starting pattern: grant all configured ordinary inventory, fixed equipment, and prizes; additionally grant every Big Key and access through every Small Key door without assigning Small Keys to pickups. Do not pre-open unrelated bombable/event doors or mark every progress flag complete. Continue discovering events, followers, refills, and Ocarina activation through the normal closure, with the player's enabled tech, notables, and proficiency levels. Health and resource handling follow the existing traversal model. A full-inventory gate is an optimistic check, not a substitute for later progression-safe placement.

Initialize both `reachable_item_locations` and `collected_item_locations` to all false, independently of the assumed inventory. Supply an all-`None` placements array so discovering a location does not grant another item. The existing closure records discovery when the location's collection action can execute, even for an unfilled location. Accept the candidate only when every `reachable_item_locations` entry is true, including ordinary locations, keys, prizes, and fixed pickups. Do not require every authored node or synthetic event-state vertex to be reachable; unused vertices do not invalidate a layout if every item location can be discovered.

Run and inspect a vanilla full-inventory baseline first. Inventory comes from the complete configured item pool, together with the fixed items, prizes, and assumed key access described above; do not grant equipment levels or capacities beyond those supplied by that pool and the normal starting state.

Candidate selection proceeds as follows:

```text
repeat for up to 1,000 complete generation attempts:
    select a pool map
    repeat for up to 10 edge-variant attempts:
        select compatible edge variants on this map
        build the complete selected graph
        initialize full inventory with empty item-location discovery state and no placements
        compute the normal discovery closure from the normal start
        if every item location has been discovered:
            accept this graph and stop retrying edge variants
    if all 10 checks failed:
        continue to the next generation attempt with a new map
    run key assignment with its current inner attempt limits
    if key assignment exhausts its attempts:
        continue to the next generation attempt with a new map
    run one complete item-placement attempt with the current per-wave candidate limit
    if a wave exhausts its candidates or final collectibility fails:
        continue to the next generation attempt with a new map
    return the completed seed
report generation exhaustion if all 1,000 attempts fail
```

Discard all temporary reachability state between candidates. Once accepted, initialize the real key-placement and item-placement state afresh; do not carry full inventory, unlocked doors, or discoveries into real generation. Reuse the accepted graph for both key assignment and normal item placement.

Keep the 10-attempt edge-selection limit as a constant initially. Move random map selection and the discovery gate inside the existing `MAX_GENERATION_ATTEMPTS` loop, whose limit remains 1,000. All 10 edge selections failing consumes one iteration of that loop; it does not consume 10 generation attempts. A later key-assignment or item-placement failure also consumes that same iteration, never an additional charge for an earlier stage that succeeded. There is no separate overall map-selection quota and no nested 1,000-attempt item-placement loop for a selected map.

Preserve the existing inner limits: 1,000 attempts per dungeon with up to 100 candidate batches per key-assignment step, and up to 100 candidates per item-placement wave. Individual key candidates, per-dungeon retries, and item candidates remain internal to their current generation attempt. Exhausted key assignment, an exhausted item-placement wave, or failed final collectibility rejects that map and advances the shared generation-attempt counter. Discard all key assignments, placements, and traversal state from the rejected map. Vanilla mode continues using its existing placement retry behavior on the fixed vanilla graph.

Report map selection, edge attempt number, and concise undiscovered-item-location diagnostics with stable location identities and room/world context. Report whether a map was rejected by the discovery gate, key-assignment exhaustion, or item-placement exhaustion. Distinguish these retryable outcomes from invalid input/catalog errors, which must still propagate instead of being swallowed by the map-selection loop. This should be enough to diagnose rejection without adding a separate diagnostics subsystem.

## 5. ROM identity and physical-position model

The vanilla overworld screen ID currently combines physical grid position, area footprint, graphics, sprite/secret lookup, event behavior, music, and persistence. Separate at least logical area identity, paired-room identity, and physical slot. Source room IDs, vanilla area IDs, and physical slot IDs are different concepts and must have explicit mappings.

### Address audit and retained meanings

The static disassembly audit found 175 direct instruction references to `$8A` and 26 to `$040A` across 18 CPU banks, excluding data constants. These include vanilla paths already replaced by current patches, so the counts are not estimates of required patches. Most consumers select logical-area behavior: graphics, palettes, music, persistent state at `$7EF280`, secrets, entrances, signs, item pickups, and event dispatch. Current generated asset, entrance, cutscene, and item patches also depend on logical identity.

| Address | Retained meaning and lifetime |
| --- | --- |
| `$8A–$8B` | Active logical overworld area ID using stable vanilla IDs, including existing special/loading uses |
| `$040A–$040B` | Actual overworld area-identity copy, preserved across some temporary changes to `$8A` and cached/restored by existing entry paths |
| `$0700–$0701` | Coordinate-derived physical position under Link, used by overworld transition processing; not a persistent area anchor |

Vanilla `HandleOverworldTransitions` already canonicalizes a large area's constituent cells through `ActualOverworldScreenID` and writes the same canonical area ID to `$8A` and `$040A`. Preserve that logical identity, replacing the vanilla physical-cell-to-area lookup with the generated mapping. Do not store the pool's source room IDs directly in these addresses; the patcher maps those identities to the stable vanilla runtime area IDs.

`$040A` is not spare physical-position storage. `InitializeSpriteSlots` compares it with remembered sprite/overlord area ownership; `LoadSpriteProperties` and `InitSpawnedOverlord` record it. Entrance matching, secret selection, and room-specific sprite behavior also use it as identity. Repurposing it would require redirecting those consumers back to logical identity.

Preserve the addresses' existing distinct lifetimes. `ReloadSubscreenOverlay` caches `$8A` at `$7EC213`, temporarily replaces it with a synthetic overlay ID, and restores it afterward; `$040A` retains the actual area identity. The generated BG1 loader already uses the cached real ID. `LoadUnderworldEntrance` caches both addresses separately at `$7EC140` and `$7EC14C`, then clears `$8A`; `LoadCachedEntranceProperties` restores them separately. Special-overworld entry/return similarly uses `$7EC100` and `$7EC10C`. Do not impose an invariant that both addresses always equal each other, or reuse their high bytes as unrelated state: existing code accesses these values as words.

The mixed identity/geometry consumers need targeted changes. `LoadOverworldSprites` uses `$040A` both to select its logical sprite list and to calculate a world-coordinate origin; preserve the former and relocate the latter. `LoadGraphicsAndScreenSize` and camera initialization similarly need generated physical origins while retaining logical graphics and footprint selection. The current `BG2CalculateLogicalWindowOrigin` in `overworld_bg_tilemaps.asm` derives an origin from `$8A`'s coordinate bits; replace that arithmetic with a placement lookup while keeping flat-map/asset indexing logical. Area-specific identity checks can remain even when nearby absolute coordinate constants require relocation.

Continue the broader screen-ID and coordinate audit during implementation, classifying remaining indexed tables, copied values, literal IDs, and coordinate constants by their actual meaning. The direct-reference audit establishes the address strategy, but does not prove every indirect consumer or relocated coordinate correct.

### Generated position mappings

For the normal paired overworld, generate these two byte arrays from the saved layout:

| Table | Index | Value | Size |
| --- | --- | --- | --- |
| `slot_to_area[64]` | Physical cell, 0–63 | Canonical logical Light World area ID | 64 bytes |
| `area_to_anchor[64]` | Logical Light World area ID | Physical top-left cell, 0–63 | 64 bytes |

Both worlds share placement, so the mappings omit the normal overworld world bit `$40`. Strip that bit before indexing and preserve/reapply the world when constructing a world-specific identity or physical screen ID. For example, if logical Light World area `$2C` occupies cell `$15`, its Dark World counterpart `$6C` occupies the corresponding physical screen `$55`, using the same `slot_to_area[$15] = $2C` and `area_to_anchor[$2C] = $15` entries. All four cells of a large area map to the same logical owner; its reverse entry stores only the top-left anchor. Use 64 reverse entries for direct indexing by existing IDs even though only 40 paired-area anchors are meaningful.

These position mappings occupy 128 ROM bytes. Footprint information, selected openings, camera/transition data, special-area records, and asset pointers are separate; 128 bytes is not the size of the entire topology representation. Keep existing logical footprint data where it remains valid. Derive physical origins from anchors and distinguish internal movement within a large area from crossing into another logical area. Closed borders must not transition even if Link reaches a boundary coordinate.

Do not add another persistent RAM word initially. Physical placement is immutable for a seed and can be looked up from the logical area being processed, including a destination being loaded before Link finishes crossing. `$0700` is refreshed from Link's coordinates during normal overworld processing and identifies his current cell, which may be an interior quadrant of a large area; it is not valid as an area-anchor cache at every loading stage. Keep its existing role. A physical-origin cache can be proposed later if measured performance justifies the added state and synchronization requirements.

### Special overworld areas

Special overworld areas stay outside the shuffled 8×8 grid. Preserve their existing special-area IDs, internal terrain/object arrangement, coordinate systems, and camera setup. They do not consume cells in the map pool or entries in the two normal-overworld mapping arrays. Some IDs in the special range identify synthetic overlays rather than independently visitable areas, so preserve that distinction as well.

Determine whether the active context is a normal area or a special/loading context before applying the normal lookup or masking IDs with `$3F`. Blindly masking a special ID into the normal range would incorrectly resolve an unrelated shuffled area. Preserve existing special-area Mirror restrictions and loading behavior; do not treat special areas as ordinary Light/Dark pairs merely because their IDs contain bit `$40`.

| Part of a special-area connection | Handling |
| --- | --- |
| Entry trigger in a normal area | Move with its logical owner and retain its area-relative trigger position |
| Destination inside the special area | Retain its authored special-area identity, coordinates, and camera setup |
| Special area's internal objects, events, and pickups | Retain logical ownership and their existing internal positions |
| Cached return to the normal overworld | Restore the relocated parent's cached identity, Link coordinates, and camera state |
| Table-driven normal-overworld destination | Relocate the destination explicitly to its logical parent's generated position |

For example, moving Lost Woods moves its entry to the Master Sword grove, while the grove retains its special identity and internal layout. `LoadSpecialOverworld` already caches the originating area IDs, Link coordinates, and camera properties; `LoadOverworldFromSpecialOverworld` restores them. When those values were captured on the relocated normal area, they already identify the correct return position. This does not eliminate the need to relocate destinations provided by fixed exit records or to verify nested special-area transitions and their cache lifetimes.

Normal-to-special entry, special-to-normal return, nested special transitions, and special-area pickups must agree between logic and ROM behavior. Retaining separate coordinate spaces and the existing return caches does not require an additional persistent physical-position word.

Generate compact runtime lookup data from `WorldLayout`: slot ownership, area anchor and footprint, local quadrant, counterpart mapping, camera bounds, and permitted neighboring transitions. Lookup must distinguish movement within a large area from crossing into another logical area. Replace vanilla arithmetic in size resolution, screen transitions, and coordinate/camera setup where it assumes the vanilla arrangement. Closed borders must not transition even if Link can reach the boundary coordinate.

Matched edge openings should preserve the coordinate along their border. Keep authored coordinates relative to the logical area and perform relocation in Rust where possible. Add runtime coordinate transforms only where actual engine behavior requires them.

## 6. Desert assets, transitions, and gameplay relocation

For vanilla map tiling, select only the Desert `Main` layer and compile its artwork and collision directly. Do not evaluate conditions or composite any other layers onto this tiling.

For random map tiling, retain each source layer's `condition` in the retiling catalog and evaluate the existing expression language against saved edge choices. Reuse its documented slots, aliases, and composition order rather than defining new names. Compile BG2 artwork and collision from the same selected layers. Ensure event/BG1 layers retain their current interpretation. Do not change the artist-owned source data as part of this work.

Replace `build_scrollable_transition_pairs`' vanilla adjacency list with pairs derived from the resolved layout, including specialized world-specific openings. Newly adjacent areas affect palette coexistence, character allocation, and asset transfer scheduling. Existing hardware budgets and failure reporting still apply; do not silently drop artwork or change transition style when an allocation fails.

Move existing per-area assets with their logical owners: maps, BG1 backgrounds, animation tracks, entrance lists, special transitions, cutscenes, and weather/background settings. Replace current vanilla-ID dispatch where it actually means location. Some cutscene/overlay code may retain vanilla IDs as stable identities if the chosen runtime identity model makes that correct; changing every numeric ID indiscriminately is unnecessary.

Relocate the remaining gameplay data using original area-relative coordinates:

| Data or behavior | Relocation work |
| --- | --- |
| Sprites and overlords | Move spawn positions, state variants, proximity loading, graphics selection, and persistent spawn state with the logical area |
| Entrances and holes | Move detection lists and all exterior return records, including Link coordinates, camera coordinates, and door-animation positions |
| Overworld pickups and secrets | Preserve item identities while moving pickup positions, dig/bonk prizes, signs, and interaction targets |
| Dynamic tiles and events | Preserve event ownership and persistence while moving overlays, opened entrances, and changed tiles |
| Mirror and portals | Resolve the paired world's corresponding position and preserve area-relative landing coordinates |
| Flute and whirlpools | Move destinations and camera setup while retaining authored logical destinations and existing flute numbering initially |
| Spawn, save/reload, save-and-quit | Resolve Link's House and saved logical destinations at their selected physical positions |
| Special overworlds and returns | Preserve special-area identities while moving the ordinary-overworld endpoints they reference |
| Music, weather, and area-specific rules | Follow logical ownership unless the screen-ID audit establishes that a rule genuinely belongs to physical position |

The invariant is that entering an authored interior and returning outside reaches the correct relocated exterior, with matching logic and ROM behavior. Moving entrance detection without moving the return path is insufficient. Likewise, persistence must not become attached to whichever area happens to occupy a vanilla slot.

## 7. Mode 7 maps and presentation

Generate Light and Dark World map art from the final placements and edge choices. Move Link, dungeon/objective, portal, and flute markers consistently. Keep this presentation pipeline distinct from playable-overworld tiles while deriving it from the same saved layout.

Use the authored map grids for normal areas, with `Main` alone in vanilla mode and the selected edge layers in random mode. Use the regular `Main` tile grids of `Light World Clouds` and `Dark World Clouds` as the respective full-map backgrounds, then overlay the normal areas in the central region. Unpainted cloud tiles and the fill beyond the map remain blank until those assets are populated; do not import vanilla map artwork as a fallback.

Retain authored flute location numbers initially; geographic renumbering is not part of this change. Confirm flute selection, landing, map zoom/pan, menu exit, and restoration of playable graphics and palettes. A ROM with a vanilla-looking map over a random layout is only an intermediate development checkpoint.

## Implementation milestones and verification

The implementation covers pool selection and saved borders, configured-inventory item discovery, the shared attempt budget, conditional Desert artwork, logical-ID-preserving ROM topology, relocated exterior returns/transport destinations and area-specific coordinates, and authored Mode 7 maps. The physical-cell ownership table uses the native 64-byte table at `$82A4E3`, and logical-area anchors use 64 bytes at `$BFE540`. Map graphics occupy bank `$CA`; the two full tilemaps occupy `$CB8000–$CB9FFF`, and their palettes replace the native map palettes. Allocation failures remain patch-time errors.

Development checks cover a vanilla discovery baseline, rejection with reduced configured inventory, acceptance of an unused unreachable vertex, rejection of an unexecutable collection action, malformed pool headers/length, deterministic layouts and items, configured and explicit pool paths, bundled operation outside the repository, and pool-independent vanilla generation and saved-seed patching. Repatching the same saved seed produces identical ROM bytes. A temporary cloud-art check verifies that authored cloud placements appear while missing cloud tiles and fill remain blank. Emulator smoke checks on a saved random layout cover relocated house exit, all four ordinary transition directions with logical IDs retained, flute selection and landing in a relocated large area, both world maps, map zoom/exit, and restoration of playable graphics. These checks do not replace the broader transition, special-area, transport, event, persistence, and completion-playthrough checkpoints below. Temporary test code has been removed.

1. **Settings, pool reader, and seed representation.** Add the configured `map_pool` path with default `maps/map_pool.bin`, the `--map-pool` CLI override, and bundled-pool handling. Decode the supplied pool in Rust, select reproducible records, serialize resolved layout choices, and retain vanilla mode. Confirm CLI override precedence and that patch-only code has no pool dependency. Do not make random the shipped default until the complete generation/patching path supports it; the final bundled default must be random.
2. **Variant-aware logic and reachability.** Compile source variants, resolve selected connections, implement specialized semantics, and implement the every-item-location discovery gate using the complete configured item pool and the same-map retry loop. Establish the vanilla reference before drawing conclusions from random-map rejections.
3. **Item-placement integration.** Enclose map selection, the discovery gate, key assignment, and one complete item-placement attempt in the existing 1,000-attempt generation loop. Preserve inner key/candidate limits and move to a new map on any failed generation attempt. Confirm full-inventory state never leaks into real placement, failed-map state is discarded, and the existing all-items collection requirement remains enforced.
4. **Desert compilation for both modes.** Use `Main` directly for vanilla map tiling, with no condition evaluation or additional layer composition. For random mode, resolve the saved border choices into matching artwork and collision, and derive allocation constraints from actual adjacency. Verify every supported connection family and important combined corner layers without changing artist-owned source files; no vanilla/default-variant equivalence check is needed.
5. **Vanilla layout through generated ROM topology.** Retain logical identity in `$8A`/`$040A`, generate the normal position mappings without a new persistent RAM word, and update geometry consumers. Continue the broader screen-ID/coordinate audit and reproduce vanilla movement and entry paths, including special-area entry/return, before moving rooms.
6. **Fixed relocated layout.** Exercise small and large rooms at varied positions, all approach directions, paired-world travel, and the gameplay data above using one saved hand-selected rearrangement. Use this stable seed to debug relocation independently of randomized selection.
7. **Pool-selected playable layouts.** Connect generation to ROM patching, cover specialized edge behavior, and validate multiple saved layouts with representative player settings. Complete Mode 7 maps and markers, then enable the final random default.

Use temporary focused probes and existing build/check commands; remove temporary verification files before completing each implementation task. Do not add permanent tests without a separate request. Useful checks include seeded selection reproducibility, saved-layout round trips, directed variant semantics, discovery rejection/retry counts, separation of full-inventory and real-generation state, and agreement between graph connections and rendered openings. Verify that assumed item ownership does not pre-discover locations, that a reachable vertex with an unexecutable collection action fails the gate, and that an unused unreachable vertex does not cause rejection when every location is discoverable. Repatch the same saved seed to confirm patching does not consume RNG or access the pool.

ROM checkpoints require emulator verification of ordinary transitions, multi-slot internal movement, dashing, carried objects and followers, entrances/returns, Mirror and portals, flute/whirlpools, save/reload, dungeon openings, persistent changes, and map-screen restoration. Finish with a completion playthrough; passing the full-inventory gate alone cannot establish progression-safe or engine-correct playability. Follow the project's register/scratch preservation, balanced-stack, free-space, and exported-symbol conventions for all ASM hooks.

Additional themes, edge weights, connection search/backtracking, independent world layouts, entrance shuffling, geographic flute numbering, new object-placement overrides, and backward-compatibility machinery are outside the initial scope.
