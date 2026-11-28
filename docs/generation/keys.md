# Door-specific keys

Each Small Key opens one lock in its own dungeon. Chest keys, enemy drops, pot keys, standing keys, and torch keys all announce the lock when collected. Touching a lock without its key displays a message once per approach. The HUD counts unused keys in the current dungeon. Small and Big Key pickups remain at their vanilla locations.

## Placement

The **Keys** generation setting selects the pairings. **Simple** follows a curated vanilla route, including keys for optional rooms and shortcuts. Within each dungeon, `door_id` refers to an upstream `lockedDoors[].id` and `simple_item_id` refers to an upstream `items[].id`. To exchange the destinations of two Simple keys, swap their `simple_item_id` values. Ordinary placement checks reachability with the resulting pairings on the selected map. Both modes use the same door `label`, written on one line of up to 19 mixed-case letters and spaces. The current ROM shows “THIS KEY OPENS” on the first row and the first 14 characters of the label in uppercase on the second. The third row is reserved for the dungeon name. [Paired key table](../../data/paired_keys.json)

**Random** is the default. Each complete generation attempt assigns Small Keys one dungeon at a time. This pass uses normal traversal and the regular interpreter with full non-key inventory from the configured pool, fixed items, and prizes. For the selected map, the generator computes one baseline closure with no dungeon keys and derives each dungeon’s initial state by granting access through the other dungeons’ locked doors. Each dungeon assignment starts from its own initial state. The selected dungeon's Small Keys and fixed Big Key must be collected normally. Player settings, resource preparation, flags, followers, and obstacles retain their normal behavior. Collectible unassigned keys are paired with distinct remaining doors as a batch; accept a candidate that exposes another key or makes every item in the dungeon collectible. A failed dungeon attempt restarts only that dungeon, preserving earlier dungeon assignments. The limits are 100 candidates per wave and 1000 attempts per dungeon.

The full-inventory pass establishes dungeon key access, but main placement must still establish item progression. Main placement starts with those keys, fixed Big Keys, shuffled prizes, and empty ordinary and Great Fairy slots. It samples from the full remaining ordinary pool in collectible slots, after prioritizing dungeon Maps and Compasses, and uses normal traversal closure to accept progression. There is no separate junk fill or backtracking. Rejected candidates preserve committed state. In Random mode a failed complete attempt rerandomizes all keys before starting ordinary placement again; the generator permits 100 candidates per wave and 1000 complete attempts.

Completion uses the accepted closure’s collected-location bits after every slot is filled, including fixed key pickups, prizes, and fairy exchanges. No additional solve is needed.

## Logic and game representation

The [logic catalog](logic.md) retains stable authored door identities and lock types. `Item::SmallKey(DoorId)` stores the assigned source room and door ID directly in each saved placement. Collecting a key grants persistent access to that door. Big Keys remain scoped to their pickup’s dungeon. Save and quit retains inventory and permanent door access.

Normal traversal also tracks route-local resources, followers, and obstacles. Both preliminary key assignment and ordinary item placement share this interpreter and retain their own global state, local-state reducers, and placements.

The patcher takes each key’s destination from the saved placement. The catalog builder uses the vanilla chest record’s item byte to identify a chest pickup and the upstream item’s `supertile` to identify a loose pickup. Each `lockedDoors[].faces` entry supplies the `supertile` and `doorIndex` for a native door record. All faces of a lock share the same key, receipt text, and ownership bit. The six Small Key faces on the backs of Big Key doors use the dungeon Big Key. Vanilla handles opening animations, open-door flags, staircases, and the dungeon HUD count.
