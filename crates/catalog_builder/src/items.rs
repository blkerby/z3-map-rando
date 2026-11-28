use anyhow::{Context, Result};
use common_types::{DoorId, ItemKind, ItemLocationId, SourceRoomId};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

/// Source item entries without a randomizer location. The second items of the
/// Waterfall of Wishing and Pyramid fairies are replaced by one upgrade per fairy.
pub const RETIRED_ITEM_LOCATIONS: [ItemLocationId; 2] = [
    ItemLocationId {
        room_id: SourceRoomId(52),
        item_id: 2,
    },
    ItemLocationId {
        room_id: SourceRoomId(109),
        item_id: 2,
    },
];

/// A record of the randomizer's item definitions (`items` in catalog-build.toml).
#[derive(Deserialize)]
pub struct ItemRecord {
    /// The item kind, written as its token, e.g. `fire_rod`.
    pub name: ItemKind,
    /// The z3-json-data item name of its receipt.
    pub upstream_name: String,
    /// Further z3-json-data item names that vanilla locations use for it.
    pub other_names: Vec<String>,
}

pub fn read_item_records(path: &Path) -> Result<Vec<ItemRecord>> {
    serde_json::from_slice(&fs::read(path)?).with_context(|| format!("reading {}", path.display()))
}

#[derive(Deserialize)]
pub struct PairedKey {
    door_id: u32,
    pub simple_item_id: u32,
    pub label: String,
}

pub fn read_paired_keys(path: &Path) -> Result<BTreeMap<DoorId, PairedKey>> {
    #[derive(Deserialize)]
    struct Dungeon {
        room_id: SourceRoomId,
        keys: Vec<PairedKey>,
    }

    let dungeons: Vec<Dungeon> = crate::z3_json_data::read_source(path)?;
    let mut doors = BTreeMap::new();
    for dungeon in dungeons {
        for key in dungeon.keys {
            let id = DoorId {
                room_id: dungeon.room_id,
                door_id: key.door_id,
            };
            anyhow::ensure!(
                doors.insert(id, key).is_none(),
                "duplicate paired door {id:?}"
            );
        }
    }
    Ok(doors)
}
