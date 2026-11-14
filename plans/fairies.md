# Great Fairy upgrades

Status: the Lake Hylia fairy and starting items are implemented; the Waterfall and Pyramid fairies are planned. This replaces the vanilla item exchanges at the Waterfall of Wishing and Pyramid fairies, and the capacity shop at the Lake Hylia fairy. All three use sprite `$72` (`Sprite_72_FairyPond`); see [items.md](items.md) for the vanilla receipt system.

## Waterfall and Pyramid fairies

Each fairy offers a single upgrade for one of eight item classes, selected during placement. The upgrade can be received once per fairy. Both fairies may offer the same class when it has enough upgrade levels.

| Class | Required to upgrade | Received item | Maxed when |
| --- | --- | --- | --- |
| boomerang | Blue Boomerang | `Boomerang` | Red Boomerang |
| mail | Blue Mail | `Mail` | Red Mail |
| shield | Fighter Shield | `Shield` | Mirror Shield |
| sword | Fighter Sword | `Sword` | Golden Sword |
| bow | Bow | `SilverArrows` | Silver Arrows owned |
| glove | Power Glove | `Glove` | Titan's Mitt |
| bomb | Nonzero bomb capacity | `BombCapacityUpgrade` | Capacity 99 |
| arrow | Nonzero arrow capacity | `ArrowCapacityUpgrade` | Capacity 99 |

Green Mail does not count as having mail. Bomb and arrow capacity are nonzero from the start unless the [starting capacity settings](#starting-items-settings) override them to zero, in which case the upgrade is refused until a capacity upgrade is collected elsewhere.

### Dialogue

Simplify the vanilla sequence (`UpgradeFairy_*` in `bank_06.asm`), dropping the item-selection menu and the liar branches:

1. On contact, she asks "Throw <item>?" with "Yes"/"No" options, naming the fairy's class. "No" ends the sequence immediately.
2. If Link has none of the class, she says "Come back when you have <item>." and the sequence ends.
3. If the class is not maxed and her upgrade is unused, Link throws the item in, and she says "I see you dropped your <item>. I will give you something better." She then grants the placed item through a regular item receipt, so progressive receipts resolve to the next tier. Mark her upgrade as used in a saved bit.
4. If the class is maxed or her upgrade is already used, Link throws the item in, and she says "I see you dropped your <item>. I will give it back." No item is displayed or granted.

The thrown item keeps its inventory value; only the toss animation shows it leaving. The receipt performs the upgrade. The text uses custom English messages with the class name substituted, rendered like the existing English spawn menu.

### Placement

Use item 1 of the Waterfall of Wishing (room 12) and of the Pyramid Great Fairy (room 69) as the two fairy locations, and retire both item 2 locations without reusing their IDs. Saved placements hold the received item, for example `Item::Sword`. The patcher writes each fairy's class to a ROM table from the placements, and the catalog stops clearing these locations' ROM addresses.

Remove the fixed fairy-exchange handling from `crates/generator/src/inputs.rs`. In each wave, the generator attempts the fairy locations first:

- A fairy location can receive one of the eight class items if the fairy is reachable, an instance remains in the pool, and the minimum level required for that class was collectible in a previous step.
- If no class is eligible, defer the location to a later wave.
- When filling other locations, keep at least as many class items in the pool as there are unfilled fairy locations.

The logical requirement for collecting a fairy location is the class requirement in the table above, replacing the z3-json-data strat requirements (`Boomerang`, `shieldExact`, or none).

The four formerly fixed items return to the pool: sword 4, shield 3, boomerang 2, and silver_arrows 1. Two Twenty Rupees are removed to keep the pool equal to the location count, since four fixed locations become two ordinary ones.

## Lake Hylia fairy

Implemented in `fairy_ponds.asm`, replacing the vanilla capacity shop. The fairy does not appear; the sequence runs from the pond.

1. On contact, she says "Throw in a bottle and I will make your wishes come true." with "Yes"/"No" options. "No" ends the sequence immediately.
2. If "Yes", she asks "Wish for" with options "Fairy" and "Golden bee". Choice text starts after the vanilla arrow in column 4, leaving 10 characters.
3. If Link has an empty bottle, he throws it in and receives the chosen bottle contents through an item receipt. Otherwise she says "Come back when you have an empty bottle."

The exchange is free and repeatable without limit. Like vanilla, the offer repeats after a few seconds while Link stays in contact.

### Receipts

The internal receipts `$B8` (fairy) and `$B9` (golden bee), unused by ALTTPR, refill the first empty bottle. They are not placeable, so they have no `Item` variant or `items.json` entry.

Vanilla `ItemReceipt_GiveBottledItem` refills the first empty bottle with value *index + 3* for each receipt in its refill list. The vanilla list leaves the fairy entry (value 6) unused and has no golden bee entry (value 8). `item_receipt_tables.inc` relocates the list to `BottleRefillReceipts` with both receipts, and raises the loop's starting index at `$09895A` from 4 to 5.

Their receipt table entries reuse the graphics, palette, and width of `$3D` (new bottle with fairy) and `$48` (new bottle with golden bee), with `$FF` values to skip the generic inventory write and no message. The toss animation leaves the bottle's inventory value unchanged.

### Logic

The logic builder adds a Lake Hylia Great Fairy entrance-node edge with `ExchangeBottleForFairy`, which provides bottled fairies without the Bug Catching Net. Golden bees have no logical use, so they are not modeled.

## Starting items settings

Implemented. The `starting_items` settings contain `bomb_capacity` and `arrow_capacity`, defaulting to the vanilla 10 and 30 and allowing 0 to 99. The generator's initial capacities use these values. The patcher writes them to the exported `StartingCapacities`, which `InitializeGameState` in `new_game.asm` copies into a new save.
