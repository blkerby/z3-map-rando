//! Uniform prototype generation without logical placement.

pub mod cost_metrics;
pub mod graph;
pub mod inputs;
pub mod interpreter;
pub mod key_assignment;
pub mod key_traversal;
pub mod local_state_reducer;
pub mod state;
pub mod traversal;

pub use seed_data::{SeedData, Settings};

use anyhow::{Result, bail};
use common_types::Item;
use inputs::prepare_inputs;
use logic_catalog::{LogicCatalog, RoomIndex};
use rand::{Rng, seq::SliceRandom};
use retiling_catalog::RetilingCatalog;
use seed_data::{AreaLayout, ItemPlacement, SEED_FORMAT_VERSION, SeedMetadata, WorldLayout};
use std::collections::BTreeMap;

/// Shuffle ordinary items globally and maps/compasses within their source dungeon.
/// Keys and fairy exchanges remain vanilla; pendants and crystals shuffle among prize locations.
/// The caller supplies RNG state and metadata;
/// tech and proficiency settings are saved but do not constrain placement yet.
pub fn generate(
    settings: Settings,
    logic_catalog: &LogicCatalog,
    retiling_catalog: &RetilingCatalog,
    theme_name: &str,
    metadata: SeedMetadata,
    rng: &mut impl Rng,
) -> Result<SeedData> {
    let inputs = prepare_inputs(logic_catalog, &settings)?;
    let mut item_placements = Vec::new();
    let mut locations_by_room = BTreeMap::<RoomIndex, Vec<usize>>::new();
    let mut dungeon_items_by_room = BTreeMap::<RoomIndex, Vec<String>>::new();
    let mut prizes = Vec::new();
    for location in &logic_catalog.item_locations {
        let item_name = &logic_catalog.item(location.vanilla_item_idx).name;
        item_placements.push(ItemPlacement {
            item_location_id: location.id,
            item_name: item_name.clone(),
        });
    }
    for &location_idx in &inputs.ordinary_locations {
        let location = logic_catalog.item_location(location_idx);
        let room_idx = logic_catalog.vertex(location.vertex_idx).room_idx;
        locations_by_room
            .entry(room_idx)
            .or_default()
            .push(location_idx.0 as usize);
    }
    for (index, dungeon) in inputs.dungeons.iter().enumerate() {
        let room_idx = logic_catalog.dungeons[index];
        let mut items = Vec::new();
        for item in &dungeon.map_compass_pool {
            let name = match item {
                Item::Map => "Map",
                Item::Compass => "Compass",
                _ => bail!("unexpected item {item:?} in Map/Compass pool for room {room_idx:?}"),
            };
            items.push(name.to_owned());
        }
        dungeon_items_by_room.insert(room_idx, items);
    }
    for &location_idx in &inputs.prize_locations {
        let location = logic_catalog.item_location(location_idx);
        prizes.push(logic_catalog.item(location.vanilla_item_idx).name.clone());
    }

    // Keep the prototype's string output until the typed-placement migration in Step 11.
    let mut receipt_names = BTreeMap::new();
    for (definition, item) in logic_catalog.items.iter().zip(&inputs.items) {
        if let Some(item) = item {
            receipt_names
                .entry(*item)
                .or_insert(definition.name.as_str());
        }
    }
    let mut item_pool = Vec::new();
    for entry in &settings.item_pool {
        let name = match entry.item {
            Item::Sword => "ProgressiveSword",
            Item::Shield => "ProgressiveShield",
            Item::Glove => "ProgressiveGlove",
            Item::Mail => "ProgressiveArmor",
            // The catalog has no progressive boomerang receipt yet; retain the prototype encoding.
            Item::Boomerang => "BlueBoomerang",
            Item::SilverArrows => "SilverArrows",
            Item::Ocarina => "OcarinaInactive",
            item => receipt_names[&item],
        };
        for _ in 0..entry.count {
            item_pool.push(name.to_owned());
        }
    }

    prizes.shuffle(rng);
    for (location_idx, item_name) in inputs.prize_locations.into_iter().zip(prizes) {
        item_placements[location_idx.0 as usize].item_name = item_name;
    }

    let mut ordinary_locations = Vec::new();
    for (room_idx, mut dungeon_locations) in locations_by_room {
        let room_dungeon_items = dungeon_items_by_room.remove(&room_idx).unwrap_or_default();
        dungeon_locations.shuffle(rng);
        for (offset, placement_idx) in dungeon_locations.into_iter().enumerate() {
            if offset < room_dungeon_items.len() {
                item_placements[placement_idx].item_name = room_dungeon_items[offset].clone();
            } else {
                ordinary_locations.push(placement_idx);
            }
        }
    }
    item_pool.shuffle(rng);
    for (placement_idx, item_name) in ordinary_locations.into_iter().zip(item_pool) {
        item_placements[placement_idx].item_name = item_name;
    }

    let mut areas = Vec::new();
    for area in retiling_catalog.areas.values() {
        if let Some(vanilla_map_id) = area.vanilla_map_id {
            if area.themes.contains_key(theme_name) {
                areas.push(AreaLayout {
                    vanilla_map_id,
                    theme_name: theme_name.to_owned(),
                });
            }
        }
    }
    areas.sort_by_key(|area| area.vanilla_map_id);
    Ok(SeedData {
        format_version: SEED_FORMAT_VERSION,
        metadata,
        settings,
        world_layout: WorldLayout { areas },
        item_placements,
    })
}
