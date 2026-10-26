# Resource preparation and incremental traversal

Status: Phases 1–3 are implemented. Placement candidates retain traversal reducers alongside global state, and actual global changes schedule dependent edges through a reverse index built once with the expanded graph. Focused comparisons against fresh traversal and the Phase 2 all-reached reference passed. Additional generation timing runs remain deferred at the user's request.

This plan supersedes the resource initialization, environmental refill, and fresh-wave policies in [the item-placement plan](../plan.md) and [the logic plan](logic.md). Their unrelated placement, key-assignment, follower, and progression policies continue to apply. Update their descriptions when implementing each phase so they describe the resulting behavior.

## Purpose

Avoid repeatedly evaluating unchanged routes during item placement. Profiling the optimized generator with default and hard settings attributed approximately 53–54% of sampled CPU time to resource scoring and another 20–23% to reducer bookkeeping. A temporary default-settings trace evaluated 18,760 candidates and performed 14,877 traversals in approximately 97 seconds without completing generation. Reducing repeated traversal work should avoid much of this scoring without retuning unrelated metrics.

Separate the intentional resource-policy changes from the traversal optimization. Compare the incremental traverser with a fresh traverser using the new resource policies, rather than requiring equivalent results to the old resource model.

## Agreed resource model

### Full starting health

Assume Link can repeatedly collect the three heart pots at Link's House until fully healed. Every route starts with zero health consumption relative to the current full health capacity. There is no starting-health baseline or starting-versus-full scoring preference. Health uses the two representations described below so Fairy revival can retain a fixed remaining amount.

Save-and-quit returns to the starting vertex and includes this implicit healing, selecting the full-relative representation with zero consumed. Keep its existing follower loss and obstacle reset. Preserve magic, bomb, arrow, and bottle-use counters across this in-route transition; it does not create another preparation opportunity or replenish the bottle budget.

### Global usable capacities

Keep physical capacities in `GlobalState` and add `magic_usable_capacity`, `bomb_usable_capacity`, and `arrow_usable_capacity`. Track obtained preparation independently in `GlobalState::refills`, even when physical capacity is zero. Usable capacity is derived as physical capacity when the corresponding refill flag is set, or zero otherwise. A zero usable capacity must never be used to infer whether a source has been obtained.

Local magic, bomb, and arrow usage becomes ordinary unsigned consumption, initialized to zero. Remove `ResourceUsage::UNPREPARED` and its sentinel handling; remove the wrapper if it has no remaining purpose. Available resources are calculated as `usable_capacity - used`. Health availability depends on its representation: current capacity minus consumption, or an explicit remaining amount.

Preparation updates global usable capacity between waves, without rewriting local states. Recompute usable capacities from physical capacities and obtained refill flags after preparation discovery or capacity changes. A source obtained at zero capacity remains recorded, so a later upgrade immediately makes that capacity usable; an unobtained source leaves usable capacity zero. Use this distinction consistently for ammunition, magic spending, potion effects, and resource checks. A full-resource check requires the resource to be prepared before interpreting its usage as depletion from full; an unprepared resource with zero consumption is not full. Health is always prepared.

Keep magic, ammo, and bottle scoring based on the existing local consumption counters and fixed metric weights. Use the capacity-independent health mapping described below. Preparation and capacity increases do not change cached metric scores. Preserve the other metric definitions and avoid unrelated tuning.

### Environmental refills establish preparation only

Retain preparation discoveries from partial and full environmental sources. The agreed reasoning is that a source first reached while its resource is unprepared was reached without spending that resource; repeated visits with save-and-quit between them permit stocking up before a subsequent route.

For now, assume no environmental source is repeatable during a route. Every environmental `Refill` requirement leaves local consumption unchanged, regardless of its limit. This includes full magic pots, partial health supplies, Great Fairy heals, and direct healing from loose fairies. Link's House is covered implicitly by starting health and save-and-quit, without a location-specific refill exception in the interpreter.

Discover bottle-content preparation sources independently of bottle ownership. Fairy-source discovery still requires the Bug-Catching Net; actual potion use or Fairy revival requires an unused owned bottle. These sources do not replenish `bottles_used` or provide additional bottle uses during a route. Preserve the current distinction between direct magic preparation and bottle-content preparation; a potion does not implicitly prepare an otherwise unprepared magic resource.

One-time refill item placements continue to have no logical resource-restoring effect. Their locations and actual placed items remain present. Retain existing handling of permanent capacity upgrades, free rupee costs, and ignored Bee/Gold Bee contents.

Do not add repeatability annotations or edit `z3-json-data` in this implementation. Preserve refill limits in the catalog for later work. The conservative policy can exclude valid routes and change generated placements or generation success rates; never compensate by silently treating an unclassified source as repeatable.

### Health after bottled Fairy revival

Red and Blue Potions restore full health and spend a local bottle use. Green and Blue Potions restore prepared magic and spend a bottle use. These are explicit consumable actions, distinct from environmental refills; keep their existing bottle budget and sequential-use behavior.

Retain bottled Fairy revival using two explicit health representations, for example `HealthState::Consumed(u8)` and `HealthState::Remaining(u8)`. Only Fairy revival selects `Remaining`; ordinary route initialization, save-and-quit, and full-healing potions select `Consumed`. There is no general partial-refill expression or additional baseline system.

| Operation | Full-relative consumption | Explicit remaining health |
| --- | --- | --- |
| Read available health | `health_capacity - consumed` | Stored `remaining` |
| Survive a damage hit | Increase `consumed` by the damage | Decrease `remaining` by the damage |
| Bottled Fairy revival | Spend one bottle use and switch to `Remaining(min(health_capacity, 56))` | Spend one bottle use and reset to `Remaining(min(health_capacity, 56))` |
| Full heal or save-and-quit | Set `Consumed(0)` | Switch to `Consumed(0)` |
| Capacity increase | Keep consumption unchanged; availability increases | Keep the stored remaining amount unchanged |

All health amounts remain in eighths of a heart. Apply damage hit by hit and retain the current survival condition: remaining health must exceed the hit's damage to survive without a revival. The revival amount is capped and stored when the revival occurs; do not recompute it against a later capacity. Further hits continue using the explicit remaining amount until another revival or full heal changes it.

Keep `ResourceMissingAtMost` conservative: a health check requires the full-relative representation and compares `consumed` with the permitted missing amount. A `Remaining` state does not qualify, even if it happens to equal the current maximum. This avoids a capacity increase invalidating an earlier successful full-health check.

### Capacity-independent health scoring

Score ordinary resource states using their consumption counters. Favor lower consumption; this can be expressed as a negative resource value, but retain the existing lower-is-better cost convention and lookup tables rather than changing signs throughout the reducer.

For health scoring, map `Consumed(u)` to effective usage `u`, and map `Remaining(r)` to effective usage `160 - r`, using the maximum health capacity of 20 hearts in eighth-heart units. Feed this effective usage into the existing health cost tables. For example, seven remaining hearts map to 13 hearts consumed for scoring, regardless of the player's actual current capacity. Never convert the stored remaining amount to consumption using the current capacity.

This is intentionally pessimistic for remaining-health states below maximum capacity. It can cause the bounded reducer to discard useful alternatives and therefore sacrifice completeness; it never changes actual health availability or grants extra health. Keep actual survival checks separate from scoring. Preserve the 18-metric bound and tie policy, with no additional baseline bonus or separate reducer banks.

### Cached revival paths after capacity or armor improves

Retain cached revival paths when an improved global state lets Link survive the hit that formerly triggered a Fairy. The survivor retains an additional unused bottle/Fairy. If later damage makes the lower immediate health matter, that Fairy can revive Link at the later lethal hit instead. If no such hit occurs, the saved Fairy was unnecessary. Later revival restores at least the amount established by the earlier revival, since capacity has not decreased.

For example, compare a cached state with seven hearts after spending a Fairy against a survivor with one heart and that Fairy still available. A subsequent six-heart hit leaves the cached route with one heart, while the survivor revives with seven. Continue applying damage hit by hit so this delayed-revival behavior is preserved.

Treat the cached state as a conservative summary of achievable continuation, rather than requiring its exact earlier death and revival to occur again. This reasoning relies on retaining the extra bottle use in the real continuation and on remaining-health states being ineligible for `ResourceMissingAtMost` health checks; such a state cannot require a full-health action before a later revival. Full heals and save-and-quit merge the health representations back into `Consumed(0)`, with the surviving route retaining any saved bottle use. No proactive Fairy-use action or capacity-triggered invalidation is needed for this case. Verify this relation explicitly against the fresh traversal during implementation.

## Phase 1: resource handling with fresh traversals

1. Update `state.rs` to hold usable capacities, maintain them when permanent capacities increase, store plain magic/ammo/bottle usage counters, and introduce the two health representations. Keep preparation discoveries separate from the global state they update so a newly found source becomes available in the next wave.
2. Update `traversal.rs` to initialize health as `Consumed(0)`, initialize the other local usage counters to zero, and apply preparation discoveries to usable capacities between otherwise unchanged fresh waves. Remove the spawn-health table and its callers once unused.
3. Update `interpreter.rs` to use usable capacities for spending and checks, remove environmental restoration, and retain preparation discovery. Preserve effects discovered in successful requirement prefixes even when a later requirement fails. Handle both health representations in damage, Fairy revival, full-health checks, and potions, retaining the existing bottle-use budget.
4. Rename `Requirement::ResetHealthToSpawn` to `ResetHealthToFull` and update its interpreter and generated save-and-quit edges. Update the catalog comments describing refill behavior. Rebuild the logic catalog because the serialized requirement schema changes; do not add compatibility machinery.
5. Update `cost_metrics.rs` to read plain counters, map `Remaining(r)` to `160 - r` for health scoring, and remove sentinel-specific explanations. Keep existing cost tables, weights, reducer bounds, and tie policy unchanged.
6. Update the existing resource and save-and-quit documentation in `plan.md` and `plans/logic.md` to match the implemented model.

Keep `PlacementState` and the traversal queue behavior unchanged in this phase. No retained graph state, reverse dependency index, new configuration, or source-data schema is needed yet.

### Phase 1 verification and completion

Use temporary focused checks for full starting/save-and-quit health, empty resources before preparation, preparation without rewriting existing local states, upgrades before and after preparation, preparation discovery at zero capacity followed by a later upgrade, closure termination when a known source still has zero capacity, and resource checks on unprepared resources. Exercise environmental health/magic/ammo refills with both small and full limits and verify that they discover preparation where appropriate but never restore local availability. Check potion budgets, sequential damage/magic handling, survival and repeated Fairy revivals in both health modes, and switching back to `Consumed(0)` after a full heal. Verify that a cached `Remaining` amount does not increase with capacity and never satisfies a full-relative health check. Check that both health representations have unchanged scores across capacity increases, including `Remaining(56)` scoring as effective usage 104 at every capacity where it can occur.

Run the applicable Cargo checks and rebuild the logic catalog. A bounded set of default- and hard-settings release generations was exercised during Phase 1, with outputs outside the worktree. Further timing runs are deferred at the user's request. Reduced reachability can change results, so old placements are not an equivalence target. Investigate material regressions before proceeding.

Remove temporary checks and instrumentation before completing the phase, following repository instructions. Capture a fresh-traversal timing baseline under the new semantics when performance comparison is resumed. Phase 1 is complete when the resource model works independently of traversal reuse.

### Phase 1 verification results

The workspace check and tests passed, and the logic catalog was rebuilt after renaming the health-reset requirement. Eight temporary integration checks covered resource preparation, capacity upgrades, one-time receipts, ignored environmental restoration, bottle sources and budgets, both health representations, full-health checks, sequential healing/damage, and save-and-quit. The checks included 4,320 batched-magic cases compared with a per-use reference and 1,920 damage/Fairy cases compared with sequential evaluation. Full-inventory closure reached all 259 catalog item locations with every reducer bounded by 18 states. A further temporary check verified independent source discovery with all three capacities zero, termination without repeated discovery at zero capacity, preserved local states, later capacity upgrades enabling prepared resources, and upgrades leaving resources unusable when no source was obtained. Workspace tests passed again after this refinement. Temporary test sources were removed after passing.

## Phase 2: retain traversal state

Store the per-vertex `LocalStateReducer` collection with committed `PlacementState`, alongside `global`. Initialize it from the starting vertex only for a new complete generation attempt. Extend it across closure waves and accepted placement candidates instead of rebuilding it.

Use the existing candidate cloning boundary first: each candidate receives the committed global and traversal state; acceptance commits both, rejection discards both. Rejected candidates must not leak inventory, discoveries, or reducers into later candidates. A new complete generation attempt starts fresh because its keys and prizes change. Do not share this cache between attempts or generation requests.

Refactor `traverse`/`compute_closure` to operate on retained reducers and an explicit initial work set. Continue collecting newly placed items at already-reachable locations before evaluating edges. Keep discoveries separate from the global snapshot used for the current wave, apply discoveries and collections between waves, and stop when no relevant global changes remain. Junk items that change no traversal capability should not trigger a traversal.

During development, use an all-reached-vertices seed pass as a simple reference for state reuse before adding selective scheduling. This is a staging aid, not an additional supported traversal mode. Do not introduce copy-on-write state or other clone optimizations unless measurements identify candidate cloning as a material new bottleneck.

Verify that preparation and capacity changes leave cached local values and metric scores intact. Continue reevaluating resource-consuming edges when capacities or cost modifiers improve; unchanged counters do not imply unchanged edge outcomes. Previously retained states must remain conservative summaries of achievable continuations under the updated global state. Exercise delayed Fairy revival when an old Fairy-triggering hit becomes survivable, without requiring the exact old revival state to be reproduced.

### Phase 2 verification results

Seven temporary release checks passed against the Phase 1 fresh-traversal reference. They covered fixed global snapshots, multiple discoveries in a wave, discoveries from failed prefixes and unchanged destinations, nested alternatives, self-loop requeueing, preparation at zero capacity, later prepared capacity upgrades, Mail/Half Magic improvements, health-capacity changes, unchanged cached values and metric scores in both health representations, junk collection without traversal, and candidate acceptance/rejection isolation. Eighteen Fairy scenarios checked delayed revival after capacity or Mail improves, including retained continuations without rerunning the formerly lethal prefix and rejection of remaining-health states by full-health checks.

Default and hard settings with numeric seeds 0, 1, and 2 exercised 240 candidates across six attempts: 30 accepted and 210 rejected. Initial closures and every accepted candidate matched fresh traversal for reachable/collected locations, inventory, flags, doors, activation, and preparation. Rejections preserved the committed state, including reducers. Reinitializing each attempt reproduced fresh starting state without inheriting candidate traversal. These bounded attempts did not complete generation; they are semantic comparisons, not generation-success or timing claims. Full-inventory closures under both settings reached all 259 item locations and matched fresh traversal; every reducer stayed within 18 states.

The sampled partial-attempt reducer payload was approximately 97–99 KiB per candidate, excluding spare vector capacity and allocator overhead. Cloning the entire candidate averaged approximately 11–14 microseconds over 1,000 clones per sample in the optimized build. These samples do not justify clone optimizations. Additional full-generation timing comparisons remain deferred. Temporary checks and the fresh reference were removed after verification; no runtime reference mode, instrumentation, dependencies, or catalog changes remain.

## Phase 3: seed only affected edges

Build a reverse dependency index once with the expanded `TraversalGraph`, including catalog edges, room connections, and generated save-and-quit edges. Refer to edges by their existing outgoing-list position or a comparably small runtime identifier; avoid changing the catalog solely to assign runtime edge IDs.

Walk nested `And` and `Or` requirements and index mutable global values that can enable an edge outcome or a new discovery, including implicit reads inside primitive requirements. Static player settings need no change notifications during generation. `Refill` reads of already-known source flags only suppress duplicate discoveries and need no scheduling dependency; the Fairy Net check is its only dependency.

| Global change | Edge evaluations that must be reconsidered |
| --- | --- |
| Unique item or equipment level | Direct item/equipment requirements and indirect capability checks, including Bow/Silver Arrows for ammo, Half Magic for magic cost, Mail for damage, and Net for fairy-source discovery |
| Flag, door, Big Key, Ocarina, or prize ownership | Corresponding requirements, including Big Key checks inside door evaluation and individual prize changes feeding prize-count checks |
| Health capacity | Damage/survival and health-dependent resource or consumable checks |
| Magic/ammo usable capacity or obtained refill flags | Spending and resource checks for that resource, including newly prepared resources and checks enabled by an obtained source even when usable capacity remains zero |
| Bottles owned or prepared content availability | Bottle-count/content checks and potion-dependent damage, magic, and missing-resource evaluation; source discovery is independent of bottle ownership, with Fairy Net discovery indexed under unique items |
| Reachable item location | Collection effects that read its reachability to filter discoveries; collected-location bookkeeping itself does not schedule edges |

Drive scheduling from newly applied discoveries and item receipts. Discoveries already exclude known progress and directly supply keys for locations, flags, doors, activation, and preparation. `GlobalState::collect_item` returns an optional scheduling key from its existing item match, suppressing duplicate/capped capabilities, partial heart pieces, and ammunition upgrades that leave usable capacity unchanged. Accumulate keys in a `HashSet` so each changed dependency is looked up once per wave, before sorting and deduplicating the affected edges. Edge deduplication still handles overlap between different dependency keys, and sorting preserves deterministic graph order. No per-wave global-state clone, full dependency scan, or generic event system is needed. Include changes observed through edge effects as well as requirements when determining dependencies.

At the start of a wave, select affected edges only when their source vertex has retained states. Deduplicate this initial work, preserve deterministic graph order, and evaluate the initial edges directly before processing the vertex queue. A global change must force affected edges to run even if no source reducer improved. Successful requirement prefixes must still report discoveries when an edge fails or its destination reducer does not improve.

When an evaluated edge improves its destination reducer, enqueue that destination vertex unless already queued. Process the vertex queue by clearing its queued bit, snapshotting its retained states once, and evaluating all outgoing edges against that snapshot. Self-loop improvements can requeue the vertex because its queued bit is cleared before expansion. Newly reached vertices expand normally, so edges whose source was unreachable at notification time need no separate pending-change mechanism. The first traversal still begins at the starting vertex.

### Phase 3 verification results

Six temporary release checks passed against both the Phase 1 fresh-traversal reference and the Phase 2 retained all-reached reference. Focused cases covered every equipment kind and individual prize, direct and implicit Bow/Silver Arrows requirements, capacity upgrades, Half Magic, Mail, Big Key checks inside doors, Small Key door updates, flags, activation, nested alternatives, discoveries from failed prefixes and unchanged destinations, self-loop propagation, newly reached sources, preparation at zero capacity, bottle counts and all four prepared contents, health/magic potion checks, and eighteen delayed-Fairy scenarios. A reached source with 100 unrelated outgoing edges verified that a new capability evaluates only dependent edges even when the source reducer is unchanged. Junk collection scheduled zero edge evaluations. A rejected candidate that improved its private reducers left committed inventory and reducers unchanged.

Default and hard settings with numeric seeds 0, 1, and 2 exercised 600 candidates across six attempts: 34 accepted and 566 rejected. Initial closures matched fresh traversal, and every accepted closure matched both references for reachable/collected locations, inventory, flags, doors, activation, and preparation. Reinitialization reproduced each fresh attempt without retaining candidate state. Full-inventory closures under both settings matched both references and reached all 259 item locations; every reducer remained bounded by 18 states. These bounded attempts did not complete generation.

Temporary counters measured paired work for the 34 accepted closures with identical placements and initial global snapshots (Phase 2 also started from the same retained reducers as selective scheduling): selective scheduling evaluated 11,636 edges and attempted 47,782 reducer insertions, compared with 126,440 edges / 372,443 insertions for Phase 2 and 197,199 edges / 541,730 insertions for fresh traversal. These are closure-work counts, not full-generation timing or success-rate comparisons. Generation timing remains deferred at the user's request. The dependency index is shared with the graph and is not cloned with candidates; the Phase 2 candidate-memory and clone samples remain the available measurements. Temporary checks, counters, and both references were removed after verification. No dependency, catalog, settings, seed-format, key-pass behavior, or runtime traversal-mode changes were needed.

### Follow-up: bottle-independent source discovery

Bottle-content sources are now recorded before a bottle is owned, with only the Fairy Net check retained as a `Refill` scheduling dependency. Three temporary focused checks passed, including 264 resource/limit/Net/bottle/known-source cases, unchanged local counters, successful refill prefixes and discoveries before failed suffixes, delayed availability between waves, consumption requiring an unused owned bottle, exhausted bottle budgets, and incremental Fairy discovery when the Net is collected before a bottle. The temporary checks were removed after verification.

### Follow-up: vertex queue for local propagation

Initial affected edges now run directly in a separate pass, and only improved destination vertices enter the propagation queue. Queued bookkeeping is one bit per vertex, and each vertex expansion shares a single source-state snapshot across its outgoing edges. Four temporary release checks passed against the prior edge-queue implementation: selective initial work and discoveries without destination improvements, joins without invented state combinations, initial and propagated self-loop requeueing, six default/hard attempts with 600 candidates (34 accepted and 566 rejected), and full-inventory closures reaching all 259 locations under both settings. Initial and accepted closures matched for reachable/collected locations, inventory, flags, doors, activation, and preparation; rejected candidates preserved committed state, and reducers stayed within 18 states. Temporary checks and the reference were removed afterward. Full-generation timing was not repeated, and the earlier Phase 3 work counts describe the edge-queue implementation.

### Follow-up: scheduling from discoveries and item receipts

Closure now accumulates dependency keys while applying discoveries and collecting placed items, then looks up only those keys in the existing reverse index. `GlobalDependency` lives beside global state, and `collect_item` returns at most one changed key per receipt. Sorted/deduplicated initial edges, vertex propagation, and fixed global state within each wave are unchanged.

A temporary unit check compared all 73 item variants across 7,008 receipts with the previous item implementation and verified that returned keys exactly matched the former global-state comparisons. Cases included repeated and capped grants, all heart-piece counts, prepared/unprepared and zero ammunition capacities, Ocarina states, bottles, prizes, and both key types. Three temporary release checks compared closure with the previous snapshot scheduler, covering multiple discoveries, failed prefixes, unchanged source reducers, preparation at zero capacity, later usable-capacity upgrades, and collection in already-reachable slots. Six default/hard attempts exercised 600 candidates (34 accepted, 566 rejected); initial and accepted closures matched, rejections preserved committed state, and fresh initialization reproduced each attempt. Both full-inventory closures reached all 259 locations, with reducers bounded by 18 states. Temporary checks and references were removed after verification. Full-generation timing was not measured.

## Incremental verification and performance comparison

Keep a temporary fresh-traversal reference using Phase 1 semantics. Compare reachable and collected locations, flags, unlocked doors, preparation, and inventory at closure boundaries for selected generation attempts. Exercise multiple discoveries in one wave, unchanged source reducers with newly enabled edges, self-loops, nested alternatives, cost reductions from Mail/Half Magic, capacity changes in both health modes, formerly lethal hits becoming survivable, bottle preparation, rejected candidates, and complete-attempt restarts.

Do not require identical reducer ordering or byte-identical seeds: metric tie handling and queue order may choose different witnesses. Investigate reachability/discovery discrepancies as blockers before claiming equivalence; do not hide them by changing the reducer policy or widening the optimization scope.

Measure fresh versus incremental generation using the same optimized build settings, catalog contents, player settings, numeric seeds, and time bounds. Include several default and hard seeds and record completed generations separately from timed-out or exhausted runs. Use temporary counts for traversals/edge evaluations and reducer insertions to explain the timing changes. Measure memory and candidate-clone overhead now that candidates carry graph state. Remove temporary references and instrumentation after verification.

## Files and interfaces affected

| Area | Planned changes |
| --- | --- |
| `crates/generator/src/state.rs` | Usable capacities, two health representations, plain magic/ammo/bottle counters, capacity upgrades, preparedness state, scheduling keys reported by item collection |
| `crates/generator/src/interpreter.rs` | Resource checks/spending, preparation-only environmental sources, full-health reset, consumables, dependency reads |
| `crates/generator/src/traversal.rs` | Full-health initialization, preparation application, discovery/item scheduling keys, retained reducers, work scheduling |
| `crates/generator/src/cost_metrics.rs` | Plain-counter access and comparison of health representations; retain unrelated metric definitions |
| `crates/generator/src/local_state_reducer.rs` | Update wave-only documentation for cache reuse; retain reduction and tie behavior |
| `crates/generator/src/graph.rs` | Full-health save-and-quit requirement and runtime reverse dependency index |
| `crates/generator/src/placement.rs` | Cache ownership, initialization, candidate clone/commit boundaries, closure callers |
| `crates/generator/src/lib.rs` | Update affected callers if needed; keep complete-attempt and candidate retry policies |
| `crates/logic_catalog/src/lib.rs` | Rename the full-health reset variant and update resource semantics documentation; rebuild the catalog |
| `plan.md`, `plans/logic.md` | Replace superseded resource and traversal descriptions as each phase lands |

Search all callers and matches of `ResourceUsage`, `UNPREPARED`, `ResetHealthToSpawn`, `get_spawn_health`, `traverse`, and `compute_closure` before editing. The closure callers are ordinary placement initialization and candidate evaluation, shared no-key baseline initialization, and incremental dungeon key setup and candidate evaluation. `compute_closure` accepts an initial `HashSet<GlobalDependency>` for capabilities granted before a call; its existing reverse-index processing handles these alongside newly discovered changes. Verify compilation across consumers after changing public requirement or traversal interfaces.

A subsequent approved change reuses normal incremental closure for preliminary key assignment with full non-key inventory and other dungeons' key access. A common full-inventory closure without dungeon keys is initialized once per generation call and reused across complete attempts. Each selected dungeon clones this baseline, grants other dungeons' key access, and reports those initial dependency changes to normal closure. Global state, local-state reducers, and placements are cloned together for candidates and reset together for dungeon retries. Keep the item-placement and retry policies, CLI/settings/seed format, ROM patches, and submodules unchanged. No new dependencies, permanent tests, fixtures, runtime switches, or telemetry are planned.

## Deferred work

Repeatable-refill classification belongs in later source-data work. Future annotations must distinguish safe repeated route restoration from sources that reset only between routes. Adding route refills later must preserve valid cached witnesses and explicitly reconsider capacity-dependent refill representations.

Keep resource-model changes, state reuse, and selective scheduling reviewable and measurable as separate phases. Proceed to the next phase only after the preceding phase's semantics and verification are settled.
