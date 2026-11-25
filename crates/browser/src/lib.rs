use common_types::Item;
use logic_catalog::{ItemLocationIndex, LogicCatalog};
use seed_data::SeedData;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Patch a vanilla ROM with a saved seed (JSON), customization settings (JSON), and the
/// catalogs matching this build.
#[wasm_bindgen]
pub fn patch_rom(
    rom: &[u8],
    seed_json: &[u8],
    customization_json: &[u8],
    retiling_catalog: &[u8],
    patch_catalog: &[u8],
) -> Result<Vec<u8>, JsError> {
    let patch = || -> anyhow::Result<Vec<u8>> {
        let seed = SeedData::read(seed_json)?;
        let settings = patcher::PatchSettings {
            customization: serde_json::from_slice(customization_json)?,
            transition_asset_phase: Default::default(),
        };
        let retiling = retiling_catalog::decode_catalog(retiling_catalog)?;
        let patches = patch_catalog::decode_catalog(patch_catalog)?;
        patcher::patch_rom(rom, &seed, &retiling, &patches, &settings)
    };
    patch().map_err(|error| JsError::new(&format!("{error:#}")))
}

#[derive(Serialize)]
struct SpoilerOutput {
    spheres: Vec<Vec<SpoilerEntry>>,
    unreachable: Vec<SpoilerEntry>,
}

#[derive(Serialize)]
struct SpoilerEntry {
    location: String,
    /// Item kind token, or None for a location without a saved item.
    item: Option<String>,
    /// For a Small Key, the name of the door it opens.
    door: Option<String>,
}

/// Replay a saved seed (JSON) against the logic catalog matching this build, returning its
/// spheres and unreachable locations as JSON, with display names from the catalog.
#[wasm_bindgen]
pub fn replay_spoiler(seed_json: &[u8], logic_catalog: &[u8]) -> Result<String, JsError> {
    let replay = || -> anyhow::Result<String> {
        let seed = SeedData::read(seed_json)?;
        let catalog = logic_catalog::decode_catalog(logic_catalog)?;
        let result = generator::spoiler::replay_spheres(&catalog, &seed)?;
        let mut spheres = Vec::new();
        for sphere in &result.spheres {
            let mut entries = Vec::new();
            for &location_idx in sphere {
                entries.push(create_spoiler_entry(&catalog, &result.items, location_idx)?);
            }
            spheres.push(entries);
        }
        let mut unreachable = Vec::new();
        for &location_idx in &result.unreachable {
            unreachable.push(create_spoiler_entry(&catalog, &result.items, location_idx)?);
        }
        Ok(serde_json::to_string(&SpoilerOutput {
            spheres,
            unreachable,
        })?)
    };
    replay().map_err(|error| JsError::new(&format!("{error:#}")))
}

fn create_spoiler_entry(
    catalog: &LogicCatalog,
    items: &[Option<Item>],
    location_idx: ItemLocationIndex,
) -> anyhow::Result<SpoilerEntry> {
    let item = items[location_idx.0 as usize];
    let mut item_name = None;
    let mut door = None;
    if let Some(item) = item {
        // An item kind serializes as its token string.
        item_name = Some(
            serde_json::to_value(item.get_kind())?
                .as_str()
                .unwrap()
                .to_owned(),
        );
        if let Item::SmallKey(door_id) = item {
            for catalog_door in &catalog.doors {
                let room = &catalog.rooms[catalog_door.room_idx.0 as usize];
                if room.room_id == Some(door_id.room_id) && catalog_door.door_id == door_id.door_id
                {
                    door = Some(format!("{} - {}", room.name, catalog_door.name));
                }
            }
        }
    }
    Ok(SpoilerEntry {
        location: catalog.item_locations[location_idx.0 as usize].name.clone(),
        item: item_name,
        door,
    })
}
