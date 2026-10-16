//! Inventory and route-local state for normal logical traversal.

use crate::inputs::GenerationInputs;
use common_types::Item;
use logic_catalog::{FollowerMask, ItemLocationIndex, LogicCatalog, OcarinaState, UniqueItem};

/// Permanent inventory and progress, held fixed during a traversal wave.
#[derive(Clone, Debug)]
pub struct GlobalState {
    pub unique_items: [bool; UniqueItem::COUNT],
    /// Acquired levels: Sword 0..=4, Shield 0..=3, Glove 0..=2,
    /// Mail 0..=2 (zero is Green Mail), and Boomerang 0..=2.
    pub sword_level: u8,
    pub shield_level: u8,
    pub glove_level: u8,
    pub mail_level: u8,
    pub boomerang_level: u8,
    pub ocarina: OcarinaState,

    /// Health in eighths of a heart.
    pub health_capacity: u8,
    /// Magic in normalized catalog units.
    pub magic_capacity: u8,
    pub bomb_capacity: u8,
    pub arrow_capacity: u8,
    /// Pieces toward the next heart container, 0..=3.
    pub heart_pieces: u8,
    pub bottles_owned: u8,
    pub refills: RefillFlags,

    /// Owned Big Keys, indexed by catalog DungeonIndex.
    pub big_keys: Vec<bool>,
    /// Permanent door access, indexed by catalog DoorIndex, regardless of mechanism.
    /// Small Key grants resolve their stable DoorId and set the same bit as OpenDoor.
    pub unlocked_doors: Vec<bool>,
    /// Permanent progress flags, indexed by catalog FlagIndex.
    pub flags: Vec<bool>,
}

impl GlobalState {
    /// Apply a placed reward. The caller is responsible for collecting each location once.
    /// The pickup location determines Big Key scope; Small Keys carry their door identity.
    /// One-time resource refills and initial bottle contents have no logical effect.
    pub fn grant_item(
        &mut self,
        catalog: &LogicCatalog,
        inputs: &GenerationInputs,
        item: Item,
        location_idx: ItemLocationIndex,
    ) {
        match item {
            Item::Sword => self.sword_level = (self.sword_level + 1).min(4),
            Item::Shield => self.shield_level = (self.shield_level + 1).min(3),
            Item::Glove => self.glove_level = (self.glove_level + 1).min(2),
            Item::Mail => self.mail_level = (self.mail_level + 1).min(2),
            Item::Boomerang => self.boomerang_level = (self.boomerang_level + 1).min(2),
            Item::Ocarina => self.ocarina = self.ocarina.max(OcarinaState::Inactive),
            Item::BossHeartContainer | Item::SanctuaryHeartContainer => {
                self.health_capacity = (self.health_capacity + 8).min(160);
            }
            Item::PieceOfHeart => {
                self.heart_pieces += 1;
                if self.heart_pieces == 4 {
                    self.heart_pieces = 0;
                    self.health_capacity = (self.health_capacity + 8).min(160);
                }
            }
            Item::BombUpgrade5 => self.bomb_capacity = (self.bomb_capacity + 5).min(50),
            Item::BombUpgrade10 => self.bomb_capacity = (self.bomb_capacity + 10).min(50),
            Item::BombUpgrade50 => self.bomb_capacity = 50,
            Item::ArrowUpgrade5 => self.arrow_capacity = (self.arrow_capacity + 5).min(70),
            Item::ArrowUpgrade10 => self.arrow_capacity = (self.arrow_capacity + 10).min(70),
            Item::ArrowUpgrade70 => self.arrow_capacity = 70,
            // These receipts all create a bottle, including the filled-bottle rewards.
            Item::Bottle
            | Item::Bee
            | Item::Fairy
            | Item::GoldBee
            | Item::RedPotion
            | Item::GreenPotion
            | Item::BluePotion => self.bottles_owned = (self.bottles_owned + 1).min(4),
            Item::SmallKey { door_id } => {
                let door_idx = inputs
                    .door_ids
                    .iter()
                    .position(|&id| id == door_id)
                    .unwrap();
                self.unlocked_doors[door_idx] = true;
            }
            Item::BigKey => {
                let location = catalog.item_location(location_idx);
                let room = catalog.room(catalog.vertex(location.vertex_idx).room_idx);
                self.big_keys[room.dungeon_idx.unwrap().0 as usize] = true;
            }
            Item::FireRod => self.unique_items[UniqueItem::FireRod as usize] = true,
            Item::IceRod => self.unique_items[UniqueItem::IceRod as usize] = true,
            Item::Hammer => self.unique_items[UniqueItem::Hammer as usize] = true,
            Item::Hookshot => self.unique_items[UniqueItem::Hookshot as usize] = true,
            Item::Bow => self.unique_items[UniqueItem::Bow as usize] = true,
            Item::SilverArrows => self.unique_items[UniqueItem::SilverArrows as usize] = true,
            Item::MagicPowder => self.unique_items[UniqueItem::MagicPowder as usize] = true,
            Item::Bombos => self.unique_items[UniqueItem::Bombos as usize] = true,
            Item::Ether => self.unique_items[UniqueItem::Ether as usize] = true,
            Item::Quake => self.unique_items[UniqueItem::Quake as usize] = true,
            Item::Lamp => self.unique_items[UniqueItem::Lamp as usize] = true,
            Item::Shovel => self.unique_items[UniqueItem::Shovel as usize] = true,
            Item::CaneOfSomaria => self.unique_items[UniqueItem::CaneOfSomaria as usize] = true,
            Item::CaneOfByrna => self.unique_items[UniqueItem::CaneOfByrna as usize] = true,
            Item::Cape => self.unique_items[UniqueItem::Cape as usize] = true,
            Item::MagicMirror => self.unique_items[UniqueItem::MagicMirror as usize] = true,
            Item::BookOfMudora => self.unique_items[UniqueItem::BookOfMudora as usize] = true,
            Item::Flippers => self.unique_items[UniqueItem::Flippers as usize] = true,
            Item::MoonPearl => self.unique_items[UniqueItem::MoonPearl as usize] = true,
            Item::BugCatchingNet => self.unique_items[UniqueItem::BugCatchingNet as usize] = true,
            Item::Mushroom => self.unique_items[UniqueItem::Mushroom as usize] = true,
            Item::PegasusBoots => self.unique_items[UniqueItem::PegasusBoots as usize] = true,
            Item::HalfMagic => self.unique_items[UniqueItem::HalfMagic as usize] = true,
            Item::PendantOfCourage => {
                self.unique_items[UniqueItem::PendantOfCourage as usize] = true
            }
            Item::PendantOfWisdom => self.unique_items[UniqueItem::PendantOfWisdom as usize] = true,
            Item::PendantOfPower => self.unique_items[UniqueItem::PendantOfPower as usize] = true,
            Item::Crystal1 => self.unique_items[UniqueItem::Crystal1 as usize] = true,
            Item::Crystal2 => self.unique_items[UniqueItem::Crystal2 as usize] = true,
            Item::Crystal3 => self.unique_items[UniqueItem::Crystal3 as usize] = true,
            Item::Crystal4 => self.unique_items[UniqueItem::Crystal4 as usize] = true,
            Item::Crystal5 => self.unique_items[UniqueItem::Crystal5 as usize] = true,
            Item::Crystal6 => self.unique_items[UniqueItem::Crystal6 as usize] = true,
            Item::Crystal7 => self.unique_items[UniqueItem::Crystal7 as usize] = true,
            Item::Triforce => self.unique_items[UniqueItem::Triforce as usize] = true,
            Item::Arrow
            | Item::TenArrows
            | Item::Bomb
            | Item::ThreeBombs
            | Item::TenBombs
            | Item::Heart
            | Item::SmallMagic
            | Item::OneRupee
            | Item::FiveRupees
            | Item::TwentyRupees
            | Item::FiftyRupees
            | Item::OneHundredRupees
            | Item::ThreeHundredRupees
            | Item::Map
            | Item::Compass => {}
        }
    }
}

/// Reusable sources discovered globally, enabling preparation in subsequent waves.
/// Bottle sources never replenish the current route's bottle-use budget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RefillFlags {
    pub health: bool,
    pub magic: bool,
    pub bombs: bool,
    pub arrows: bool,
    pub red_potion: bool,
    pub green_potion: bool,
    pub blue_potion: bool,
    pub fairy: bool,
}

/// Route-local resource usage and temporary state, including alternative followers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalState {
    /// Net health depletion in eighths of a heart; full health has zero used.
    pub health_used: u8,
    pub magic_used: ResourceUsage,
    pub bombs_used: ResourceUsage,
    pub arrows_used: ResourceUsage,
    /// Each use may choose any globally known content. Never reduced along a route.
    pub bottles_used: u8,
    /// Bit (follower as u8) represents a possible catalog Follower.
    /// No follower is always available by freely discarding any follower; zero is valid.
    /// A follower requirement needs its bit present, then narrows the mask to that bit.
    pub possible_followers: FollowerMask,
    /// Bit n represents ObstacleIndex(n) in the containing vertex's room.
    /// Room resets set this mask to zero.
    pub cleared_obstacles: u8,
}

/// Net resource depletion, with u8::MAX reserved for an initially-empty resource.
/// Ordinary values increase with spending and decrease with refills.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceUsage(pub u8);

impl ResourceUsage {
    /// No initial preparation: availability stays zero when capacity increases.
    /// A refill replaces this with finite usage relative to the current capacity.
    pub const UNPREPARED: Self = Self(u8::MAX);
}
