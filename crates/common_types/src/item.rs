use crate::DoorId;
use serde::{Deserialize, Serialize};
use type_hash::TypeHash;

/// An item shared by logical generation, saved placements, and patching.
/// Progressive equipment grants one upgrade per pickup, independent of pickup order.
/// Inventory activation events and internal receipts are not placed items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TypeHash)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    // Progressive equipment
    Sword,
    Shield,
    Glove,
    Mail,
    Boomerang,

    // Unique items
    FireRod,
    IceRod,
    Hammer,
    Hookshot,
    Bow,
    SilverArrows,
    MagicPowder,
    Bombos,
    Ether,
    Quake,
    Lamp,
    Shovel,
    Ocarina,
    CaneOfSomaria,
    CaneOfByrna,
    Cape,
    MagicMirror,
    BookOfMudora,
    Flippers,
    MoonPearl,
    BugCatchingNet,
    Mushroom,
    PegasusBoots,

    // Bottles and contents
    Bottle,
    Bee,
    Fairy,
    GoldBee,
    RedPotion,
    GreenPotion,
    BluePotion,

    // Health and resource upgrades
    HeartContainer,
    PieceOfHeart,
    HalfMagic,
    BombUpgrade5,
    BombUpgrade10,
    BombUpgrade50,
    ArrowUpgrade5,
    ArrowUpgrade10,
    ArrowUpgrade70,

    // Resource refills
    Arrow,
    TenArrows,
    Bomb,
    ThreeBombs,
    TenBombs,
    Heart,
    SmallMagic,

    // Currency
    OneRupee,
    FiveRupees,
    TwentyRupees,
    FiftyRupees,
    OneHundredRupees,
    ThreeHundredRupees,

    // Dungeon items
    Map,
    Compass,
    SmallKey { door_id: DoorId },
    BigKey,

    // Dungeon prizes
    PendantOfCourage,
    PendantOfWisdom,
    PendantOfPower,
    Crystal1,
    Crystal2,
    Crystal3,
    Crystal4,
    Crystal5,
    Crystal6,
    Crystal7,

    // Goal items
    Triforce,
}

/// A randomizer item that settings can put in the item pool, as defined in `data/items.json`.
/// Its other fields (count limits, display category, and placement tier) are not used by Rust yet.
#[derive(Clone, Debug, Deserialize)]
pub struct ItemRecord {
    pub name: Item,
    /// The z3-json-data item name used for this item's receipt when patching.
    pub upstream_name: String,
    /// Further z3-json-data item names that vanilla locations use for this item.
    pub other_names: Vec<String>,
}

/// Read the item records bundled from `data/items.json`.
pub fn read_item_records() -> Vec<ItemRecord> {
    serde_json::from_str(include_str!("../../../data/items.json")).unwrap()
}
