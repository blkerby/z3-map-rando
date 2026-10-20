use crate::DoorId;
use serde::{Deserialize, Serialize};
use type_hash::TypeHash;

/// A reward shared by logical generation, saved placements, and patching.
/// Progressive equipment grants one upgrade per pickup, independent of pickup order.
/// Inventory activation events and internal receipts are not placed rewards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TypeHash)]
#[serde(tag = "name")]
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
    BowAndSilverArrows,
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
    BossHeartContainer,
    SanctuaryHeartContainer,
    PieceOfHeart,
    HalfMagic,
    BombCapacityUpgrade,
    ArrowCapacityUpgrade,

    // Resource refills
    OneArrow,
    TenArrows,
    OneBomb,
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
