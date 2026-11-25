# Traversal

Generation decides reachability with one traverser, shared by Small Key assignment and main placement. It is implemented in [`traversal.rs`](../../crates/generator/src/traversal.rs), with requirements interpreted by [`interpreter.rs`](../../crates/generator/src/interpreter.rs) and state in [`state.rs`](../../crates/generator/src/state.rs). See [logic](logic.md#traversal-implications) for how the catalog's requirements map onto local and global state.

## Resource model

### Health

Every route starts at full health, since Link can repeatedly collect the heart pots at Link's House. Health is stored in eighths of a heart, in one of two representations:

| Operation | `Consumed(u)`, relative to full capacity | `Remaining(r)`, after bottled Fairy revival |
| --- | --- | --- |
| Available health | `health_capacity - u` | `r` |
| Survive a damage hit | Increase `u` by the damage | Decrease `r` by the damage |
| Bottled Fairy revival | Spend a bottle use; switch to `Remaining(min(health_capacity, 56))` | Spend a bottle use; reset to `Remaining(min(health_capacity, 56))` |
| Full heal or save-and-quit | `Consumed(0)` | Switch to `Consumed(0)` |
| Capacity increase | Availability increases | Unchanged |

Damage applies hit by hit, and surviving a hit without revival requires more remaining health than its damage. Red and Blue Potions restore full health, and Green and Blue Potions restore prepared magic, each spending a bottle use. A missing-health check requires the `Consumed` representation, so a capacity increase cannot invalidate an earlier full-health check.

For scoring only, `Remaining(r)` counts as usage `160 - r`, using the 20-heart maximum, so scores do not depend on current capacity. This is pessimistic for remaining-health states and can make the bounded reducer discard useful alternatives, but it never changes actual availability.

A cached revival route stays valid when a later capacity or armor improvement lets Link survive the hit that triggered the Fairy: the surviving route keeps its unused Fairy, which can revive Link at a later lethal hit, restoring at least as much as the earlier revival. The cached state is therefore a conservative summary of an achievable continuation.

### Magic, ammunition, and bottles

Global state keeps physical capacities and, separately, whether each resource has an obtained refill source, even at zero capacity. Usable capacity is the physical capacity once a source is obtained, and zero otherwise; it is recomputed after discoveries and capacity changes, so a source obtained at zero capacity makes a later capacity upgrade usable immediately. Local state keeps plain consumption counters starting at zero, and availability is usable capacity minus consumption. A full-resource check requires the resource to be prepared, and a potion does not prepare an otherwise unprepared magic resource.

Environmental refill sources only establish preparation for later waves: a source first reached without its resource was reached without spending it, so repeated visits with save-and-quit between them can stock up before a later route. No environmental source restores resources during a route, including magic pots, partial health supplies, Great Fairy heals, and loose fairies. Refill limits stay in the catalog for later use.

Global state tracks owned bottles and the bottle contents with a known source; local state tracks only `bottles_used`. A bottle use needs an unused owned bottle and a known Red Potion, Green Potion, Blue Potion, or Fairy source, and does not replenish during a route. Bottle-content sources are recorded before a bottle is owned; a Fairy source also requires the Bug-Catching Net.

## Local states

Each vertex keeps a reducer of local states: resources, obstacles, and followers. Identical states merge, and dominance compares only compatible states, so independent maxima never invent a state. Cleared obstacles are not always preferable, because the source data also tests uncleared obstacles. The reducer keeps the state winning each of 19 fixed cost metrics (see [`cost_metrics.rs`](../../crates/generator/src/cost_metrics.rs)), which bounds the retained states at the cost of completeness.

## Waves and incremental scheduling

Traversal runs in waves with fixed global state: inventory, flags, unlocked doors, flute activation, prizes, and preparation. Discoveries made during a wave, including those from successful requirement prefixes of edges that fail later, are applied between waves by `compute_closure`, which also collects placed items at reachable locations.

Reducers persist across waves and accepted placement candidates; a rejected candidate's clone discards them. When global state changes, only the dependent edges run again. A reverse index, built once with the expanded graph, maps each mutable global value to the edges whose requirements or effects read it, including implicit reads such as Mail for damage, Half Magic for magic cost, Bow for arrows, and the Net for fairy sources. Applied discoveries supply their dependency keys directly, and `GlobalState::collect_item` returns a key only when a receipt changes a capability. Each key is looked up once per wave.

A wave first evaluates the affected edges whose source vertex has retained states, in sorted graph order without duplicates, even if no source reducer improved. Each edge that improves its destination reducer queues that vertex once. Expanding a queued vertex evaluates all its outgoing edges against one snapshot of its states; self-loops can requeue it. Newly reached vertices expand normally, so edges from vertices unreachable when a change occurred need no pending-change state. The first traversal of an attempt starts at the starting vertex.
