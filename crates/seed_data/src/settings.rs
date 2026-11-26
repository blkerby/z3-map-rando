use common_types::{ItemKind, SourceRoomId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Generation settings: complete concrete values, in the shape of a settings document
/// (see `docs/data-formats/settings.md`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub map_layout: MapLayoutSetting,
    pub skill: SkillSettings,
    pub item_pool: ItemPoolSettings,
    pub starting_state: StartingState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemPoolSettings {
    pub deduct_starting_items: bool,
    pub bomb_upgrade_size: u8,
    pub arrow_upgrade_size: u8,
    /// Pickup counts before deducting starting equipment. Dungeon items and prizes are separate.
    pub items: BTreeMap<ItemKind, u32>,
}

/// Concrete inventory, resources, and progress when a new game begins.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartingState {
    pub equipment: StartingEquipment,
    pub resources: StartingResources,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartingEquipment {
    pub fire_rod: bool,
    pub ice_rod: bool,
    pub hammer: bool,
    pub hookshot: bool,
    pub magic_powder: bool,
    pub bombos: bool,
    pub ether: bool,
    pub quake: bool,
    pub lamp: bool,
    pub shovel: bool,
    pub cane_of_somaria: bool,
    pub cane_of_byrna: bool,
    pub cape: bool,
    pub magic_mirror: bool,
    pub book_of_mudora: bool,
    pub flippers: bool,
    pub moon_pearl: bool,
    pub bug_catching_net: bool,
    pub mushroom: bool,
    pub pegasus_boots: bool,
    pub half_magic: bool,
    pub sword: StartingSword,
    pub shield: StartingShield,
    pub glove: StartingGlove,
    pub mail: StartingMail,
    pub bow: StartingBow,
    pub boomerang: StartingBoomerang,
    pub ocarina: StartingOcarina,
}

impl StartingEquipment {
    /// Pickup equivalents for pool deduction and logical initialization.
    pub fn count_items(&self) -> BTreeMap<ItemKind, u32> {
        let mut counts = BTreeMap::new();
        for (item, owned) in [
            (ItemKind::FireRod, self.fire_rod),
            (ItemKind::IceRod, self.ice_rod),
            (ItemKind::Hammer, self.hammer),
            (ItemKind::Hookshot, self.hookshot),
            (ItemKind::MagicPowder, self.magic_powder),
            (ItemKind::Bombos, self.bombos),
            (ItemKind::Ether, self.ether),
            (ItemKind::Quake, self.quake),
            (ItemKind::Lamp, self.lamp),
            (ItemKind::Shovel, self.shovel),
            (ItemKind::CaneOfSomaria, self.cane_of_somaria),
            (ItemKind::CaneOfByrna, self.cane_of_byrna),
            (ItemKind::Cape, self.cape),
            (ItemKind::MagicMirror, self.magic_mirror),
            (ItemKind::BookOfMudora, self.book_of_mudora),
            (ItemKind::Flippers, self.flippers),
            (ItemKind::MoonPearl, self.moon_pearl),
            (ItemKind::BugCatchingNet, self.bug_catching_net),
            (ItemKind::Mushroom, self.mushroom),
            (ItemKind::PegasusBoots, self.pegasus_boots),
            (ItemKind::HalfMagic, self.half_magic),
        ] {
            counts.insert(item, if owned { 1 } else { 0 });
        }
        counts.insert(
            ItemKind::Sword,
            match self.sword {
                StartingSword::None => 0,
                StartingSword::Fighter => 1,
                StartingSword::Master => 2,
                StartingSword::Tempered => 3,
                StartingSword::Golden => 4,
            },
        );
        counts.insert(
            ItemKind::Shield,
            match self.shield {
                StartingShield::None => 0,
                StartingShield::Fighter => 1,
                StartingShield::Fire => 2,
                StartingShield::Mirror => 3,
            },
        );
        counts.insert(
            ItemKind::Glove,
            match self.glove {
                StartingGlove::None => 0,
                StartingGlove::Power => 1,
                StartingGlove::Titans => 2,
            },
        );
        counts.insert(
            ItemKind::Mail,
            match self.mail {
                StartingMail::Green => 0,
                StartingMail::Blue => 1,
                StartingMail::Red => 2,
            },
        );
        counts.insert(
            ItemKind::Bow,
            match self.bow {
                StartingBow::None => 0,
                StartingBow::Bow => 1,
                StartingBow::Silver => 2,
            },
        );
        counts.insert(
            ItemKind::Boomerang,
            match self.boomerang {
                StartingBoomerang::None => 0,
                StartingBoomerang::Blue => 1,
                StartingBoomerang::Red => 2,
            },
        );
        counts.insert(
            ItemKind::Ocarina,
            match self.ocarina {
                StartingOcarina::None => 0,
                StartingOcarina::Inactive => 1,
                StartingOcarina::Active => 1,
            },
        );
        counts
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartingSword {
    None,
    Fighter,
    Master,
    Tempered,
    Golden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartingShield {
    None,
    Fighter,
    Fire,
    Mirror,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartingGlove {
    None,
    Power,
    Titans,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartingMail {
    Green,
    Blue,
    Red,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartingBow {
    None,
    Bow,
    Silver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartingBoomerang {
    None,
    Blue,
    Red,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartingOcarina {
    None,
    Inactive,
    Active,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartingResources {
    /// Capacity in whole hearts; the new game begins at full health.
    pub heart_capacity: u8,
    pub bomb_capacity: u8,
    pub arrow_capacity: u8,
    pub bombs: u8,
    pub arrows: u8,
    pub rupees: u16,
    pub bottle_1: StartingBottle,
    pub bottle_2: StartingBottle,
    pub bottle_3: StartingBottle,
    pub bottle_4: StartingBottle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartingBottle {
    None,
    Empty,
    RedPotion,
    GreenPotion,
    BluePotion,
    Fairy,
    Bee,
    GoldBee,
}

#[derive(Debug)]
pub struct InsufficientPoolItems {
    pub item: ItemKind,
    pub available: u32,
    pub starting: u32,
}

impl std::fmt::Display for InsufficientPoolItems {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "item pool contains {} {:?}, but starting equipment requires {}",
            self.available, self.item, self.starting
        )
    }
}

impl std::error::Error for InsufficientPoolItems {}

impl Settings {
    /// Items remaining for placement; starting resources and flags do not deduct pickups.
    pub fn resolve_item_pool(&self) -> Result<BTreeMap<ItemKind, u32>, InsufficientPoolItems> {
        let mut items = self.item_pool.items.clone();
        if self.item_pool.deduct_starting_items {
            for (item, starting) in self.starting_state.equipment.count_items() {
                let available = items.entry(item).or_default();
                if starting > *available {
                    return Err(InsufficientPoolItems {
                        item,
                        available: *available,
                        starting,
                    });
                }
                *available -= starting;
            }
        }
        Ok(items)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MapLayoutSetting {
    Vanilla,
    Random,
}

/// Skill assumptions, keyed by stable z3-json-data identities.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillSettings {
    /// Proficiency level by tech ID.
    pub proficiencies: BTreeMap<u32, u32>,
    /// Enabled flag by tech ID.
    pub tech: BTreeMap<u32, bool>,
    /// Enabled flags of notables, keyed by room ID and then by room-scoped notable ID.
    pub notables: BTreeMap<SourceRoomId, BTreeMap<u32, bool>>,
}

/// The player's cosmetic choices, as described by `schema/settings/customization.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomizationSettings {
    /// HUD heart color as hexadecimal `rrggbb`.
    pub heart_color: String,
    pub reduced_flashing: bool,
}
