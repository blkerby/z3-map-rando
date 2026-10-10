use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub proficiencies: Vec<ProficiencySetting>,
    pub tech: Vec<TechSetting>,
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
    /// Stable ID from sm-json-data tech.json.
    pub tech_id: u32,
    /// Informational; does not determine identity or affect logic.
    pub name: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemPoolEntry {
    /// Item identity from items.json.
    pub name: String,
    /// Number of pickups to place.
    pub count: u32,
}
