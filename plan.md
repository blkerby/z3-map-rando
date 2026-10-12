# Logical item placement implementation plan

Status: Step 1 implemented; Steps 2 through 11 remain planned. The placement strategy and the resource, fairy-exchange, and retry policies below reflect the decisions made in discussion.

## Agreed behavior

- All generation uses logical item placement with player tech and proficiency settings.
- Generation must make every catalog item location collectible, including fixed rewards, by executing its collection action rather than merely reaching its associated vertex.
- Save and quit is always available and returns to the catalog's starting vertex while retaining collected inventory and persistent progress. Do not require an escape route from a pickup.
- Small and Big Key pickups stay in their vanilla locations. Each small key receives a persistent identity that unlocks exactly one same-dungeon small-key door. Assume the small-key pickup count matches the small-key door count; the updated Ganon's Tower source now has eight small-key pickups and eight small-key doors.
- Represent placeable rewards with a shared `Item` enum containing an explicit variant for each supported pickup, rather than every source receipt or inventory state. Store the reward directly in `ItemPlacement.item`; `Item::SmallKey { door_id: DoorId }` carries the assigned door identity. Persist key assignments within `SeedData.item_placements`, with no parallel assignment array.
- Use `Item::Sword`, `Item::Shield`, `Item::Glove`, `Item::Mail`, and `Item::Boomerang` with no tier payload. Each collected copy advances the corresponding equipment level by one, up to the game's maximum, regardless of collection order. Mail progresses from the starting Green Mail to Blue Mail and then Red Mail; Boomerang progresses from none to Blue and then Red. Model these progressive grants in logic and defer their ROM behavior.
- Use `Item::Ocarina` for the unactivated pickup and track activation as a permanent inventory upgrade from the bird event. Use `Item::SilverArrows` for the arrow upgrade independently of Bow ownership.
- Start without a shield. Two progressive Shield rewards come from the ordinary pool and one from the fixed Waterfall fairy exchange, providing the complete progression from no shield to Blue, Red, and Mirror Shield. The fairy exchange requires at least a Blue Shield and is collected once.
- Assign small-key identities in a preliminary pass, processing one dungeon at a time. Traverse the whole world, following its directed connections, while treating other dungeons' locks and non-key requirements as free. Enforce the selected dungeon's small-key doors and Big Key requirements.
- Assign all unassigned collectible small keys as a batch, so a candidate can open several doors before exposing another key. Do not demand progress from each individual key assignment.
- If a dungeon's preliminary attempt fails, restart only that dungeon. Retain successfully completed assignments for other dungeons. Stop with failure after a bounded number of attempts on a given dungeon; use a constant of 1000.
- During main placement, place a dungeon's Map and Compass in collectible eligible locations before placing any other randomized ordinary rewards there. Keep keys and prizes in their separate location classes.
- Sample candidates from the complete remaining ordinary pool for the newly collectible ordinary locations. There is no separate progression-only fill followed by junk fill.
- A main-placement candidate succeeds when it exposes at least one additional collectible item location, or when all required locations are collectible. Retry failed candidates and restart a failed main-placement attempt without backtracking.
- Treat rupee costs as free. Do not track rupee balances in traversal state.
- Initialize health using the game's capacity-dependent spawn-health table. Magic, bombs, and arrows initially start empty. Once a resource refill is logically obtained, remember that resource in global state so subsequent traversals can start with it full at the current capacity. Resource costs still apply along a route; arriving at another vertex does not replenish resources.
- Only environmental refill sources establish this global preparation permission. Ignore the resource-restoring effects of one-time refill rewards in logical reachability; retain their actual rewards and placement slots. Permanent capacity upgrades still affect logic.
- Track bottles owned and logically established reusable bottle-refill types globally. Local traversal tracks concrete contents, including empty bottles. Prepare possible fillings from known reusable sources at traversal initialization; consumption uses an actual stored content and leaves an empty bottle. In-route refills supply only their specified content.
- When defining future cost metrics, prefer Blue Potions over all other contents and Red Potions over Fairies. These are cost preferences, not unconditional reachability dominance: retain Fairy states when automatic revival is necessary.
- Keep fixed fairy exchanges in the all-items guarantee. Compile every `shieldExact` requirement as the corresponding minimum-level `shield` requirement; revisit this if fairy-exchange behavior or shield availability changes.
- Migrate the prototype seed representation directly without a format-version update or backward-compatibility machinery.
- Bound candidate retries, per-dungeon key-assignment attempts, and main-placement attempts. Report generation failure when a budget is exhausted.
- Postpone ROM support for door-specific keys and optimization through a reduced preliminary graph.

## Settled resource, fairy-exchange, and retry policies

### Resource preparation

Follow the game's starting-health behavior and start the other resources empty until their first logical environmental refill. Thereafter, a global refill-discovery bit permits starting future traversals with that resource full. Bottle preparation uses the global set of reusable content types to construct concrete local filling alternatives; with no known source, bottles start empty. One-time resource rewards do not establish refill discoveries and their finite amounts are ignored by the logical model, so there is no shared finite-pickup or spending history to track.

For example, `ThreeBombs`, `TenArrows`, `Heart`, and `SmallMagic` remain rewards in the placement pool but add no logical resource amount. Heart containers, heart pieces, ammunition-capacity upgrades, magic upgrades, and new bottles still increase their permanent capacities; ignore any accompanying one-time refill or initially supplied bottle contents. Only a logically usable reusable bottle-content source expands the global set of preparation types. A Green Potion refill encountered on a route can fill empty bottles with Green Potions; it cannot manufacture another known content such as a Fairy.

Use [`MaxHealthBasedSpawnHP`](jpdasm/bank_09.asm:21348), indexed by maximum whole hearts, for health before a health refill is known. For capacities of 3 through 20 hearts, its starting-heart values are `3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10`. The ROM table stores these values in eighths of a heart. New-game capacity is three hearts; initial capacities for the other resources follow vanilla defaults and acquired upgrades.

### Fixed fairy exchanges and the all-items guarantee

The current generator keeps the Waterfall and Pyramid fairy exchanges fixed. Preserve those placements and include their locations in the all-items guarantee, converting any sword, shield, or boomerang reward to its progressive variant. The Waterfall boomerang reward contributes one progressive upgrade, collected once after obtaining a boomerang. The Waterfall shield exchange currently compiles to `Never` because exact shield requirements and the free-upgrade helper are disabled. Compile `shieldExact: n` as the same minimum-level shield requirement as `shield: n` everywhere it appears. This makes the exchange's `shieldExact: 1` branch usable without a location-specific override. Progressive shields supersede the earlier rationale that the upgraded shield was obtainable only from this exchange; retain the agreed minimum-level interpretation and revisit it if exact shield levels become necessary for an action.

Retain the current conservative treatment of other exact equipment and negative-flag requirements. Document the general shield interpretation beside its compiler implementation and revisit it if fairy exchanges or shield availability are changed. Do not enable the free-upgrade helper globally or remove the exchange from the required-location set.

### Retry termination

Use separate internal budgets for candidates within a wave, attempts for the current dungeon, and complete main-placement attempts. Candidate exhaustion restarts only the current dungeon in the preliminary phase, or the current main-placement attempt in the main phase. Exhausting the relevant outer budget returns a generation failure through the existing `Result` API. Preserve successfully settled earlier dungeons throughout preliminary retries and preserve all settled key assignments throughout main retries.

Use 1000 attempts per dungeon for preliminary assignment. Choose the other initial budget constants during implementation and tune them with representative generation timings; no new user-facing settings are necessary for the first version. A failure report should identify the phase, selected dungeon when applicable, exhausted budget, and remaining inaccessible locations or lack of progress. Identify cases where mandatory Map/Compass placements consume the available slots, rather than describing every failure as an unlucky ordinary-item sample. Distinguish a failed randomized attempt from a proof that the layout has no solution.

## Current code and scope

[`generator::generate`](crates/generator/src/lib.rs) currently classifies locations, shuffles prizes, places Maps and Compasses within their source dungeon, shuffles the ordinary pool, and constructs `SeedData`. Replace its uniform placement algorithm with logical placement. [`LogicCatalog`](crates/logic_catalog/src/lib.rs) provides authored edges, recursive requirements, effects, typed indices, the start vertex, and separate connection records; it does not implement traversal. [`Settings`](crates/seed_data/src/settings.rs) supplies player tech, proficiencies, and the item pool. [`SeedData`](crates/seed_data/src/output.rs) has no small-key identity assignments.

Implement the first version inside the generator crate, splitting graph preparation, traversal, and placement into modules. Keep the catalog immutable; use catalog indices for generation and stable authored identities for saved seeds. No new crate or general graph framework is needed.

Use vanilla world connections initially. Keep connection expansion separate from traversal so future randomized maps can supply another connection set. Do not implement map randomization, multiple restart destinations, key receipt patches, or door-binding patches in this work.

The logical model will use door-specific keys and progressive swords, shields, gloves, mail, and boomerangs before the ROM does. Saved logical placements can be inspected and solved under that model, but the existing patcher will still produce vanilla key behavior and does not implement progressive equipment receipts. The eventual ROM-playability guarantee depends on the deferred key and progressive-equipment patches and any other gameplay changes assumed by the catalog.

## Implementation steps

### Step 1: Prepare logical generation inputs and shared item identities

Prepare the existing location classification and pool expansion for logical generation. Every generated seed will use traversal and preliminary key assignment, producing typed item-placement records and the existing world layout. Replace the prototype shuffle when the logical generator is integrated in Step 11.

Define `DoorId` and `Item` in `common_types`, shared by generation, saved seeds, and patching. `DoorId` consists of the source room ID and authored door ID; it does not contain catalog indices or ROM addresses. Give `Item` an explicit variant for each supported placeable reward. Source sword, shield, glove, mail, and boomerang tiers become `Sword`, `Shield`, `Glove`, `Mail`, and `Boomerang`, each representing one progressive upgrade without a tier payload. The flute pickup becomes `Ocarina`, and the separate silver-arrow upgrade becomes `SilverArrows`. Other supported reward names follow `items.json`. Use payloads where an item carries additional identity, initially the assigned door for a Small Key. Do not introduce a generic string variant or a separate generated key-number namespace; the assigned `DoorId` is the key's inventory identity.

The final saved types have this shape; the enum excerpt shows representative variants, not the complete reward list:

```rust
pub struct DoorId {
    pub room_id: SourceRoomId,
    pub door_id: u32,
}

#[serde(tag = "name")]
pub enum Item {
    Sword,
    Shield,
    Glove,
    Mail,
    Boomerang,
    Ocarina,
    SilverArrows,
    Hookshot,
    Lamp,
    Map,
    Compass,
    BigKey,
    SmallKey { door_id: DoorId },
    // Remaining supported rewards have explicit variants.
}

pub struct ItemPlacement {
    pub item_location_id: ItemLocationId,
    pub item: Item,
}
```

Keep `ItemPlacement` in `seed_data`. An ordinary reward serializes as `"item": { "name": "Hookshot" }`. A Small Key serializes as `"item": { "name": "SmallKey", "door_id": { "room_id": { "Underworld": 100 }, "door_id": 3 } }`. The containing placement's `item_location_id` identifies the key pickup. Save placements in stable location order, and retain the existing dungeon scope for Big Keys, Maps, and Compasses through their placement context.

Define the shared identity types first. Migrate `ItemPlacement` and its generator and patcher consumers together in Step 11, after the preliminary phase can supply real door identities. Keep catalog `ItemIndex` and source item definitions distinct from the shared reward enum; source Small Key definitions remain anonymous until generation assigns a door. Store configured pool rewards directly as `ItemPoolEntry.item: Item` with `#[serde(flatten)]`, keeping the JSON shape `{"name": "Sword", "count": 3}`. Reject Small Keys, Big Keys, Maps, Compasses, pendants, and crystals in the ordinary pool during input preparation because their placement phases are separate. Reuse `Item` rather than introducing a duplicate pool enum. Normalize fixed non-key source reward names once when preparing generation, and resolve stable tech IDs to catalog indices. Construct each `Item::SmallKey` after its door assignment is settled.

Normalize every source sword, shield, glove, mail, and boomerang tier to `Item::Sword`, `Item::Shield`, `Item::Glove`, `Item::Mail`, or `Item::Boomerang`, including fixed rewards. Map `BlueMail`, `RedMail`, and `ProgressiveArmor` to `Item::Mail`; map `BlueBoomerang` and `RedBoomerang` to `Item::Boomerang`. Sum their pool quantities without removing copies or changing the number of placement slots. Keep equipment requirements tiered: the reward supplies an upgrade, while a requirement still asks for the needed acquired level. Saved rewards use names such as `"item": { "name": "Mail" }`, without an expected collection order or fixed resulting tier. Postpone progressive receipt behavior in the ROM.

Map the source `OcarinaInactive` pickup to `Item::Ocarina` and `SilverArrowUpgrade` to `Item::SilverArrows`. The catalog's `GrantItem(OcarinaActive)` is a fixed activation event, not another placed reward; retain its source identity for the normal interpreter instead of mapping it to an `Item` variant. Exclude the unsupported combined bow receipts, Quarter Magic, Triforce Pieces, and internal `HeartContainerFromPieces` receipt from `Item`. Heart-piece accumulation grants the capacity increase directly. The catalog may retain these definitions without exposing them as supported placement rewards.

The bundled ordinary pool uses three `Sword` copies, two `Shield` copies, two `Glove` copies, two `Mail` copies, one `Boomerang`, and one `Ocarina`. Replace one `TwentyRupees` reward with the additional Shield, reducing that rupee entry from 30 to 29 and keeping the ordinary pool at 149 items. The fixed Waterfall exchanges supply the third Shield upgrade and the second Boomerang upgrade; neither item is granted at the start or bundled with a Sword. Leave the unused combined `L1SwordAndShield` receipt unsupported. Unknown pool reward names fail when settings are deserialized; unknown source placement reward names and player tech IDs fail during input preparation. These identity-resolution errors and the ordinary-pool restriction check are explicitly approved.

Build each dungeon's fixed key pickup lists, small-key door list, Map/Compass pool, prize locations, and ordinary locations. Record fixed Big Key and fairy-exchange rewards in the global `fixed_items` map; fairy exchanges have no separate per-dungeon list. Derive dungeon ownership from the catalog's source room, which represents the whole authored dungeon. Retain the existing ordinary-pool count behavior and adjust bundled pool quantities if upstream changes make that necessary.

Completion: logical generation has indexed inputs, shared reward and door identities, and a defined saved representation for later integration. Do not add backward-compatibility machinery.

Implemented in `common_types::{DoorId, Item}` and `generator::inputs::prepare_inputs`. The prototype generator reuses the prepared location classification while retaining string placements until Step 11; its `Sword`, `Shield`, `Glove`, and `Mail` pool entries use the catalog's progressive receipt names (`ProgressiveArmor` for Mail), `Ocarina` uses `OcarinaInactive`, and `SilverArrows` uses `SilverArrowUpgrade`. This names the intended receipts without implementing their deferred ROM behavior.

There is no progressive boomerang receipt in the current source catalog. Until ROM support is implemented, the prototype's ordinary `Boomerang` uses the existing Blue Boomerang encoding and the fixed fairy reward retains its vanilla Red Boomerang encoding. The prepared logical rewards both use `Item::Boomerang`; define the progressive receipt when implementing its ROM behavior.

### Step 2: Compile exact shield requirements as minimum levels

Change the general requirement compiler so `shieldExact: n` uses the same minimum-level equipment requirement as `shield: n`. Do not special-case a room or exchange. Keep the Waterfall exchange's stable location identity and fixed progressive Shield reward, and leave other exact-equipment requirements, negative-flag requirements, and the free-upgrade helper unchanged. Document this agreed monotonic interpretation, including the progressive shield model, and revisit it if fairy exchanges or shield requirements change. Rebuild the logic catalog so normal traversal sees the approved monotonic requirement.

Completion: all `shieldExact` conditions use the approved minimum-level interpretation, allowing the fixed exchange into the all-items solve.

### Step 3: Expand the selected connections into a playable graph

Build outgoing adjacency for catalog edges plus generation-owned connection edges. Expand doors bidirectionally, drops only inward, teleports only outward, and screen boundaries and whirlpools bidirectionally. The destination vertex determines the world; do not add an independent world state to interior vertices.

Apply entrance follower rules and reset room-local obstacles when leaving their owning source room. Match event-exit and event-entry vertices only across the selected physical connection with the same event identity. Do not attach event-entry actions to ordinary endpoint vertices.

Add flute travel using the catalog's destinations and the active-flute capability. Keep save-and-quit behavior available to both traversers. Normal traversal returns to the start with persistent discoveries retained, follower and room obstacles reset, and resource preparation governed by the global refill-discovery bits and spawn-health rule. The simplified traverser can restart directly at the start because it has no resources or followers.

Completion: both traversers can use the same directed world topology, including routes that leave and reenter a dungeon through another entrance.

### Step 4: Implement the simplified key-pass traversal

Use a vertex work queue, a visited set for the current key inventory, and discoveries of the selected dungeon's key pickups and item collection actions. There is no resource, bottle, follower, or obstacle frontier in this mode.

Evaluate requirement composition while treating non-key leaves as free. Enforce a selected-dungeon small-key door through its candidate assignment. Enforce all selected-dungeon Big Key requirements through collection of its vanilla Big Key, including door locks and bare `Item(BigKey)` requirements on chests or other actions. Treat other dungeons' small-key and Big Key requirements as free. Big chests need no special handling; retaining their Big Key check also supports future changes to key pickup locations. Ignore temporary-state effects; preserve collection discoveries. An unassigned small-key pickup is collectible but enables no door until a candidate assigns it an identity.

Run to closure after discovering a usable key, restarting or extending traversal as needed. Report collected small-key locations, Big Key availability, and collectible item locations in the selected dungeon. Do not seed every dungeon entrance as independently reachable.

Completion: the key pass can ask which target-dungeon keys and items are collectible for a partial candidate assignment.

### Step 5: Assign one dungeon's small-key identities at a time

For the current dungeon, start with no assignments and run the simplified traverser. Gather its collectible, unassigned small-key pickups and sample a distinct unassigned door for each pickup in the candidate batch. Recompute closure with those assignments. Accept a candidate if another small key becomes collectible or every item location in that dungeon becomes collectible; otherwise discard the candidate without altering committed assignments.

Continue with the newly collectible unassigned keys. Once every dungeon item is collectible, randomly pair any remaining unassigned key pickups and doors. Those keys are already collectible without their new doors, so these final assignments cannot introduce a key-collection cycle. The result assigns an identity to every small-key pickup and door.

If an attempt cannot continue, discard only the current dungeon's assignments and restart that dungeon within its attempt budget. Never discard successfully settled previous dungeons. Other dungeons' locks remain free regardless of their saved assignments. Freeze all successful assignments before main placement.

Completion: every dungeon has a complete key assignment whose keys and item locations are collectible under the preliminary pass's relaxed requirements.

### Step 6: Define normal global and local state and item grants

Global state contains permanent item capabilities and equipment levels, health and resource capacities, heart-piece accumulation, bottles owned, known reusable bottle-content types, discovered environmental health/magic/bomb/arrow refills, collected location identities, permanent flags, dungeon-scoped Big Keys, acquired door-specific small keys, and persistently opened non-key doors. One-time refill rewards add no logical resource amount or refill-discovery bit.

Initialize shield level to zero. The first ordinary Shield grants Blue; the Waterfall reward and the other ordinary Shield then grant Red and Mirror in whichever order they are collected. The fairy reward is available once Link has at least shield level one and contributes only one collected upgrade.

Local state contains current health, magic, bombs, arrows, concrete bottle contents, the current follower, and the owning room's obstacle states. Represent bottles as a canonical multiset of contents and empty bottles so interchangeable bottle slots do not create duplicate states. At traversal initialization, construct possible fillings from globally known reusable sources within the owned bottle count; retain alternatives that can enable different routes. Omit rupees. Keep resource quantities and bottle contents together in actual route states rather than taking independent maxima.

Implement reward grants through exhaustive matches on the shared `Item` enum. Each `Item::Sword`, `Item::Shield`, `Item::Glove`, `Item::Mail`, or `Item::Boomerang` increments its equipment level by one up to the game's maximum, independently of the source tier or pickup order. The two Glove pickups grant Power Glove and Titan's Mitt capabilities in that order. Initialize mail level to zero for Green Mail; the two Mail pickups grant Blue Mail and then Red Mail, capped at level two. Use the acquired mail level for the catalog's damage table. Initialize boomerang level to zero; its ordinary pickup grants Blue and the fixed fairy reward grants Red, capped at level two. Source Blue Boomerang requirements accept level one or higher, while Red Boomerang requirements require level two. `Item::SilverArrows` grants only the silver-arrow capability; shooting still requires Bow ownership and ammunition. Capacities derive from acquired upgrades, and every fourth Piece of Heart increases health capacity directly without an internal receipt reward. Collect each location once. Grant a Small Key's persistent capability from its `Item::SmallKey` door payload, and resolve Big Key scope from the pickup's dungeon. Keep logic-catalog item requirements indexed rather than replacing them with placement-specific rewards, evaluating equipment requirements against acquired progressive levels.

`Item::Ocarina` grants flute ownership without activation. Track activation separately as permanent global progress. Interpret the catalog's fixed `GrantItem(OcarinaActive)` event as activating the owned flute, preserving ownership. Source `OcarinaInactive` requirements check ownership, including after activation; `OcarinaActive` requirements check ownership and activation. Do not route the activation event through the placed-reward lookup.

Use eighths of a heart for health and the catalog's normalized magic units. Account for potions and Fairy revival through concrete bottle contents. Consuming a Red Potion restores health, a Green Potion restores magic, and a Blue Potion restores both; each leaves an empty bottle. A stored Fairy can revive Link after a fatal hit with seven hearts, capped by health capacity, and also leaves an empty bottle. Distinguish a one-time filled-bottle reward from a reusable refill source; buying or finding one potion does not itself establish an unlimited supply.

Completion: applying a reward produces the capacities and capabilities needed by requirement evaluation. Route initialization follows the spawn-health table and known environmental refill bits.

### Step 7: Interpret normal requirements and effects

Implement each `Requirement` variant over a fixed global snapshot and local input state. Execute `And` children in order; evaluate each `Or` branch from the same input and retain meaningful successful output states. Honor tech enablement, proficiency thresholds, prize counts, followers, obstacles, and doors. Preserve `Never` in normal traversal.

Apply ammunition and magic costs, damage, refills, and resource-state checks to the local state. Process magic uses and damage hits individually so potions and Fairy revival can intervene. Consumption requires the corresponding concrete bottle content; knowing another refill type does not let a stored bottle change contents. In-route bottle refills fill empty bottles only with their specified type and honor the source's quantity limit. A Fairy refill can heal immediately; storing it for later use must honor capture requirements, including the net where applicable. Rupee payment and refill operations have no balance effect.

Report environmental refill discoveries as soon as their requirement operation is successfully reached through an executable prefix. Preserve those discoveries even if a later `And` child fails, or if the rest of that `Or` branch fails; never report a refill behind an unsatisfied preceding requirement. For example, Ganon's Tower has a route that refills magic before requiring more magic than that immediate refill supplies. The refill must still become known so a subsequent prepared traversal can attempt the whole route. Return these discoveries separately from successful requirement output states.

Apply strat effects only after the whole requirement succeeds. Temporary obstacle and follower effects update the local state. Collection, fixed item grants, flags, and persistent door openings become global discoveries only through successful actions; a failed suffix must not grant them. Do not grant a hypothetical item from an unfilled location: report that the collection action is executable so the filler can use the slot.

Completion: the interpreter supports the catalog's action semantics and returns successful states plus global discoveries.

### Step 8: Traverse normal routes and compute discovery closure

Use a work queue of vertex/local-state pairs. Store multiple compatible local states at a vertex. Begin with exact deduplication and conservative resource dominance between states with matching bottle contents and temporary state; retain alternatives when bottle contents, obstacle, follower, or resource combinations differ. Never combine unrelated states into an invented route.

When cost metrics are introduced, prefer Blue Potions over every other content and Red Potions over Fairies. Keep these preferences separate from safe reachability pruning: a Fairy can revive after a hit exceeding maximum health, while a potion used beforehand cannot prevent that death. Retain necessary Fairy alternatives and any other content-specific capabilities; do not replace revival semantics with potion healing. Defer metric-limited frontier pruning until its completeness tradeoff is explicitly addressed.

Run a traversal wave with a fixed global snapshot, gather discoveries, apply the guaranteed discoveries, and repeat until no additional global capabilities, flags, door openings, or refill types are discovered. Global discoveries may come from fixed placements, committed randomized placements, fixed events, or refill operations reached through successful requirement prefixes. Each new traversal starts with the known refilled resources full at their current capacities and concrete bottle-filling alternatives from known reusable sources; other resources start empty, except health uses its spawn table until a health refill is known.

Keep inventory dependent continuations for the next wave; a successful collection effect does not let the old snapshot immediately use the new item. Track the provenance of discoveries and successful collection actions sufficiently to explain inaccessible locations during development.

Return the collectible location set and executable unfilled collection actions. Do not use vertex reachability as a substitute for collection. Postpone a bounded metric frontier until measurements show a need; adding one would require an explicit decision about lost completeness.

Keep adjacency order, work-queue processing, requirement alternatives, discovery application, and state tie handling deterministic. Sort unordered collections before they affect candidate construction or RNG calls. Stable saved output alone does not make generation reproducible if RNG sampling depends on hash iteration order.

Completion: given a partial placement, the filler can obtain all currently collectible slots and permanent discoveries under normal logic.

### Step 9: Prepare main placement and apply Map/Compass priority

Start each main attempt with the frozen small-key assignments, fixed vanilla key rewards, fixed fairy rewards, and a fresh shuffle of pendants and crystals among prize locations. Preassign prizes using the current placement restriction, then let normal traversal collect them when their boss reward action is executable. Do not assume possession of a prize merely because it is assigned.

Traverse committed placements to closure. For newly collectible eligible ordinary locations in each dungeon, place that dungeon's outstanding Map and Compass first, choosing their slots with the RNG. If only one slot is collectible, place one and carry the other forward. Ordinary pool rewards may use that dungeon's remaining collectible slots only once both dungeon items are placed.

Treat these forced dungeon-item placements and the sampled ordinary rewards as the complete candidate for the wave. Account for all pool quantities without pre-reserving arbitrary inaccessible dungeon slots.

Preserve the hard Map/Compass priority even when it consumes every currently available slot in a dungeon. If it prevents progress, use the agreed retry policy and report the forced-placement bottleneck; do not silently relax the rule.

Completion: candidate construction honors Map/Compass priority, fixed-key locations, and prize-only placement restrictions.

### Step 10: Sample, accept, and retry placement waves

Sample without replacement from the remaining ordinary pool to fill every other newly collectible ordinary slot. Order eligible locations and candidate inputs deterministically before sampling. Evaluate the full candidate with normal traversal to closure; items that only help as a combination can therefore succeed together.

Accept the candidate when its collectible-location set includes at least one previously uncollectible required item location, or when all required locations are collectible. On acceptance, commit its placements and pool removals. On rejection, leave the committed placements, remaining pool, and global state unchanged, then sample another candidate. Advance RNG state normally so retries do not repeat the same candidate.

Reaching all locations does not end placement if some remain unfilled. Once every required location is collectible, fill the remaining eligible slots from their remaining pools, respecting Map/Compass priority and prize restrictions. This final fill needs no further progress gain. Finish only when every location has a reward, all pools are consumed, and the completed placement remains fully collectible in the final solve.

If the attempt cannot continue, restart main placement with its ordinary placements and prize shuffle cleared and its Map/Compass pools restored. Retain the settled dungeon key assignments. No placement backtracking is needed. Enforce the agreed candidate and outer-attempt budgets and report failure when the outer budget is exhausted.

Completion: the generator can construct a complete logical assignment through advancing waves and restarts.

### Step 11: Finish the logical solve and integrate output

Solve the completed placements from the agreed starting state with all dungeon key restrictions, player settings, normal resource semantics, and save and quit enabled. Require every catalog item location to have an executable collection action, including fixed keys, prizes, and fairy exchanges. This completed solve is part of logical generation's acceptance algorithm, not a separate generic validation layer. An incomplete solve makes the main placement attempt unsuccessful.

Replace the prototype uniform shuffle in `generator::generate` with the logical placement pipeline. Change `ItemPlacement.item_name: String` to `ItemPlacement.item: Item` and update all consumers together. Return complete typed reward placements, including every Small Key's stable door identity, existing metadata, and existing vanilla world layout. `SeedData.item_placements` is the complete persisted reward assignment; do not add a parallel small-key assignment field. Keep generation free of ROM and filesystem access.

Keep the existing seed format version during this prototype migration. Do not add old-format compatibility or a version update for the typed reward representation.

Update patching to resolve the shared `Item` variants to the current patch catalog's reward encodings. For now, `Item::SmallKey { .. }` uses the existing vanilla Small Key receipt encoding; carrying a door identity does not implement its ROM behavior. Keep patch-only operation driven by saved data and the current patch catalog, without requiring the logic catalog. Defer applying the generated door identities to the ROM.

Defer progressive Sword, Shield, Glove, Mail, and Boomerang receipt behavior in the ROM as well. Their saved variants do not specify a vanilla tier; do not turn an assumed logical collection order into fixed-tier rewards. Resolve their ROM receipt encoding when progressive equipment support is implemented, defining a new progressive boomerang receipt as needed. Resolve `Item::Ocarina` to the unactivated pickup receipt and `Item::SilverArrows` to the separate silver-arrow upgrade receipt; keep the bird activation as a gameplay event.

Update generator API comments and user-facing documentation to explain logical placement and the deferred ROM key behavior. Reconcile the older strategy in [plans/keys.md](plans/keys.md) and the traversal discussion in [plans/logic.md](plans/logic.md) when implementation lands.

Completion: all generated seeds use logical placement and contain the decisions needed to reproduce the logical model.

## Development verification and deferred work

Follow the repository's instruction against adding permanent tests, assertions, validation branches, or unrelated new error paths without an explicit request. The bounded-retry failure report is explicitly requested. Use temporary developer probes when implementing each step, and remove them before completing the changes. Existing compilation and test commands can be run as appropriate; they do not authorize adding new permanent checks.

Useful temporary probes include the two-collected-keys/two-doors case, a route that leaves and reenters a dungeon, a Big Key requirement on a chest or non-door action, a pickup with a collection requirement distinct from its vertex, sequential resource costs across an `Or`, a refill followed by a failing requirement, two concretely filled bottles used for different contents, a typed refill that cannot supply another content, necessary Fairy revival despite potion cost preference, Map/Compass priority across two waves and its forced-slot bottleneck, final filling after all locations become collectible, and reproducibility with the same seed. Inspect representative generated placements and their discovery histories after integration.

Deferred: actual door-specific ROM key behavior; progressive Sword, Shield, Glove, Mail, and Boomerang ROM receipts; Thieves' Town's asymmetric reverse entrance lock beyond the current catalog model; randomized map connections; alternative restart destinations; reduced key-pass graphs; metric-limited traversal frontiers; placement balancing and progression pacing; and other gameplay patches assumed by the logic catalog.
