//! Inventory and route-local state for normal logical traversal.

use crate::inputs::GenerationInputs;
use anyhow::{Result, bail};
use common_types::Item;
use logic_catalog::{
    DoorIndex, DungeonIndex, FlagIndex, FollowerMask, ItemLocationIndex, LogicCatalog,
    OcarinaState, UniqueItem,
};

/// Mutable global reads that can change an edge's results or discoveries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GlobalDependency {
    Item(usize),
    Sword,
    Shield,
    Glove,
    Mail,
    Boomerang,
    Ocarina,
    BigKey(DungeonIndex),
    Door(DoorIndex),
    Flag(FlagIndex),
    ItemLocation(ItemLocationIndex),
    Health,
    Magic,
    Bombs,
    Arrows,
    Bottles,
    RedPotion,
    GreenPotion,
    BluePotion,
    Fairy,
}

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
    /// Derived from physical capacity and the independently obtained refill flags.
    pub magic_usable_capacity: u8,
    pub bomb_usable_capacity: u8,
    pub arrow_usable_capacity: u8,
    /// Pieces toward the next heart container, 0..=3.
    pub heart_pieces: u8,
    pub bottles_owned: u8,
    pub refills: RefillFlags,

    /// Owned Big Keys, indexed by catalog DungeonIndex.
    pub big_keys: Vec<bool>,
    /// Permanent door access, indexed by catalog DoorIndex, regardless of mechanism.
    /// Collecting a Small Key resolves its stable DoorId and sets the same bit as OpenDoor.
    pub unlocked_doors: Vec<bool>,
    /// Permanent progress flags, indexed by catalog FlagIndex.
    pub flags: Vec<bool>,
    /// Locations with an executable collection action, indexed by ItemLocationIndex.
    /// The normal traversal's closure loop records discoveries between waves,
    /// including unfilled locations.
    pub reachable_item_locations: Vec<bool>,
    /// Locations whose placed items have been applied, indexed by ItemLocationIndex.
    /// Reachable unfilled locations remain uncollected.
    pub collected_item_locations: Vec<bool>,
}

impl GlobalState {
    /// Begin with vanilla capacities, no equipment, and Zelda already rescued.
    pub fn initialize(catalog: &LogicCatalog) -> Self {
        let mut flags = Vec::with_capacity(catalog.flags.len());
        for flag in &catalog.flags {
            flags.push(flag == "GameStateZelda");
        }
        Self {
            unique_items: [false; UniqueItem::COUNT],
            sword_level: 0,
            shield_level: 0,
            glove_level: 0,
            mail_level: 0,
            boomerang_level: 0,
            ocarina: OcarinaState::Missing,
            health_capacity: 24,
            magic_capacity: 128,
            bomb_capacity: 10,
            arrow_capacity: 30,
            magic_usable_capacity: 0,
            bomb_usable_capacity: 0,
            arrow_usable_capacity: 0,
            heart_pieces: 0,
            bottles_owned: 0,
            refills: RefillFlags::default(),
            big_keys: vec![false; catalog.dungeons.len()],
            unlocked_doors: vec![false; catalog.doors.len()],
            flags,
            reachable_item_locations: vec![false; catalog.item_locations.len()],
            collected_item_locations: vec![false; catalog.item_locations.len()],
        }
    }

    /// Collect a placed item into inventory, returning an error for already-collected locations.
    /// The pickup location determines Big Key scope; Small Keys carry their door identity.
    /// Return the changed scheduling key, excluding duplicate or capped capabilities.
    /// One-time resource refills and initial bottle contents have no logical effect.
    pub(crate) fn collect_item(
        &mut self,
        catalog: &LogicCatalog,
        inputs: &GenerationInputs,
        item: Item,
        location_idx: ItemLocationIndex,
    ) -> Result<Option<GlobalDependency>> {
        use GlobalDependency as D;
        let collected = &mut self.collected_item_locations[location_idx.0 as usize];
        if *collected {
            bail!("item location {location_idx:?} has already been collected");
        }
        *collected = true;
        Ok(match item {
            Item::SmallKey { door_id } => {
                let door_idx = inputs.door_indices[&door_id];
                let unlocked = &mut self.unlocked_doors[door_idx.0 as usize];
                if !*unlocked {
                    *unlocked = true;
                    Some(D::Door(door_idx))
                } else {
                    None
                }
            }
            Item::BigKey => {
                let location = catalog.item_location(location_idx);
                let room = catalog.room(catalog.vertex(location.vertex_idx).room_idx);
                let dungeon_idx = room.dungeon_idx.unwrap();
                let owned = &mut self.big_keys[dungeon_idx.0 as usize];
                if !*owned {
                    *owned = true;
                    Some(D::BigKey(dungeon_idx))
                } else {
                    None
                }
            }
            _ => self.grant_non_key_item(item),
        })
    }

    /// Grant an assumed non-key inventory item without collecting a location.
    pub(crate) fn grant_non_key_item(&mut self, item: Item) -> Option<GlobalDependency> {
        use GlobalDependency as D;
        let previous_bomb_usable_capacity = self.bomb_usable_capacity;
        let previous_arrow_usable_capacity = self.arrow_usable_capacity;
        let dependency = match item {
            Item::Sword if self.sword_level < 4 => {
                self.sword_level += 1;
                Some(D::Sword)
            }
            Item::Shield if self.shield_level < 3 => {
                self.shield_level += 1;
                Some(D::Shield)
            }
            Item::Glove if self.glove_level < 2 => {
                self.glove_level += 1;
                Some(D::Glove)
            }
            Item::Mail if self.mail_level < 2 => {
                self.mail_level += 1;
                Some(D::Mail)
            }
            Item::Boomerang if self.boomerang_level < 2 => {
                self.boomerang_level += 1;
                Some(D::Boomerang)
            }
            Item::Ocarina if self.ocarina == OcarinaState::Missing => {
                self.ocarina = OcarinaState::Inactive;
                Some(D::Ocarina)
            }
            Item::BossHeartContainer | Item::SanctuaryHeartContainer
                if self.health_capacity < 160 =>
            {
                self.health_capacity = (self.health_capacity + 8).min(160);
                Some(D::Health)
            }
            Item::PieceOfHeart => {
                self.heart_pieces += 1;
                if self.heart_pieces == 4 {
                    self.heart_pieces = 0;
                    if self.health_capacity < 160 {
                        self.health_capacity = (self.health_capacity + 8).min(160);
                        Some(D::Health)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Item::BombUpgrade5 => {
                self.bomb_capacity = (self.bomb_capacity + 5).min(50);
                Some(D::Bombs)
            }
            Item::BombUpgrade10 => {
                self.bomb_capacity = (self.bomb_capacity + 10).min(50);
                Some(D::Bombs)
            }
            Item::BombUpgrade50 => {
                self.bomb_capacity = 50;
                Some(D::Bombs)
            }
            Item::ArrowUpgrade5 => {
                self.arrow_capacity = (self.arrow_capacity + 5).min(70);
                Some(D::Arrows)
            }
            Item::ArrowUpgrade10 => {
                self.arrow_capacity = (self.arrow_capacity + 10).min(70);
                Some(D::Arrows)
            }
            Item::ArrowUpgrade70 => {
                self.arrow_capacity = 70;
                Some(D::Arrows)
            }
            // These receipts all create a bottle, including the filled-bottle items.
            Item::Bottle
            | Item::Bee
            | Item::Fairy
            | Item::GoldBee
            | Item::RedPotion
            | Item::GreenPotion
            | Item::BluePotion => {
                if self.bottles_owned < 4 {
                    self.bottles_owned += 1;
                    Some(D::Bottles)
                } else {
                    None
                }
            }
            Item::FireRod => Some(D::Item(UniqueItem::FireRod as usize)),
            Item::IceRod => Some(D::Item(UniqueItem::IceRod as usize)),
            Item::Hammer => Some(D::Item(UniqueItem::Hammer as usize)),
            Item::Hookshot => Some(D::Item(UniqueItem::Hookshot as usize)),
            Item::Bow => Some(D::Item(UniqueItem::Bow as usize)),
            Item::SilverArrows => Some(D::Item(UniqueItem::SilverArrows as usize)),
            Item::MagicPowder => Some(D::Item(UniqueItem::MagicPowder as usize)),
            Item::Bombos => Some(D::Item(UniqueItem::Bombos as usize)),
            Item::Ether => Some(D::Item(UniqueItem::Ether as usize)),
            Item::Quake => Some(D::Item(UniqueItem::Quake as usize)),
            Item::Lamp => Some(D::Item(UniqueItem::Lamp as usize)),
            Item::Shovel => Some(D::Item(UniqueItem::Shovel as usize)),
            Item::CaneOfSomaria => Some(D::Item(UniqueItem::CaneOfSomaria as usize)),
            Item::CaneOfByrna => Some(D::Item(UniqueItem::CaneOfByrna as usize)),
            Item::Cape => Some(D::Item(UniqueItem::Cape as usize)),
            Item::MagicMirror => Some(D::Item(UniqueItem::MagicMirror as usize)),
            Item::BookOfMudora => Some(D::Item(UniqueItem::BookOfMudora as usize)),
            Item::Flippers => Some(D::Item(UniqueItem::Flippers as usize)),
            Item::MoonPearl => Some(D::Item(UniqueItem::MoonPearl as usize)),
            Item::BugCatchingNet => Some(D::Item(UniqueItem::BugCatchingNet as usize)),
            Item::Mushroom => Some(D::Item(UniqueItem::Mushroom as usize)),
            Item::PegasusBoots => Some(D::Item(UniqueItem::PegasusBoots as usize)),
            Item::HalfMagic => Some(D::Item(UniqueItem::HalfMagic as usize)),
            Item::PendantOfCourage => Some(D::Item(UniqueItem::PendantOfCourage as usize)),
            Item::PendantOfWisdom => Some(D::Item(UniqueItem::PendantOfWisdom as usize)),
            Item::PendantOfPower => Some(D::Item(UniqueItem::PendantOfPower as usize)),
            Item::Crystal1 => Some(D::Item(UniqueItem::Crystal1 as usize)),
            Item::Crystal2 => Some(D::Item(UniqueItem::Crystal2 as usize)),
            Item::Crystal3 => Some(D::Item(UniqueItem::Crystal3 as usize)),
            Item::Crystal4 => Some(D::Item(UniqueItem::Crystal4 as usize)),
            Item::Crystal5 => Some(D::Item(UniqueItem::Crystal5 as usize)),
            Item::Crystal6 => Some(D::Item(UniqueItem::Crystal6 as usize)),
            Item::Crystal7 => Some(D::Item(UniqueItem::Crystal7 as usize)),
            Item::Triforce => Some(D::Item(UniqueItem::Triforce as usize)),
            Item::SmallKey { .. }
            | Item::BigKey
            | Item::Sword
            | Item::Shield
            | Item::Glove
            | Item::Mail
            | Item::Boomerang
            | Item::Ocarina
            | Item::BossHeartContainer
            | Item::SanctuaryHeartContainer
            | Item::Arrow
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
            | Item::Compass => None,
        };
        self.update_usable_capacities();
        match dependency {
            Some(D::Item(idx)) => {
                if !self.unique_items[idx] {
                    self.unique_items[idx] = true;
                    dependency
                } else {
                    None
                }
            }
            Some(D::Bombs) if self.bomb_usable_capacity == previous_bomb_usable_capacity => None,
            Some(D::Arrows) if self.arrow_usable_capacity == previous_arrow_usable_capacity => None,
            _ => dependency,
        }
    }

    /// Obtaining a source remains meaningful even when the physical capacity is zero.
    pub fn update_usable_capacities(&mut self) {
        self.magic_usable_capacity = if self.refills.magic {
            self.magic_capacity
        } else {
            0
        };
        self.bomb_usable_capacity = if self.refills.bombs {
            self.bomb_capacity
        } else {
            0
        };
        self.arrow_usable_capacity = if self.refills.arrows {
            self.arrow_capacity
        } else {
            0
        };
    }
}

/// Obtained preparation sources, independent of current resource capacities.
/// Bottle sources never replenish the current route's bottle-use budget.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RefillFlags {
    pub magic: bool,
    pub bombs: bool,
    pub arrows: bool,
    pub red_potion: bool,
    pub green_potion: bool,
    pub blue_potion: bool,
    pub fairy: bool,
}

impl RefillFlags {
    /// Retain sources obtained in either set, including those with zero usable capacity.
    pub fn merge(&mut self, other: Self) {
        self.magic |= other.magic;
        self.bombs |= other.bombs;
        self.arrows |= other.arrows;
        self.red_potion |= other.red_potion;
        self.green_potion |= other.green_potion;
        self.blue_potion |= other.blue_potion;
        self.fairy |= other.fairy;
    }
}

/// Route-local resource usage and temporary state, including alternative followers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalState {
    pub health: HealthState,
    pub magic_used: u8,
    pub bombs_used: u8,
    pub arrows_used: u8,
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

impl LocalState {
    /// Begin a route with full health, no consumption, followers, or cleared obstacles.
    pub(crate) fn initialize() -> Self {
        Self {
            health: HealthState::Consumed(0),
            magic_used: 0,
            bombs_used: 0,
            arrows_used: 0,
            bottles_used: 0,
            possible_followers: FollowerMask(0),
            cleared_obstacles: 0,
        }
    }
}

/// Health in eighths of a heart, preserving the fixed amount from a Fairy revival.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealthState {
    Consumed(u8),
    Remaining(u8),
}
