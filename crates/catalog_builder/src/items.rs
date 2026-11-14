use anyhow::{Context, Result};
use common_types::{Item, ItemLocationId, SourceRoomId};
use serde::Deserialize;
use std::{fs, path::Path};

/// Source item entries without a randomizer location. The second items of the
/// Waterfall of Wishing and Pyramid fairies are replaced by one upgrade per fairy.
pub const RETIRED_ITEM_LOCATIONS: [ItemLocationId; 2] = [
    ItemLocationId {
        room_id: SourceRoomId::Underworld(12),
        item_id: 2,
    },
    ItemLocationId {
        room_id: SourceRoomId::Underworld(69),
        item_id: 2,
    },
];

/// A record of the randomizer's item definitions (`items` in catalog-build.toml).
#[derive(Deserialize)]
pub struct ItemRecord {
    /// The `Item` token, e.g. `fire_rod`.
    pub name: String,
    /// The z3-json-data item name of its receipt.
    pub upstream_name: String,
    /// Further z3-json-data item names that vanilla locations use for it.
    pub other_names: Vec<String>,
}

pub fn read_item_records(path: &Path) -> Result<Vec<ItemRecord>> {
    serde_json::from_slice(&fs::read(path)?).with_context(|| format!("reading {}", path.display()))
}

impl ItemRecord {
    /// The item this record defines, or None for Small Keys, whose items carry the door they open.
    pub fn item(&self) -> Result<Option<Item>> {
        if self.name == "small_key" {
            return Ok(None);
        }
        let item = serde_json::from_value(serde_json::Value::String(self.name.clone()))
            .with_context(|| format!("unknown item {}", self.name))?;
        Ok(Some(item))
    }
}
