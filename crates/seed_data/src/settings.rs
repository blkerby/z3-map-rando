use common_types::Item;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Generation settings: complete concrete values, in the shape of a settings document
/// (see `docs/data-formats/settings.md`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub map_layout: MapLayoutSetting,
    pub skill: SkillSettings,
    /// Number of pickups to place per ordinary item. Dungeon items and prizes use separate
    /// placement phases.
    pub item_pool: BTreeMap<Item, u32>,
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
    pub notables: NotableSettings,
}

/// Enabled flags of notables, keyed by room ID and then by room-scoped notable ID.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotableSettings {
    pub overworld: BTreeMap<u32, BTreeMap<u32, bool>>,
    pub underworld: BTreeMap<u32, BTreeMap<u32, bool>>,
}
