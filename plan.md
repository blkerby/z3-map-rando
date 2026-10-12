# Logical item placement implementation plan

Status: implementation plan. The placement strategy and the resource, fairy-exchange, and retry policies below reflect the decisions made in discussion.

## Agreed behavior

- Keep the existing uniform randomizer as the `no-logic` setting. Logical mode uses player tech and proficiency settings.
- Logical mode must make every catalog item location collectible, including fixed rewards, by executing its collection action rather than merely reaching its associated vertex.
- Save and quit is always available and returns to the catalog's starting vertex while retaining collected inventory and persistent progress. Do not require an escape route from a pickup.
- Small and Big Key pickups stay in their vanilla locations. Each small key receives a persistent identity that unlocks exactly one same-dungeon small-key door. Assume the small-key pickup count matches the small-key door count; the missing Ganon's Tower key will be supplied upstream.
- Assign small-key identities in a preliminary pass, processing one dungeon at a time. Traverse the whole world, following its directed connections, while treating other dungeons' locks and non-key requirements as free. Enforce the selected dungeon's small-key doors and Big Key requirements.
- Assign all unassigned collectible small keys as a batch, so a candidate can open several doors before exposing another key. Do not demand progress from each individual key assignment.
- If a dungeon's preliminary attempt fails, restart only that dungeon. Retain successfully completed assignments for other dungeons. Stop with failure after a bounded number of attempts on a given dungeon; use a constant of 1000.
- During main placement, place a dungeon's Map and Compass in collectible eligible locations before placing any other randomized ordinary rewards there. Keep keys and prizes in their separate location classes.
- Sample candidates from the complete remaining ordinary pool for the newly collectible ordinary locations. There is no separate progression-only fill followed by junk fill.
- A main-placement candidate succeeds when it exposes at least one additional collectible item location, or when all required locations are collectible. Retry failed candidates and restart a failed main-placement attempt without backtracking.
- Treat rupee costs as free. Do not track rupee balances in traversal state.
- Initialize health using the game's capacity-dependent spawn-health table. Magic, bombs, and arrows initially start empty. Once a resource refill is logically obtained, remember that resource in global state so subsequent traversals can start with it full at the current capacity. Resource costs still apply along a route; arriving at another vertex does not replenish resources.
- Only environmental refill sources establish this global preparation permission. Ignore the resource-restoring effects of one-time refill rewards in logical reachability; retain their actual rewards and placement slots. Permanent capacity upgrades still affect logic.
- Track bottles owned and logically established reusable bottle-refill types globally. Local traversal tracks remaining bottle uses without committing each bottle to a particular content. Consuming a bottle chooses any established content type and spends one use; a Blue Potion spends one use while restoring both health and magic.
- Keep fixed fairy exchanges in the all-items guarantee. Treat `shieldExact` requirement as a regular `shield` requirement; revisit this if fairy-exchange behavior changes.
- Bound candidate retries, per-dungeon key-assignment attempts, and main-placement attempts. Report generation failure when a budget is exhausted.
- Postpone ROM support for door-specific keys and optimization through a reduced preliminary graph.

## Settled resource, fairy-exchange, and retry policies

### Resource preparation

Follow the game's starting-health behavior and start the other resources empty until their first logical environmental refill. Thereafter, a global refill-discovery bit permits starting future traversals with that resource full. Bottle preparation uses the separately agreed global set of reusable content types. One-time resource rewards do not establish refill discoveries and their finite amounts are ignored by the logical model, so there is no shared finite-pickup or spending history to track.

For example, `ThreeBombs`, `TenArrows`, `Heart`, and `SmallMagic` remain rewards in the placement pool but add no logical resource amount. Heart containers, heart pieces, ammunition-capacity upgrades, magic upgrades, and new bottles still increase their permanent capacities; ignore any accompanying one-time refill or initially supplied bottle contents. Only a logically usable reusable bottle-content source expands the set of flexible bottle contents.

Use [`MaxHealthBasedSpawnHP`](jpdasm/bank_09.asm:21348), indexed by maximum whole hearts, for health before a health refill is known. For capacities of 3 through 20 hearts, its starting-heart values are `3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10`. The ROM table stores these values in eighths of a heart. New-game capacity is three hearts; initial capacities for the other resources follow vanilla defaults and acquired upgrades.

### Fixed fairy exchanges and the all-items guarantee

The current generator keeps the Waterfall and Pyramid fairy exchanges fixed. Preserve those placements and include their locations in the all-items guarantee. The Waterfall shield exchange currently compiles to `Never` because exact shield requirements and the free-upgrade helper are disabled. Compile that exchange's `shieldExact: 1` condition as a minimum level-one shield requirement so its fixed upgraded shield can be collected. Since this is the only purpose of `shieldExact` this can be implemented in how `shieldExact` is processed in general.

Retain the current conservative treatment of other exact equipment and negative-flag requirements. Add a nearby explanation that the exception must be revisited if fairy exchanges or shield availability are changed. Do not enable the free-upgrade helper globally or remove the exchange from the required-location set.

### Retry termination

Use separate internal budgets for candidates within a wave, attempts for the current dungeon, and complete main-placement attempts. Candidate exhaustion restarts only the current dungeon in the preliminary phase, or the current main-placement attempt in the main phase. Exhausting the relevant outer budget returns a generation failure through the existing `Result` API. Preserve successfully settled earlier dungeons throughout preliminary retries and preserve all settled key assignments throughout main retries.

Choose initial budget constants during implementation and tune them with representative generation timings; no new user-facing settings are necessary for the first version. A failure report should identify the phase, selected dungeon when applicable, exhausted budget, and remaining inaccessible locations or lack of progress. Distinguish a failed randomized attempt from a proof that the layout has no solution.

## Current code and scope

[`generator::generate`](crates/generator/src/lib.rs) currently classifies locations, shuffles prizes, places Maps and Compasses within their source dungeon, shuffles the ordinary pool, and constructs `SeedData`. [`LogicCatalog`](crates/logic_catalog/src/lib.rs) provides authored edges, recursive requirements, effects, typed indices, the start vertex, and separate connection records; it does not implement traversal. [`Settings`](crates/seed_data/src/settings.rs) has no placement-mode setting, and [`SeedData`](crates/seed_data/src/output.rs) has no small-key identity assignments.

Implement the first version inside the generator crate, splitting graph preparation, traversal, and placement into modules. Keep the catalog immutable; use catalog indices for generation and stable authored identities for saved seeds. No new crate or general graph framework is needed.

Use vanilla world connections initially. Keep connection expansion separate from traversal so future randomized maps can supply another connection set. Do not implement map randomization, multiple restart destinations, key receipt patches, or door-binding patches in this work.

The logical model will use door-specific keys before the ROM does. Saved logical placements can be inspected and solved under that model, but the existing patcher will still produce vanilla key behavior. The eventual ROM-playability guarantee depends on the deferred key patches and any other gameplay changes assumed by the catalog.

## Implementation steps

### Step 1: Add placement mode and saved key assignments

Add a placement-mode setting with `no-logic` and logical variants, and make the selected mode explicit in bundled settings. Preserve the current uniform shuffle as its own generation branch. Both branches produce the existing item-placement records and world layout; only logical mode runs the traverser and assigns key identities.

Introduce a stable authored door identity consisting of source room ID and authored door ID. Save each small-key assignment as the vanilla pickup's `ItemLocationId` paired with its assigned door identity. Keep the pickup's saved item name `SmallKey`, so this phase does not require changing the patcher's receipt encoding. No-logic seeds have no generated small-key assignments.

Resolve item names and stable tech IDs to catalog indices once when preparing logical generation. Build each dungeon's fixed key pickup lists, small-key door list, Map/Compass pool, prize locations, ordinary locations, and fixed exchange locations. Derive dungeon ownership from the catalog's source room, which represents the whole authored dungeon. Retain the existing ordinary-pool count behavior and adjust bundled pool quantities if upstream changes make that necessary.

Completion: the current generator can still produce no-logic seeds, and logical generation has the indexed data and saved representation needed by later steps. Do not add backward-compatibility machinery.

### Step 2: Enable the fixed Waterfall shield exchange in the catalog

Apply the narrowly scoped minimum-shield interpretation while compiling the Waterfall shield exchange's requirement. Keep its stable location identity and fixed reward. Document why this exception applies and when it must be revisited. Rebuild the logic catalog so normal traversal sees the approved monotonic requirement rather than an impossible action.

Completion: the fixed exchange can be represented in the all-items solve without enabling unrelated exact-equipment routes or fairy helpers.

### Step 3: Expand the selected connections into a playable graph

Build outgoing adjacency for catalog edges plus generation-owned connection edges. Expand doors bidirectionally, drops only inward, teleports only outward, and screen boundaries and whirlpools bidirectionally. The destination vertex determines the world; do not add an independent world state to interior vertices.

Apply entrance follower rules and reset room-local obstacles when leaving their owning source room. Match event-exit and event-entry vertices only across the selected physical connection with the same event identity. Do not attach event-entry actions to ordinary endpoint vertices.

Add flute travel using the catalog's destinations and the active-flute capability. Keep save-and-quit behavior available to both traversers. Normal traversal returns to the start with persistent discoveries retained, follower and room obstacles reset, and resource preparation governed by the global refill-discovery bits and spawn-health rule. The simplified traverser can restart directly at the start because it has no resources or followers.

Completion: both traversers can use the same directed world topology, including routes that leave and reenter a dungeon through another entrance.

### Step 4: Implement the simplified key-pass traversal

Use a vertex work queue, a visited set for the current key inventory, and discoveries of the selected dungeon's key pickups and item collection actions. There is no resource, bottle, follower, or obstacle frontier in this mode.

Evaluate requirement composition while treating non-key leaves as free. Enforce a selected-dungeon small-key door through its candidate assignment, and a selected-dungeon Big Key door through collection of its vanilla Big Key. Treat other dungeons' doors and ordinary `Item(BigKey)` collection requirements as free. Ignore temporary-state effects; preserve collection discoveries. An unassigned small-key pickup is collectible but enables no door until a candidate assigns it an identity.

Run to closure after discovering a usable key, restarting or extending traversal as needed. Report collected small-key locations, Big Key availability, and collectible item locations in the selected dungeon. Do not seed every dungeon entrance as independently reachable.

Completion: the key pass can ask which target-dungeon keys and items are collectible for a partial candidate assignment.

### Step 5: Assign one dungeon's small-key identities at a time

For the current dungeon, start with no assignments and run the simplified traverser. Gather its collectible, unassigned small-key pickups and sample a distinct unassigned door for each pickup in the candidate batch. Recompute closure with those assignments. Accept a candidate if another small key becomes collectible or every item location in that dungeon becomes collectible; otherwise discard the candidate without altering committed assignments.

Continue with the newly collectible unassigned keys. Once every dungeon item is collectible, randomly pair any remaining unassigned key pickups and doors. Those keys are already collectible without their new doors, so these final assignments cannot introduce a key-collection cycle. The result assigns an identity to every small-key pickup and door.

If an attempt cannot continue, discard only the current dungeon's assignments and restart that dungeon within its attempt budget. Never discard successfully settled previous dungeons. Other dungeons' locks remain free regardless of their saved assignments. Freeze all successful assignments before main placement.

Completion: every dungeon has a complete key assignment whose keys and item locations are collectible under the preliminary pass's relaxed requirements.

### Step 6: Define normal global and local state and item grants

Global state contains permanent item capabilities and equipment levels, health and resource capacities, heart-piece accumulation, bottles owned, known reusable bottle-content types, discovered environmental health/magic/bomb/arrow refills, collected location identities, permanent flags, dungeon-scoped Big Keys, acquired door-specific small keys, and persistently opened non-key doors. One-time refill rewards add no logical resource amount or refill-discovery bit.

Local state contains current health, magic, bombs, arrows, remaining flexible bottle uses, the current follower, and the owning room's obstacle states. Omit rupees. Keep resource quantities together in actual route states rather than taking independent maxima.

Implement reward grants for the supported item definitions. Equipment uses the highest acquired level, bow and silver-arrow capabilities remain distinct, flute activation supplies the active capability without removing the inactive capability, and capacities derive from acquired upgrades. Collect each location once. Resolve fixed key grants using the location's dungeon and saved assignment rather than the anonymous item name alone.

Use eighths of a heart for health and the catalog's normalized magic units. Account for potions and Fairy revival through the flexible-bottle model. Distinguish a one-time filled-bottle reward from a reusable refill source; buying or finding one potion does not itself establish an unlimited supply.

Completion: applying a reward produces the capacities and capabilities needed by requirement evaluation. Route initialization follows the spawn-health table and known environmental refill bits.

### Step 7: Interpret normal requirements and effects

Implement each `Requirement` variant over a fixed global snapshot and local input state. Execute `And` children in order; evaluate each `Or` branch from the same input and retain meaningful successful output states. Honor tech enablement, proficiency thresholds, prize counts, followers, obstacles, and doors. Preserve `Never` in normal traversal.

Apply ammunition and magic costs, damage, refills, and resource-state checks to the local state. Process magic uses and damage hits individually so potions and Fairy revival can intervene. A bottle consumption may choose an established refill content and spends one shared bottle use. A Fairy refill can heal immediately; storing it for later use must honor capture requirements, including the net where applicable. Rupee payment and refill operations have no balance effect.

Apply strat effects after successful requirements. Temporary obstacle and follower effects update the local state. Collection, fixed item grants, flags, persistent door openings, resource refills, and newly established reusable bottle refill types become global discoveries. Record a refill discovery only after its action is logically obtainable. Do not grant a hypothetical item from an unfilled location: report that the collection action is executable so the filler can use the slot.

Completion: the interpreter supports the catalog's action semantics and returns successful states plus global discoveries.

### Step 8: Traverse normal routes and compute discovery closure

Use a work queue of vertex/local-state pairs. Store multiple compatible local states at a vertex. Begin with exact deduplication and conservative resource dominance between states with matching temporary state; retain alternatives when obstacle, follower, or resource combinations differ. Never combine unrelated states into an invented route.

Run a traversal wave with a fixed global snapshot, gather discoveries, apply the guaranteed discoveries, and repeat until no additional global capabilities, flags, door openings, or refill types are discovered. Global discoveries may come from fixed placements, committed randomized placements, fixed events, or successful refill actions. Each new traversal starts with the known refilled resources full at their current capacities; others start empty, except health uses its spawn table until a health refill is known.

Keep inventory dependent continuations for the next wave; a successful collection effect does not let the old snapshot immediately use the new item. Track the provenance of discoveries and successful collection actions sufficiently to explain inaccessible locations during development.

Return the collectible location set and executable unfilled collection actions. Do not use vertex reachability as a substitute for collection. Postpone a bounded metric frontier until measurements show a need; adding one would require an explicit decision about lost completeness.

Completion: given a partial placement, the filler can obtain all currently collectible slots and permanent discoveries under normal logic.

### Step 9: Prepare main placement and apply Map/Compass priority

Start each main attempt with the frozen small-key assignments, fixed vanilla key rewards, fixed fairy rewards, and a fresh shuffle of pendants and crystals among prize locations. Preassign prizes using the current placement restriction, then let normal traversal collect them when their boss reward action is executable. Do not assume possession of a prize merely because it is assigned.

Traverse committed placements to closure. For newly collectible eligible ordinary locations in each dungeon, place that dungeon's outstanding Map and Compass first, choosing their slots with the RNG. If only one slot is collectible, place one and carry the other forward. Ordinary pool rewards may use that dungeon's remaining collectible slots only once both dungeon items are placed.

Treat these forced dungeon-item placements and the sampled ordinary rewards as the complete candidate for the wave. Account for all pool quantities without pre-reserving arbitrary inaccessible dungeon slots.

Completion: candidate construction honors Map/Compass priority, fixed-key locations, and prize-only placement restrictions.

### Step 10: Sample, accept, and retry placement waves

Sample without replacement from the remaining ordinary pool to fill every other newly collectible ordinary slot. Evaluate the full candidate with normal traversal to closure; items that only help as a combination can therefore succeed together.

Accept the candidate when its collectible-location set includes at least one previously uncollectible required item location, or when all required locations are collectible. On acceptance, commit its placements and pool removals. On rejection, leave the committed placements, remaining pool, and global state unchanged, then sample another candidate. Advance RNG state normally so retries do not repeat the same candidate.

If the attempt cannot continue, restart main placement with its ordinary placements and prize shuffle cleared and its Map/Compass pools restored. Retain the settled dungeon key assignments. No placement backtracking is needed. Enforce the agreed candidate and outer-attempt budgets and report failure when the outer budget is exhausted.

Completion: logical mode can construct a complete assignment through advancing waves and restarts.

### Step 11: Finish the logical solve and integrate output

Solve the completed placements from the agreed starting state with all dungeon key restrictions, player settings, normal resource semantics, and save and quit enabled. Require every catalog item location to have an executable collection action, including fixed keys, prizes, and fairy exchanges. This completed solve is part of logical generation's acceptance algorithm, not a separate generic validation layer. An incomplete solve makes the main placement attempt unsuccessful.

Return the saved placement mode, complete reward placements, stable small-key assignments, existing metadata, and existing vanilla world layout. Keep generation free of ROM and filesystem access. Keep patch-only operation driven by saved data; defer applying generated key identities to the ROM.

Update generator API comments and user-facing documentation to explain the two placement modes and the deferred ROM key behavior. Reconcile the older strategy in [plans/keys.md](plans/keys.md) and the traversal discussion in [plans/logic.md](plans/logic.md) when implementation lands.

Completion: callers can choose no-logic or logical generation, and saved logical seeds contain the decisions needed to reproduce the logical model.

## Development verification and deferred work

Follow the repository's instruction against adding permanent tests, assertions, validation branches, or unrelated new error paths without an explicit request. The bounded-retry failure report is explicitly requested. Use temporary developer probes when implementing each step, and remove them before completing the changes. Existing compilation and test commands can be run as appropriate; they do not authorize adding new permanent checks.

Useful temporary probes include the two-collected-keys/two-doors case, a route that leaves and reenters a dungeon, a pickup with a collection requirement distinct from its vertex, sequential resource costs across an `Or`, two flexible bottles used for different contents, Map/Compass priority across two waves, and the final-wave acceptance exception. Inspect representative generated placements and their discovery histories after integration.

Deferred: actual door-specific ROM key behavior; Thieves' Town's asymmetric reverse entrance lock beyond the current catalog model; randomized map connections; alternative restart destinations; reduced key-pass graphs; metric-limited traversal frontiers; placement balancing and progression pacing; and other gameplay patches assumed by the logic catalog.
