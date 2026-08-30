# Collectible items

The game does not have one unified item or item-location structure. Permanent
rewards normally converge on the item-receipt system, while ordinary drops,
pots, overworld secrets, shops, minigames, and several upgrades use related but
separate data. This document describes both the reward identities and the
structures which place and persist them.

## Item-receipt IDs

[`GrantItemReceipt`](../jpdasm/bank_07.asm#L5293) receives an ID in `Y`, copies
it to `$02D8`, and creates item-receipt ancilla `$22`.
[`$02E9`](../jpdasm/symbols_wram.asm#L1209) identifies how the item was
obtained:

| Value | Source |
| --- | --- |
| `$00` | Text, NPC, shop, or other ordinary source |
| `$01` | Chest |
| `$02` | Dropped boss heart or upgrade-fairy return |
| `$03` | Falling dungeon milestone |

[`AncillaAdd_ItemReceipt`](../jpdasm/bank_09.asm#L1335) grants the reward and
initializes its presentation. IDs `$00-$4B` index seven parallel tables
beginning at [`$098366`](../jpdasm/bank_09.asm#L753):

| Table | Address | Entry size | Purpose |
| --- | --- | --- | --- |
| `offset_y` | `$098366` | 1 byte | Vertical drawing offset |
| `offset_x` | `$0983B2` | 1 byte | Horizontal drawing offset |
| `item_gfx_index` | `$0983FE` | 1 byte | Item-graphics decompression index |
| `width` | `$09844A` | 1 byte | OAM size/layout selector |
| `prop` | `$098496` | 1 byte | OAM palette/property selector |
| `sram_write` | `$0984E2` | 2 bytes | Low 16 bits of the `$7E:Fxxx` inventory destination |
| `sram_value` | `$09857A` | 1 byte | Value to store; negative values skip the generic store |

The item-receipt ancilla has another parallel table at
[`$08C301`](../jpdasm/bank_08.asm#L13400) containing a two-byte message ID per
receipt ID; `$FFFF` means no message. It also supplies special animation and
message behavior for rupees, heart pieces, pendants, crystals, and swords.

The complete receipt-ID namespace is:

| ID | Reward | Saved state or effect |
| --- | --- | --- |
| `$00` | Fighter Sword | Sword level `$7EF359 = 1`; this grant also writes the Fighter Shield, and vanilla uses it for the uncle's equipment bundle. |
| `$01` | Master Sword | Sword level `$7EF359 = 2` |
| `$02` | Tempered Sword | Sword level `$7EF359 = 3` |
| `$03` | Golden Sword (`BUTTER SWORD` in jpdasm) | Sword level `$7EF359 = 4` |
| `$04` | Fighter Shield | Shield level `$7EF35A = 1` |
| `$05` | Fire Shield | Shield level `$7EF35A = 2` |
| `$06` | Mirror Shield | Shield level `$7EF35A = 3` |
| `$07` | Fire Rod | `$7EF345 = 1` |
| `$08` | Ice Rod | `$7EF346 = 1` |
| `$09` | Magic Hammer | `$7EF34B = 1` |
| `$0A` | Hookshot | `$7EF342 = 1` |
| `$0B` | Bow | Bow state `$7EF340 = 1` |
| `$0C` | Blue Boomerang | Boomerang level `$7EF341 = 1` |
| `$0D` | Magic Powder | Mushroom/powder state `$7EF344 = 2` |
| `$0E` | Bee refill | Puts a bee in an existing empty bottle |
| `$0F` | Bombos | `$7EF347 = 1` |
| `$10` | Ether | `$7EF348 = 1` |
| `$11` | Quake | `$7EF349 = 1` |
| `$12` | Lamp | `$7EF34A = 1` |
| `$13` | Shovel | Shovel/flute state `$7EF34C = 1` |
| `$14` | Inactive Flute | Shovel/flute state `$7EF34C = 2` |
| `$15` | Cane of Somaria | `$7EF350 = 1` |
| `$16` | Empty Bottle | Creates an empty bottle in the first unused bottle slot |
| `$17` | Piece of Heart | Increments `$7EF36B` modulo four; the fourth piece grants receipt `$26` |
| `$18` | Cane of Byrna | `$7EF351 = 1` |
| `$19` | Magic Cape | `$7EF352 = 1` |
| `$1A` | Magic Mirror | Mirror state `$7EF353 = 2` |
| `$1B` | Power Glove | Glove level `$7EF354 = 1` |
| `$1C` | Titan's Mitt | Glove level `$7EF354 = 2` |
| `$1D` | Book of Mudora | `$7EF34E = 1` |
| `$1E` | Flippers | `$7EF356 = 1`; also sets the swim bit in `$7EF379` |
| `$1F` | Moon Pearl | `$7EF357 = 1` and restores human graphics |
| `$20` | Crystal | Sets the current dungeon's bit in `$7EF37A` during the rising-crystal sequence |
| `$21` | Bug-Catching Net | `$7EF34D = 1` |
| `$22` | Blue Mail | Armor level `$7EF35B = 1`, unless better armor is already owned |
| `$23` | Red Mail | Armor level `$7EF35B = 2` |
| `$24` | Small Key | Adds one to current-dungeon keys `$7EF36F`, capped at 99 |
| `$25` | Compass | Sets the current dungeon's bit in `$7EF364-$7EF365` |
| `$26` | Heart Container from four pieces | Adds eight to maximum health `$7EF36C` and heals the difference |
| `$27` | One Bomb | Queues one bomb in `$7EF375` |
| `$28` | Three Bombs | Queues three bombs in `$7EF375` |
| `$29` | Mushroom | Sets `$7EF344 = 1`, but does not replace powder |
| `$2A` | Red Boomerang | Boomerang level `$7EF341 = 2` |
| `$2B` | New Bottle with Red Potion | Creates bottle value `3` in the first unused bottle slot |
| `$2C` | New Bottle with Green Potion | Creates bottle value `4` in the first unused bottle slot |
| `$2D` | New Bottle with Blue Potion | Creates bottle value `5` in the first unused bottle slot |
| `$2E` | Red Potion refill | Replaces the first empty bottle with bottle value `3` |
| `$2F` | Green Potion refill | Replaces the first empty bottle with bottle value `4` |
| `$30` | Blue Potion refill | Replaces the first empty bottle with bottle value `5` |
| `$31` | Ten Bombs | Queues ten bombs in `$7EF375` |
| `$32` | Big Key | Sets the current dungeon's bit in `$7EF366-$7EF367` |
| `$33` | Dungeon Map | Sets the current dungeon's bit in `$7EF368-$7EF369` |
| `$34` | One Rupee | Adds 1 to `$7EF360` |
| `$35` | Five Rupees | Adds 5 to `$7EF360` |
| `$36` | Twenty Rupees | Adds 20 to `$7EF360` |
| `$37` | Green Pendant / Courage | Sets bit `$04` in `$7EF374` |
| `$38` | Red Pendant / Wisdom | Sets bit `$01` in `$7EF374` |
| `$39` | Blue Pendant / Power | Sets bit `$02` in `$7EF374` |
| `$3A` | Tossed Bow | Bow state `$7EF340 = 1`; used by fairy item exchange |
| `$3B` | Silver Arrows | Bow state `$7EF340 = 3` |
| `$3C` | New Bottle with Bee | Creates bottle value `7` in the first unused bottle slot |
| `$3D` | New Bottle with Fairy | Creates bottle value `6` in the first unused bottle slot |
| `$3E` | Boss Heart Container | Adds eight to maximum health and heals; normally collected from sprite `$EA` |
| `$3F` | Sanctuary Heart Container | Adds eight to maximum health and heals |
| `$40` | 100 Rupees | Adds 100 to `$7EF360` |
| `$41` | 50 Rupees | Adds 50 to `$7EF360` |
| `$42` | One Heart | Queues eight HP in `$7EF372` |
| `$43` | One Arrow | Queues one arrow in `$7EF376` |
| `$44` | Ten Arrows | Queues ten arrows in `$7EF376` |
| `$45` | Small Magic | Queues `$10` magic in `$7EF373` |
| `$46` | 300 Rupees | Adds 300 to `$7EF360` |
| `$47` | Twenty Rupees, green graphic | Adds 20 to `$7EF360` |
| `$48` | New Bottle with Good Bee | Creates bottle value `8` in the first unused bottle slot |
| `$49` | Tossed Fighter Sword | Sword level `$7EF359 = 1`; used by fairy item exchange |
| `$4A` | Activated Flute | Shovel/flute state `$7EF34C = 3` |
| `$4B` | Pegasus Boots | `$7EF355 = 1`; also sets the run bit in `$7EF379` |

The red and blue pendant comments are inconsistent inside jpdasm. The
authoritative behavior is the bit selected in
[`AncillaAdd_ItemReceipt`](../jpdasm/bank_09.asm#L1464), together with the
documented [`PENDANTS`](../jpdasm/symbols_sram.asm#L781) bitfield and dungeon
prize assignment. Those establish `$38` as red/Wisdom and `$39` as blue/Power.

### Special receipt handling

- [`ItemReceipt_GiveBottledItem`](../jpdasm/bank_09.asm#L2104) maps the seven
  new-bottle IDs to bottle values `2-8`, and separately maps potion/bee refill
  IDs to values `3-5` and `7`. New-bottle rewards require an unused slot
  (`0` or `1`); refills require an existing empty bottle (`2`).
- [`Ancilla_AddRupees`](../jpdasm/bank_09.asm#L9164) handles the seven rupee
  receipt IDs rather than using their generic `sram_value` entries.
- Compass, big-key, and map receipts OR a dungeon mask into their two-byte
  bitfields. Small keys instead update the current key count, which is later
  mirrored to the per-dungeon key bytes at `$7EF37C-$7EF389`.
- Chests substitute an overflow reward only for an already-owned blue
  boomerang (ten arrows), lamp (five rupees), or red boomerang (300 rupees),
  using [`PerformOpenChest.overflow_replacement`](../jpdasm/bank_07.asm#L10977).
- `$7EF340-$7EF389` is part of the save-file mirror beginning at
  [`$7EF000`](../jpdasm/symbols_wram.asm#L5937). The detailed inventory field
  values are documented in
  [`symbols_sram.asm`](../jpdasm/symbols_sram.asm#L565).

## Permanent upgrades outside the receipt-ID namespace

Three collectible upgrades modify save data directly and have no distinct
item-receipt ID:

| Upgrade | Data |
| --- | --- |
| Bomb capacity | The Lake Hylia fairy increments `$7EF370`. Indexes `0-7` select capacities `10, 15, 20, 25, 30, 35, 40, 50`. |
| Arrow capacity | The fairy increments `$7EF371`. Indexes `0-7` select capacities `30, 35, 40, 45, 50, 55, 60, 70`. |
| Half magic | The magic bat sets `$7EF37B = 1`. (`2` is supported as quarter magic but is not a normal collectible.) |

The capacity arrays are [`CapacityUpgrades`](../jpdasm/bank_0D.asm#L13482),
their grants are in
[`LakeHyliaFairy_UpgradeBombs`](../jpdasm/bank_06.asm#L13669) and
[`LakeHyliaFairy_UpgradeArrows`](../jpdasm/bank_06.asm#L13780), and the magic
upgrade is in [`MagicBat_EnhanceMagic`](../jpdasm/bank_05.asm#L22727).

## Ordinary collectible sprites

Short-lived refills and enemy drops use sprite IDs, not receipt IDs.
[`GetAbsorbedAsItem`](../jpdasm/bank_06.asm#L15603) dispatches this contiguous
range directly:

| Sprite | Collectible | Effect |
| --- | --- | --- |
| `$D8` | Heart | Eight HP |
| `$D9` | Green rupee | 1 rupee |
| `$DA` | Blue rupee | 5 rupees |
| `$DB` | Red rupee | 20 rupees |
| `$DC` | Bomb refill 1 | 1 bomb |
| `$DD` | Bomb refill 4 | 4 bombs |
| `$DE` | Bomb refill 8 | 8 bombs |
| `$DF` | Small magic decanter | `$10` magic |
| `$E0` | Large magic decanter | Full magic (`$80`) |
| `$E1` | Arrow refill 5 | 5 arrows, or an override stored in sprite state |
| `$E2` | Arrow refill 10 | 10 arrows |
| `$E3` | Fairy | Seven hearts on contact, or bottle value `6` when caught with the net |
| `$E4` | Small Key | One current-dungeon key and the room's small-key flag |
| `$E5` | Big Key | Grants receipt `$32` and sets the room's big-key flag |
| `$E6` | Stolen Shield | Restores the shield level cached by a Pikit |

Enemies select one of seven eight-entry sequences from
[`PrizePackPrizes`](../jpdasm/bank_06.asm#L24639). Each enemy has a prize-pack
number; the game applies that pack's 50% or 100% drop rate, advances a shared
position within the pack, and transforms the dying enemy into the selected
collectible sprite. Forced small-key, big-key, and green-rupee drops bypass the
pack choice. These transient drops are resources, not unique item locations.

## Item-location structures

### Chests

[`RoomData_ChestItems`](../jpdasm/bank_01.asm#L19731) is a flat table of 168
three-byte records:

```text
word room_id_and_big_chest
byte item_receipt_id
```

Bit 15 of the room word marks a big chest. There is no coordinate in this
table. [`OpenItemChest`](../jpdasm/bank_01.asm#L19917) determines which chest in
the current room was touched from the room tilemap/object state, then scans the
flat table for the corresponding occurrence of that room ID. Consequently,
the order of repeated room IDs is part of the format.

Chest persistence uses the six chest bits in the room flag word. The live
`TAKEN` byte at `$0403` is saved into the two-byte room record at
`$7EF000 + 2 * room_id`; its layout is documented in
[`symbols_sram.asm`](../jpdasm/symbols_sram.asm#L48). Chests therefore have an
implicit identity of `(room ID, chest ordinal)`, not a globally stored location
ID.

Chest-game chests are separate. Their rewards come from
[`VoOChestGamePrizes`](../jpdasm/bank_01.asm#L20332) and
[`OtherChestGamePrizes`](../jpdasm/bank_01.asm#L20558), selected with random and
game-specific state rather than `RoomData_ChestItems`.

### Underworld pots and liftable secrets

[`RoomData_PotItems_Pointers`](../jpdasm/bank_01.asm#L17083) contains 320
pointers, one for each room `$0000-$013F`. Each pointed-to list has three-byte
records terminated by a `$FFFF` word:

```text
word packed tilemap coordinate and layer
byte secret_id
```

[`RevealPotItem`](../jpdasm/bank_01.asm#L19209) recognizes these item-like IDs:

| Secret ID | Result |
| --- | --- |
| `$01` | Green rupee |
| `$07` | Blue rupee |
| `$08` | Small key |
| `$09` | Five arrows |
| `$0A` | One bomb |
| `$0B` | Heart |
| `$0C` | Small magic |
| `$0D` | Full magic |
| `$0E` | Cucco, not a collectible |
| `$80` | Hole, not a collectible |
| `$88` | Switch, not a collectible |

Non-key pickups use one bit per list entry in the transient
[`POTLIFT`](../jpdasm/symbols_wram.asm#L5945) table at `$7EF580`; it prevents a
revealed pot item from being spawned twice while that tracking remains active.
Small keys instead use the persistent room key flag. `POTLIFT` is outside the
save-file block and is cleared by `ResetPotTracking`, so ordinary pot contents
are not permanent randomizer locations.

### Overworld secrets

[`OverworldData_HiddenItems`](../jpdasm/bank_1B.asm#L13350) is a 128-entry
pointer table indexed by overworld screen `$00-$7F`. Large screens repeat the
same pointer for each constituent screen ID. Each list uses the same basic
three-byte shape as pot data and ends with `$FFFF`:

```text
word local map coordinate
byte secret_id
```

The listed IDs are `$01` green rupee, `$02` hoarder, `$03` bee, `$04` random
pack, `$05` bomb, `$06` heart, `$80` hole, `$82` warp, `$84` staircase, and
`$86` bomb door. [`RevealOverworldSecret`](../jpdasm/bank_1B.asm#L14303)
returns item/spawn IDs below `$80` through `$0B9C`; values at or above `$80`
select a terrain reveal. Ordinary outdoor item secrets only have a 50% spawn
chance and are not saved as unique collections.

Both overworld and underworld secret IDs feed
[`SpawnSecret`](../jpdasm/bank_06.asm#L456). Its full `$01-$16` namespace also
contains alternate random-bush outcomes and non-items: green/blue rupees,
bombs, hearts, small/full magic, five arrows, small key, fairy, bee, hoarder,
Cucco, guards, bush Stalfos, landmine, and null. Outdoor random-pack ID `$04`
randomly selects IDs `$13-$16` (green rupee, fairy, heart, or null).

### Freestanding sprites and dungeon milestones

Heart containers and pieces use sprites `$EA` and `$EB`.
[`SpritePrep_HeartContainer`](../jpdasm/bank_05.asm#L20178) suppresses an
already-collected sprite by checking either an overworld screen flag or one of
the room key/heart bits. Boss heart containers grant receipt `$3E`; ordinary
heart containers grant `$26`. Heart-piece sprites increment `$7EF36B` directly
and grant `$26` on the fourth piece. Their collection flag uses the same
overworld or room bits as heart containers.

Boss pendants and crystals use the falling-prize system rather than ordinary
sprite placement. A per-dungeon table in
[`RoomTag_GetHeartForPrize`](../jpdasm/bank_01.asm#L13257) selects one of seven
prize indexes. [`AncillaAdd_FallingPrize`](../jpdasm/bank_09.asm#L2555) maps
those indexes to Ether, the three pendants, a heart container, Bombos, or a
crystal and supplies fixed fall coordinates, heights, and timers. Pendant and
crystal ownership bitfields prevent the prize from respawning.

Other freestanding equipment and NPC rewards are implemented by their owning
sprite or script and normally contain a hard-coded receipt ID. Their location
persistence is likewise bespoke: a room bit, overworld screen bit, progression
flag, or inventory check. There is no ROM table enumerating all such locations.

### Shops and minigames

Shops are sprite behavior, not declarative item data. The room-specific setup
in [`SpritePrep_Shopkeeper`](../jpdasm/bank_06.asm#L2564) spawns one of seven
shop-item subtypes. [`Sprite_BB_Shopkeeper`](../jpdasm/bank_1E.asm#L19386)
hard-codes each subtype's price, eligibility checks, receipt ID, message, and
OAM group. Vanilla stock consists of red potion refill, Fighter Shield, Fire
Shield, heart, ten arrows, ten bombs, and bee refill.

Minigames also use their own structures. Chest games use the receipt-ID arrays
described above; archery uses
[`ArcheryGamePrizes`](../jpdasm/bank_05.asm#L768) to add rupees directly; and
the digging game uses [`SpawnDiggingGamePrize`](../jpdasm/bank_1D.asm#L21481)
to spawn ordinary refill sprites or a one-time heart-piece sprite.

## Implications for randomization

The receipt ID is a useful common reward representation, but it is not a
location representation. A complete item randomizer must give each eligible
location its own stable identity and adapt each carrier separately:

- chest records can directly hold receipt IDs;
- freestanding/NPC/shop rewards need their hard-coded grants redirected;
- dungeon milestones need their prize-index mapping replaced or bypassed;
- pot, secret, and ordinary drop IDs belong to the sprite/secret namespace and
  need conversion if they are promoted to permanent item locations;
- each converted location needs a persistent collected flag independent of the
  inventory field changed by its reward.

This separation also permits transient resources and repeatable purchases to
remain outside the randomized location pool without inventing a second item
identity for them.
