//! Logical generation on vanilla or pool-selected world layouts.

pub mod cost_metrics;
pub mod graph;
pub mod inputs;
pub mod interpreter;
pub mod key_assignment;
pub mod local_state_reducer;
pub mod map;
pub mod metadata;
pub mod placement;
pub mod spoiler;
pub mod state;
pub mod strat_check;
pub mod traversal;

pub use seed_data::{SeedData, Settings};

use anyhow::{Result, bail, ensure};
use common_types::Item;
use graph::{TraversalGraph, build_graph};
use inputs::{GenerationInputs, prepare_inputs};
use key_assignment::{KeyAssignmentExhausted, KeyPlacementState, assign_small_keys};
use logic_catalog::LogicCatalog;
use map::{MapPool, build_connections, build_vanilla_rooms, select_borders};
use placement::{PlacementState, PlacementStep};
use rand::Rng;
use retiling_catalog::palette::{
    PALETTE_ATTEMPTS, PaletteProfiles, allocate_palettes, select_palette_profiles,
};
use seed_data::{
    ItemPlacement, MapLayoutSetting, ResolvedSettings, RoomData, SeedMetadata, WorldLayout,
};
use std::collections::BTreeMap;

const MAX_GENERATION_ATTEMPTS: usize = 1000;
const MAX_CANDIDATE_ATTEMPTS: usize = 100;
const MAX_EDGE_SELECTION_ATTEMPTS: usize = 10;

/// Generate fully collectible placements using player settings and the selected world layout.
/// Each complete attempt assigns fresh Small Keys and prizes before placing items.
/// The caller supplies RNG state and metadata; generation needs no ROM or filesystem.
pub fn generate(
    resolved: ResolvedSettings,
    logic_catalog: &LogicCatalog,
    palette_profiles: &PaletteProfiles,
    theme_name: &str,
    map_pool: Option<&[u8]>,
    metadata: SeedMetadata,
    rng: &mut impl Rng,
) -> Result<SeedData> {
    let ResolvedSettings {
        values: settings,
        document: settings_document,
    } = resolved;
    ensure!(
        common_types::SUPPORTED_THEMES.contains(&theme_name),
        "unsupported theme {theme_name}"
    );
    let inputs = prepare_inputs(logic_catalog, &settings)?;
    let pool = if settings.map_layout == MapLayoutSetting::Random {
        Some(MapPool::decode(
            map_pool.ok_or_else(|| anyhow::anyhow!("random maps require a map pool"))?,
            logic_catalog,
        )?)
    } else {
        None
    };

    let palette_seed = metadata.rng_seed.parse()?;
    // Area themes do not depend on room positions, so the vanilla layout supplies them.
    let vanilla_layout = WorldLayout {
        vanilla: true,
        rooms: build_vanilla_rooms(logic_catalog, theme_name),
        borders: Vec::new(),
    };
    let mut area_themes = Vec::new();
    for (area, theme) in vanilla_layout.build_area_themes() {
        area_themes.push((usize::from(area), theme));
    }
    let profiles = select_palette_profiles(palette_profiles, area_themes)?;

    for attempt in 1..=MAX_GENERATION_ATTEMPTS {
        log::info!("Starting generation attempt {attempt}/{MAX_GENERATION_ATTEMPTS}");
        let mut world_layout = WorldLayout {
            vanilla: pool.is_none(),
            rooms: if let Some(pool) = &pool {
                pool.select_rooms(logic_catalog, theme_name, rng)?
            } else {
                vanilla_layout.rooms.clone()
            },
            borders: Vec::new(),
        };
        let graph = if pool.is_some() {
            let mut accepted = None;
            for edge_attempt in 1..=MAX_EDGE_SELECTION_ATTEMPTS {
                log::info!("Checking edge selection {edge_attempt}/{MAX_EDGE_SELECTION_ATTEMPTS}");
                select_borders(logic_catalog, &mut world_layout, rng)?;
                let graph = build_graph(
                    logic_catalog,
                    &build_connections(logic_catalog, &world_layout),
                );
                if KeyPlacementState::discover_all_items(logic_catalog, &graph, &inputs)? {
                    accepted = Some(graph);
                    break;
                }
            }
            let Some(graph) = accepted else {
                continue;
            };
            graph
        } else {
            build_graph(logic_catalog, &logic_catalog.vanilla_connections)
        };
        if allocate_palettes(
            &profiles,
            &world_layout.build_scrollable_transition_pairs(),
            palette_seed,
        )
        .is_none()
        {
            if pool.is_some() {
                log::info!(
                    "Rejected map: no palette assignment found in {PALETTE_ATTEMPTS} passes"
                );
                continue;
            }
            bail!(
                "no palette assignment found for the vanilla layout in {PALETTE_ATTEMPTS} passes"
            );
        }
        let items = match generate_items(logic_catalog, &graph, &inputs, rng) {
            Ok(Some(items)) => items,
            Ok(None) => continue,
            Err(error) if pool.is_some() && error.is::<KeyAssignmentExhausted>() => {
                log::info!("Rejected map: {error:#}");
                continue;
            }
            Err(error) => return Err(error),
        };
        let mut rooms = BTreeMap::new();
        for placement in &world_layout.rooms {
            let room_id = placement.room_id;
            rooms.insert(
                room_id,
                RoomData {
                    room_id,
                    position: Some([placement.position % 8, placement.position / 8]),
                    theme: Some(placement.theme.clone()),
                    items: Vec::new(),
                },
            );
        }
        for (location, item) in logic_catalog.item_locations.iter().zip(items) {
            let room_id = location.id.room_id;
            let room = rooms.entry(room_id).or_insert_with(|| RoomData {
                room_id,
                position: None,
                theme: None,
                items: Vec::new(),
            });
            room.items.push(ItemPlacement {
                item_id: location.id.item_id,
                item,
            });
        }
        return Ok(SeedData {
            metadata,
            settings,
            settings_document,
            vanilla_map: world_layout.vanilla,
            rooms: rooms.into_values().collect(),
            borders: world_layout.borders,
        });
    }
    bail!("generation exhausted {MAX_GENERATION_ATTEMPTS} complete attempts")
}

/// Assign fresh keys and run one complete item-placement attempt.
fn generate_items(
    catalog: &LogicCatalog,
    graph: &TraversalGraph,
    inputs: &GenerationInputs,
    rng: &mut impl Rng,
) -> Result<Option<Vec<Item>>> {
    let key_initial_states = KeyPlacementState::initialize_dungeons(catalog, graph, inputs)?;
    let small_keys = assign_small_keys(catalog, graph, inputs, &key_initial_states, rng)?;
    let mut state = PlacementState::initialize(catalog, graph, inputs, &small_keys, rng)?;
    let mut wave = 0;
    while state.placements.contains(&None) {
        wave += 1;
        log::info!("Starting placement wave {wave}");
        let mut accepted = false;
        for _ in 0..MAX_CANDIDATE_ATTEMPTS {
            if let PlacementStep::Accepted = state.place_items(catalog, graph, inputs, rng)? {
                accepted = true;
                break;
            }
        }
        if !accepted {
            return Ok(None);
        }
    }
    if state.global.collected_item_locations.contains(&false) {
        return Ok(None);
    }
    let mut items = Vec::new();
    for item in state.placements {
        items.push(item.unwrap());
    }
    Ok(Some(items))
}
