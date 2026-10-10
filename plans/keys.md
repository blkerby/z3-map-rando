# Door-specific keys

Each key unlocks one specific door and is a persistent inventory item, rather than a consumable dungeon resource. Opening one door cannot spend the means to open another. Keys normally stay in their own dungeon; a mixed-key mode changes placement eligibility without changing traversal.

This removes key-choice alternatives and the need for a special possibility traversal to discover harmful expenditures. The remaining problem is avoiding placement cycles: a key cannot depend on passing its own door, directly or through other keys and movement items.

## Placement

Use one progression-placement process for keys and other items:

1. Find logically collectible, unfilled locations with the current inventory.
2. Place progression in eligible locations; dungeon-local keys require a location in their dungeon.
3. Add the wave's items to inventory and recompute reachability.
4. Backtrack or retry when suitable placements cannot advance progression.

Constrained keys need suitable slots. An unrelated item placed in a dungeon's only accessible location can leave nowhere to put its first key. Prioritize constrained items or backtrack when their locations are exhausted. Fill junk after progression, but remember that unrestricted progression items can also occupy needed slots.

If no eligible location can be reached without a dungeon's own keys, even after obtaining independently available movement items, dungeon-local placement has no solution. The layout needs an accessible location, another entry route, a starting key, or permission to place a key elsewhere.

For example, Hookshot obtained outside a dungeon might expose a ledge where its first door key can be placed. Putting that Hookshot behind the same door would create a circular dependency.

A separate key-placement phase is optional for distribution preferences. Assuming full movement inventory while placing keys would not establish actual progression; the remaining placement still needs to avoid movement-item dependency cycles.

## Logic and game representation

The [logic catalog](logic.md) retains logical door identities and lock types. Generation assigns unique key pickups to doors independently of their placement locations. A key-door requirement checks possession of the assigned persistent key; it needs neither a door-open effect nor a remaining-key balance.

Items and permanent flags are global facts between traversal waves. Resource amounts and temporary obstacles still require local states, but there are no key-history conditions attached to them. Save and quit retains collected keys and opened doors; it does not turn directional passages into bidirectional ones.

The source currently uses anonymous `SmallKey` items and room-local door IDs. Generation-side key identities, their association with doors, and ROM receipt/door changes need to be defined. Existing big-key behavior is a separate scope decision.
