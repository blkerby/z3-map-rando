# Door-specific keys

Each Small Key unlocks one specific same-dungeon door and is a persistent inventory item, rather than a consumable dungeon resource. Opening one door cannot spend the means to open another. Small and Big Key pickups remain at their vanilla locations.

## Placement

Every complete generation attempt begins with a preliminary Small Key assignment pass, processing one dungeon at a time. This pass traverses the whole directed world while relaxing non-key requirements and other dungeons’ locks. It enforces the selected dungeon’s Small Key doors and Big Key requirements. Collectible unassigned keys are paired with distinct remaining doors as a batch; accept a candidate that exposes another key or makes every item in the dungeon collectible. A failed dungeon attempt restarts only that dungeon, preserving earlier dungeon assignments. The limits are 100 candidates per wave and 1000 attempts per dungeon.

The relaxed pass does not prove normal progression. Main placement starts with those keys, fixed Big Keys and fairy items, shuffled prizes, and empty ordinary slots. It samples from the full remaining ordinary pool in collectible slots, after prioritizing dungeon Maps and Compasses, and uses normal traversal closure to accept progression. There is no separate junk fill or backtracking. Rejected candidates preserve committed state. A failed complete attempt rerandomizes all keys before starting ordinary placement again; the generator permits 100 candidates per wave and 1000 complete attempts.

Completion uses the accepted closure’s collected-location bits after every slot is filled, including fixed key pickups, prizes, and fairy exchanges. No additional solve is needed.

## Logic and game representation

The [logic catalog](logic.md) retains stable authored door identities and lock types. `Item::SmallKey { door_id: DoorId }` stores the assigned source room and door ID directly in each saved placement. Collecting a key grants persistent access to that door. Big Keys remain scoped to their pickup’s dungeon. Save and quit retains inventory and permanent door access.

Normal traversal also tracks route-local resources, followers, and obstacles. The preliminary pass relaxes these requirements; normal item placement must still establish collection under the selected player settings.

The patcher reads typed items using only the current patch catalog. Small Key receipts and doors still behave as in vanilla; applying the saved door assignments to ROM behavior remains deferred.
