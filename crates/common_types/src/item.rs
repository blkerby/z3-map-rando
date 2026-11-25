use crate::{DoorId, SourceRoomId};
use serde::{Deserialize, Serialize};

/// A kind of item, without the identity of a particular placed item. Item records, the item
/// pool, logic item definitions, and receipt encodings use kinds; serialized names match
/// [`Item`]'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    // Progressive equipment
    Sword,
    Shield,
    Glove,
    Mail,
    Boomerang,
    /// The Bow, then Silver Arrows.
    Bow,

    // Unique items
    FireRod,
    IceRod,
    Hammer,
    Hookshot,
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
    BombCapacityUpgrade,
    ArrowCapacityUpgrade,

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
    SmallKey,
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

    /// A finalized empty location; grants no inventory.
    Nothing,
}

/// An item placed at a location, shared by logical generation, saved placements, and
/// patching. Dungeon items identify their dungeon by its room ID.
/// Progressive equipment grants one upgrade per pickup, independent of pickup order.
/// Inventory activation events and internal receipts are not placed items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    // Progressive equipment
    Sword,
    Shield,
    Glove,
    Mail,
    Boomerang,
    /// The Bow, then Silver Arrows.
    Bow,

    // Unique items
    FireRod,
    IceRod,
    Hammer,
    Hookshot,
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
    BombCapacityUpgrade,
    ArrowCapacityUpgrade,

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
    /// The map of the dungeon with this room ID.
    Map(SourceRoomId),
    /// The compass of the dungeon with this room ID.
    Compass(SourceRoomId),
    /// A Small Key opening the assigned door.
    SmallKey(DoorId),
    /// The Big Key of the dungeon with this room ID.
    BigKey(SourceRoomId),

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

    /// A finalized empty location; grants no inventory.
    Nothing,
}

impl Item {
    pub fn get_kind(self) -> ItemKind {
        match self {
            Item::Sword => ItemKind::Sword,
            Item::Shield => ItemKind::Shield,
            Item::Glove => ItemKind::Glove,
            Item::Mail => ItemKind::Mail,
            Item::Boomerang => ItemKind::Boomerang,
            Item::Bow => ItemKind::Bow,
            Item::FireRod => ItemKind::FireRod,
            Item::IceRod => ItemKind::IceRod,
            Item::Hammer => ItemKind::Hammer,
            Item::Hookshot => ItemKind::Hookshot,
            Item::MagicPowder => ItemKind::MagicPowder,
            Item::Bombos => ItemKind::Bombos,
            Item::Ether => ItemKind::Ether,
            Item::Quake => ItemKind::Quake,
            Item::Lamp => ItemKind::Lamp,
            Item::Shovel => ItemKind::Shovel,
            Item::Ocarina => ItemKind::Ocarina,
            Item::CaneOfSomaria => ItemKind::CaneOfSomaria,
            Item::CaneOfByrna => ItemKind::CaneOfByrna,
            Item::Cape => ItemKind::Cape,
            Item::MagicMirror => ItemKind::MagicMirror,
            Item::BookOfMudora => ItemKind::BookOfMudora,
            Item::Flippers => ItemKind::Flippers,
            Item::MoonPearl => ItemKind::MoonPearl,
            Item::BugCatchingNet => ItemKind::BugCatchingNet,
            Item::Mushroom => ItemKind::Mushroom,
            Item::PegasusBoots => ItemKind::PegasusBoots,
            Item::Bottle => ItemKind::Bottle,
            Item::Bee => ItemKind::Bee,
            Item::Fairy => ItemKind::Fairy,
            Item::GoldBee => ItemKind::GoldBee,
            Item::RedPotion => ItemKind::RedPotion,
            Item::GreenPotion => ItemKind::GreenPotion,
            Item::BluePotion => ItemKind::BluePotion,
            Item::HeartContainer => ItemKind::HeartContainer,
            Item::PieceOfHeart => ItemKind::PieceOfHeart,
            Item::HalfMagic => ItemKind::HalfMagic,
            Item::BombCapacityUpgrade => ItemKind::BombCapacityUpgrade,
            Item::ArrowCapacityUpgrade => ItemKind::ArrowCapacityUpgrade,
            Item::Arrow => ItemKind::Arrow,
            Item::TenArrows => ItemKind::TenArrows,
            Item::Bomb => ItemKind::Bomb,
            Item::ThreeBombs => ItemKind::ThreeBombs,
            Item::TenBombs => ItemKind::TenBombs,
            Item::Heart => ItemKind::Heart,
            Item::SmallMagic => ItemKind::SmallMagic,
            Item::OneRupee => ItemKind::OneRupee,
            Item::FiveRupees => ItemKind::FiveRupees,
            Item::TwentyRupees => ItemKind::TwentyRupees,
            Item::FiftyRupees => ItemKind::FiftyRupees,
            Item::OneHundredRupees => ItemKind::OneHundredRupees,
            Item::ThreeHundredRupees => ItemKind::ThreeHundredRupees,
            Item::Map(_) => ItemKind::Map,
            Item::Compass(_) => ItemKind::Compass,
            Item::SmallKey(_) => ItemKind::SmallKey,
            Item::BigKey(_) => ItemKind::BigKey,
            Item::PendantOfCourage => ItemKind::PendantOfCourage,
            Item::PendantOfWisdom => ItemKind::PendantOfWisdom,
            Item::PendantOfPower => ItemKind::PendantOfPower,
            Item::Crystal1 => ItemKind::Crystal1,
            Item::Crystal2 => ItemKind::Crystal2,
            Item::Crystal3 => ItemKind::Crystal3,
            Item::Crystal4 => ItemKind::Crystal4,
            Item::Crystal5 => ItemKind::Crystal5,
            Item::Crystal6 => ItemKind::Crystal6,
            Item::Crystal7 => ItemKind::Crystal7,
            Item::Triforce => ItemKind::Triforce,
            Item::Nothing => ItemKind::Nothing,
        }
    }

    /// The item of a kind that needs no identity; None for dungeon items.
    pub fn from_kind(kind: ItemKind) -> Option<Item> {
        Some(match kind {
            ItemKind::Sword => Item::Sword,
            ItemKind::Shield => Item::Shield,
            ItemKind::Glove => Item::Glove,
            ItemKind::Mail => Item::Mail,
            ItemKind::Boomerang => Item::Boomerang,
            ItemKind::Bow => Item::Bow,
            ItemKind::FireRod => Item::FireRod,
            ItemKind::IceRod => Item::IceRod,
            ItemKind::Hammer => Item::Hammer,
            ItemKind::Hookshot => Item::Hookshot,
            ItemKind::MagicPowder => Item::MagicPowder,
            ItemKind::Bombos => Item::Bombos,
            ItemKind::Ether => Item::Ether,
            ItemKind::Quake => Item::Quake,
            ItemKind::Lamp => Item::Lamp,
            ItemKind::Shovel => Item::Shovel,
            ItemKind::Ocarina => Item::Ocarina,
            ItemKind::CaneOfSomaria => Item::CaneOfSomaria,
            ItemKind::CaneOfByrna => Item::CaneOfByrna,
            ItemKind::Cape => Item::Cape,
            ItemKind::MagicMirror => Item::MagicMirror,
            ItemKind::BookOfMudora => Item::BookOfMudora,
            ItemKind::Flippers => Item::Flippers,
            ItemKind::MoonPearl => Item::MoonPearl,
            ItemKind::BugCatchingNet => Item::BugCatchingNet,
            ItemKind::Mushroom => Item::Mushroom,
            ItemKind::PegasusBoots => Item::PegasusBoots,
            ItemKind::Bottle => Item::Bottle,
            ItemKind::Bee => Item::Bee,
            ItemKind::Fairy => Item::Fairy,
            ItemKind::GoldBee => Item::GoldBee,
            ItemKind::RedPotion => Item::RedPotion,
            ItemKind::GreenPotion => Item::GreenPotion,
            ItemKind::BluePotion => Item::BluePotion,
            ItemKind::HeartContainer => Item::HeartContainer,
            ItemKind::PieceOfHeart => Item::PieceOfHeart,
            ItemKind::HalfMagic => Item::HalfMagic,
            ItemKind::BombCapacityUpgrade => Item::BombCapacityUpgrade,
            ItemKind::ArrowCapacityUpgrade => Item::ArrowCapacityUpgrade,
            ItemKind::Arrow => Item::Arrow,
            ItemKind::TenArrows => Item::TenArrows,
            ItemKind::Bomb => Item::Bomb,
            ItemKind::ThreeBombs => Item::ThreeBombs,
            ItemKind::TenBombs => Item::TenBombs,
            ItemKind::Heart => Item::Heart,
            ItemKind::SmallMagic => Item::SmallMagic,
            ItemKind::OneRupee => Item::OneRupee,
            ItemKind::FiveRupees => Item::FiveRupees,
            ItemKind::TwentyRupees => Item::TwentyRupees,
            ItemKind::FiftyRupees => Item::FiftyRupees,
            ItemKind::OneHundredRupees => Item::OneHundredRupees,
            ItemKind::ThreeHundredRupees => Item::ThreeHundredRupees,
            ItemKind::Map => return None,
            ItemKind::Compass => return None,
            ItemKind::SmallKey => return None,
            ItemKind::BigKey => return None,
            ItemKind::PendantOfCourage => Item::PendantOfCourage,
            ItemKind::PendantOfWisdom => Item::PendantOfWisdom,
            ItemKind::PendantOfPower => Item::PendantOfPower,
            ItemKind::Crystal1 => Item::Crystal1,
            ItemKind::Crystal2 => Item::Crystal2,
            ItemKind::Crystal3 => Item::Crystal3,
            ItemKind::Crystal4 => Item::Crystal4,
            ItemKind::Crystal5 => Item::Crystal5,
            ItemKind::Crystal6 => Item::Crystal6,
            ItemKind::Crystal7 => Item::Crystal7,
            ItemKind::Triforce => Item::Triforce,
            ItemKind::Nothing => Item::Nothing,
        })
    }
}
