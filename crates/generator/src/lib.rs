//! Uniform prototype generation without logical placement.

pub use seed_data::{SeedData, Settings};

use anyhow::{Result, ensure};
use logic_catalog::{LogicCatalog, RoomIndex};
use rand::{Rng, seq::SliceRandom};
use retiling_catalog::RetilingCatalog;
use seed_data::{AreaLayout, ItemPlacement, SEED_FORMAT_VERSION, SeedMetadata, WorldLayout};
use std::collections::BTreeMap;

/// Shuffle ordinary items globally and maps/compasses within their source dungeon.
/// Keys remain vanilla; pendants and crystals shuffle among prize locations.
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
    let mut item_placements = Vec::new();
    let mut locations_by_room = BTreeMap::<RoomIndex, Vec<usize>>::new();
    let mut dungeon_items_by_room = BTreeMap::<RoomIndex, Vec<String>>::new();
    let mut prize_locations = Vec::new();
    let mut prizes = Vec::new();
    let mut ordinary_location_count = 0;
    for location in &logic_catalog.item_locations {
        let item_name = &logic_catalog.item(location.vanilla_item_idx).name;
        let placement_idx = item_placements.len();
        item_placements.push(ItemPlacement {
            item_location_id: location.id,
            item_name: item_name.clone(),
        });
        let room_idx = logic_catalog.vertex(location.vertex_idx).room_idx;
        match item_name.as_str() {
            "SmallKey" | "BigKey" => continue,
            "PendantOfCourage" | "PendantOfWisdom" | "PendantOfPower" | "Crystal1" | "Crystal2"
            | "Crystal3" | "Crystal4" | "Crystal5" | "Crystal6" | "Crystal7" => {
                prize_locations.push(placement_idx);
                prizes.push(item_name.clone());
                continue;
            }
            "Map" | "Compass" => {
                // Items whose placement is constrained to stay within the same dungeon/room
                dungeon_items_by_room
                    .entry(room_idx)
                    .or_default()
                    .push(item_name.clone());
            }
            _ => ordinary_location_count += 1,
        }
        locations_by_room
            .entry(room_idx)
            .or_default()
            .push(placement_idx);
    }

    let mut item_pool = Vec::new();
    for entry in &settings.item_pool {
        for _ in 0..entry.count {
            item_pool.push(entry.name.clone());
        }
    }
    ensure!(
        item_pool.len() == ordinary_location_count,
        "item pool contains {} items, but {} ordinary locations are available",
        item_pool.len(),
        ordinary_location_count,
    );

    prizes.shuffle(rng);
    for (placement_idx, item_name) in prize_locations.into_iter().zip(prizes) {
        item_placements[placement_idx].item_name = item_name;
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
