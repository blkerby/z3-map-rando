# Item placement

The generator places every item logically, using normal traversal under the player's tech, proficiency, and notable settings. It is implemented in [`generator`](../../crates/generator/src/lib.rs); see [logic](logic.md) for the catalog it reads, [traversal](traversal.md) for how reachability is computed, [door-specific keys](keys.md) for Small Key assignment, and [Great Fairy upgrades](fairies.md) for the fairy locations.

## Guarantees

- Every catalog item location is collectible, by executing its collection action rather than merely reaching its vertex. Fixed events, such as flute activation, are not item locations.
- Save and quit is always available. It returns to the starting vertex at Link's House with collected inventory and persistent progress, restores full health, resets room-local obstacles, loses every follower, and keeps magic, bomb, arrow, and bottle-use counters. No escape route is required from a pickup.
- Rupee costs are free, and traversal tracks no rupee balance.

## Items

Placed items are `common_types::Item` values (see [item mapping](item_mapping.md)). Sword, Shield, Glove, Mail, Boomerang, and Bow are progressive: each collected copy advances that equipment by one level, up to its maximum, regardless of collection order. The Bow progresses to Silver Arrows. `Item::Ocarina` is the unactivated flute; activation is a permanent upgrade granted by the bird event. Bee and Gold Bee bottle contents have no logical use, though their items and locations remain. One-time refill items grant no logical refill or preparation; permanent capacity upgrades and bottle ownership take effect normally.

## Location classes

- Small and Big Key pickups stay at their vanilla locations. Each Small Key is assigned one door in its dungeon before main placement.
- Pendants and crystals are shuffled among the prize locations.
- The two Great Fairy upgrade locations receive a class upgrade or nothing, as described in [Great Fairy upgrades](fairies.md#placement).
- All other locations are ordinary.

## Main placement

Each complete attempt starts with the selected Small Key assignments, fixed Big Keys, shuffled prizes, and empty ordinary and fairy locations. Placement proceeds in waves. In each wave, the generator takes the ordinary locations that have become collectible, first places each dungeon's Map and Compass among collectible eligible locations in that dungeon, then samples the remaining collectible locations from the entire remaining ordinary pool. There is no separate progression fill followed by junk fill.

A candidate is accepted when its traversal closure exposes at least one more item location, or when every location is reachable. A rejected candidate leaves the committed state unchanged; there is no backtracking. Completion uses the accepted closure's collected-location bits once every slot is filled, including key pickups, prizes, and fairy upgrades, so no separate solve is needed.

## Retry budgets

| Budget | Limit | On exhaustion |
| --- | ---: | --- |
| Small Key candidates per wave | 100 | Restart the current dungeon's assignment |
| Small Key attempts per dungeon | 1000 | Reject a random map, consuming one complete attempt; fail generation for the vanilla layout |
| Placement candidates per wave | 100 | Restart the complete attempt |
| Complete generation attempts | 1000 | Report generation failure |
| Border selections per random map | 10 | Reject the map, consuming one complete attempt (see [map randomization](../map_randomization.md)) |

In Random mode, a failed Small Key dungeon attempt restarts only that dungeon and keeps earlier dungeons' assignments; every new complete attempt reassigns all keys. A placement step reports a brief failure reason distinguishing a rejected sample, no collectible empty slots, and Map/Compass placements consuming every collectible slot. A failed randomized attempt does not prove that a layout has no solution.
