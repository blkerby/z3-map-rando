# Prototype gameplay

The prototype randomizes items and overworld layouts, with broader gameplay verification still outstanding. The following item and restart behavior applies to patched seeds.

## Starting state

Starting state has independent equipment and resource settings. Unique equipment uses toggles and progressive equipment uses named tiers. Each of the four bottle slots can start missing, empty, or filled with a potion, fairy, bee, or golden bee; logic counts every starting bottle as owned but does not credit its contents. Heart capacity is 1 to 20 whole hearts; a new game begins at full health. Bomb and arrow capacities are direct values from 0 to 99, with separate starting ammunition counts bounded by their capacities. Starting rupees range from 0 to 999. New games begin with Zelda already rescued; completing her rescue does not reset later progress.

The Default preset starts with no equipment, three hearts, bomb/arrow capacities of 10/30, and no ammunition or rupees. All equipment supplies the highest equipment tiers, an active Ocarina, and four empty bottles, while keeping Default's resources. The Testing root preset overrides the capacities to 5/5.

Starting ammunition is a one-time supply and is ignored by generation logic. Logic counts bomb and arrow ammunition only after obtaining a refill source.

## Items

Swords, shields, gloves, mail, boomerangs, and bows upgrade one level per pickup, capped at the highest tier. Bomb and arrow capacity upgrades grant the separately configured amounts of capacity and ammunition, defaulting to 5 each, with capacity capped at 99. The Lake Hylia fairy fills an empty bottle with a fairy or golden bee, for free and any number of times. Visible pickups show the next upgrade. When both are owned, highlight Mushroom/Powder or Shovel/Ocarina in the item menu and press Y to switch between them.

Small and Big Keys remain at vanilla key locations. Keys still use vanilla ROM behavior, which differs from the logical model's door-specific Small Keys. The bow is progressive: the first pickup grants the Bow and the second the Silver Arrows. The Waterfall of Wishing and Pyramid fairies each offer one upgrade of a boomerang, mail, shield, sword, bow, gloves, bombs, or arrows: throw in that item, and she returns something better once, if it is not already at its highest level. Equipment receipt messages retain vanilla text pending replacement.

Item pool counts describe pickups before optional deduction. Deduction is enabled by default and removes starting equipment: owning a unique item deducts one pickup, and progressive tiers deduct the number of upgrades represented by that tier. Bottles, health, capacities, ammunition, rupees, and flags do not deduct pickups. Starting equipment must fit the corresponding pool counts when deduction is enabled.

Item pools after deduction may contain fewer pickups than the available ordinary locations; the remaining slots are empty. Empty chests open without granting an item, NPC rewards retain their collection state, and empty freestanding pickups are suppressed. Empty boss-heart rewards still release the dungeon prize. Waterfall and Pyramid fairies with empty rewards say “Sorry we're closed” and end the interaction without summoning a fairy or throwing an item.

## Saving and restarting

Save-and-quit and outdoor/cave deaths lose every follower and return to the Light World before Agahnim is defeated, regardless of Mirror ownership. After defeating Agahnim, they retain the current world. Light World restarts offer Link's House, Sanctuary after entering it, and Mountain Cave after visiting the Tower of Hera overworld area (area ID $03). Dark World restarts offer the Pyramid, plus Link's House when the Mirror is missing.

The menu uses English labels and substitutes the saved player name in the house option. A single available destination loads directly. Dungeon deaths retain their entrance restart and vanilla follower handling, including retaining Zelda or the Old Man; Ganon's room uses outdoor recovery and loses every follower.

The Sanctuary priest remains alive and available for healing and Zelda's rescue regardless of sword level or Agahnim progress.
