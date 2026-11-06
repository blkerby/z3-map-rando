//! Logical generation on vanilla or pool-selected world layouts.

pub mod cost_metrics;
pub mod graph;
pub mod inputs;
pub mod interpreter;
pub mod key_assignment;
pub mod local_state_reducer;
pub mod map;
pub mod placement;
pub mod state;
pub mod traversal;

pub use seed_data::{SeedData, Settings};

use anyhow::{Result, bail, ensure};
use graph::{TraversalGraph, build_graph};
use inputs::{GenerationInputs, prepare_inputs};
use key_assignment::{KeyAssignmentExhausted, KeyPlacementState, assign_small_keys};
use logic_catalog::LogicCatalog;
use map::{MapPool, select_borders};
use placement::{PlacementState, PlacementStep};
use rand::Rng;
use retiling_catalog::RetilingCatalog;
use seed_data::{
    AreaLayout, ItemPlacement, MapLayoutSetting, SEED_FORMAT_VERSION, SeedMetadata, WorldLayout,
};

const MAX_GENERATION_ATTEMPTS: usize = 1000;
const MAX_CANDIDATE_ATTEMPTS: usize = 100;
const MAX_EDGE_SELECTION_ATTEMPTS: usize = 10;

/// Generate fully collectible placements using player settings and the selected world layout.
/// Each complete attempt assigns fresh Small Keys and prizes before placing items.
/// The caller supplies RNG state and metadata; generation needs no ROM or filesystem.
pub fn generate(
    settings: Settings,
    logic_catalog: &LogicCatalog,
    retiling_catalog: &RetilingCatalog,
    theme_name: &str,
    map_pool: Option<&[u8]>,
    metadata: SeedMetadata,
    rng: &mut impl Rng,
) -> Result<SeedData> {
    ensure!(theme_name == "Desert", "only the Desert theme is supported");
    let inputs = prepare_inputs(logic_catalog, &settings)?;
    let pool = if settings.map_layout == MapLayoutSetting::Random {
        Some(MapPool::decode(
            map_pool.ok_or_else(|| anyhow::anyhow!("random maps require a map pool"))?,
            logic_catalog,
        )?)
    } else {
        None
    };

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
    for attempt in 1..=MAX_GENERATION_ATTEMPTS {
        eprintln!("Starting generation attempt {attempt}/{MAX_GENERATION_ATTEMPTS}");
        let mut world_layout = WorldLayout {
            areas: areas.clone(),
            rooms: if let Some(pool) = &pool {
                pool.select_rooms(logic_catalog, rng)?
            } else {
                Vec::new()
            },
            borders: Vec::new(),
        };
        let graph = if pool.is_some() {
            let mut accepted = None;
            for edge_attempt in 1..=MAX_EDGE_SELECTION_ATTEMPTS {
                eprintln!("Checking edge selection {edge_attempt}/{MAX_EDGE_SELECTION_ATTEMPTS}");
                let connections = select_borders(logic_catalog, &mut world_layout, rng)?;
                let graph = build_graph(logic_catalog, &connections);
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
        let placements = match generate_items(logic_catalog, &graph, &inputs, rng) {
            Ok(placements) => placements,
            Err(error) if pool.is_some() && error.is::<KeyAssignmentExhausted>() => {
                eprintln!("Rejected map: {error:#}");
                continue;
            }
            Err(error) => return Err(error),
        };
        if let Some(item_placements) = placements {
            return Ok(SeedData {
                format_version: SEED_FORMAT_VERSION,
                metadata,
                settings,
                world_layout,
                item_placements,
            });
        }
    }
    bail!("generation exhausted {MAX_GENERATION_ATTEMPTS} complete attempts")
}

/// Assign fresh keys and run one complete item-placement attempt.
fn generate_items(
    catalog: &LogicCatalog,
    graph: &TraversalGraph,
    inputs: &GenerationInputs,
    rng: &mut impl Rng,
) -> Result<Option<Vec<ItemPlacement>>> {
    let key_initial_states = KeyPlacementState::initialize_dungeons(catalog, graph, inputs)?;
    let small_keys = assign_small_keys(catalog, graph, inputs, &key_initial_states, rng)?;
    let mut state = PlacementState::initialize(catalog, graph, inputs, &small_keys, rng)?;
    let mut wave = 0;
    while state.placements.contains(&None) {
        wave += 1;
        eprintln!("Starting placement wave {wave}");
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
    let mut placements = Vec::new();
    for (location, item) in catalog.item_locations.iter().zip(state.placements) {
        placements.push(ItemPlacement {
            item_location_id: location.id,
            item: item.unwrap(),
        });
    }
    Ok(Some(placements))
}
