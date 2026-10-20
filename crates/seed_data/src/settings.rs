use common_types::{Item, SourceRoomId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub proficiencies: Vec<ProficiencySetting>,
    pub tech: Vec<TechSetting>,
    pub notables: Vec<NotableSetting>,
    pub item_pool: Vec<ItemPoolEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProficiencySetting {
    /// Stable ID from tech.json.
    pub tech_id: u32,
    /// Informational; does not determine identity or affect logic.
    pub name: String,
    pub level: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TechSetting {
    /// Stable ID from z3-json-data tech.json.
    pub tech_id: u32,
    /// Informational; does not determine identity or affect logic.
    pub name: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotableSetting {
    /// Stable room identity; notable IDs and names are scoped to this room.
    pub room_id: SourceRoomId,
    pub notable_id: u32,
    /// Informational; does not determine identity or affect logic.
    pub name: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemPoolEntry {
    /// Ordinary reward to place; dungeon items and prizes use separate placement phases.
    /// Flatten the tagged reward so JSON remains {"name": "Sword", "count": 3}.
    #[serde(flatten)]
    pub item: Item,
    /// Number of pickups to place.
    pub count: u32,
}
