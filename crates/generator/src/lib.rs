//! Logical item generation on the vanilla world graph.

pub mod cost_metrics;
pub mod graph;
pub mod inputs;
pub mod interpreter;
pub mod key_assignment;
pub mod local_state_reducer;
pub mod placement;
pub mod state;
pub mod traversal;

pub use seed_data::{SeedData, Settings};

use anyhow::{Result, bail};
use graph::{TraversalGraph, build_graph};
use inputs::{GenerationInputs, prepare_inputs};
use key_assignment::{KeyPlacementState, assign_small_keys};
use logic_catalog::LogicCatalog;
use placement::{PlacementState, PlacementStep};
use rand::Rng;
use retiling_catalog::RetilingCatalog;
use seed_data::{AreaLayout, ItemPlacement, SEED_FORMAT_VERSION, SeedMetadata, WorldLayout};

const MAX_GENERATION_ATTEMPTS: usize = 1000;
const MAX_CANDIDATE_ATTEMPTS: usize = 100;

/// Generate fully collectible placements using player settings and vanilla connections.
/// Each complete attempt assigns fresh Small Keys and prizes before placing items.
/// The caller supplies RNG state and metadata; generation needs no ROM or filesystem.
pub fn generate(
    settings: Settings,
    logic_catalog: &LogicCatalog,
    retiling_catalog: &RetilingCatalog,
    theme_name: &str,
    metadata: SeedMetadata,
    rng: &mut impl Rng,
) -> Result<SeedData> {
    let inputs = prepare_inputs(logic_catalog, &settings)?;
    let graph = build_graph(logic_catalog, &logic_catalog.vanilla_connections);
    let item_placements = generate_items(logic_catalog, &graph, &inputs, rng)?;

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

/// Retry complete attempts, including their Small Key assignments.
fn generate_items(
    catalog: &LogicCatalog,
    graph: &TraversalGraph,
    inputs: &GenerationInputs,
    rng: &mut impl Rng,
) -> Result<Vec<ItemPlacement>> {
    let key_baseline = KeyPlacementState::initialize(catalog, graph, inputs)?;
    'attempts: for attempt in 1..=MAX_GENERATION_ATTEMPTS {
        eprintln!("Starting generation attempt {attempt}/{MAX_GENERATION_ATTEMPTS}");
        let small_keys = assign_small_keys(catalog, graph, inputs, &key_baseline, rng)?;
        let mut state = PlacementState::initialize(catalog, graph, inputs, &small_keys, rng)?;
        let mut wave = 0;
        while state.placements.contains(&None) {
            wave += 1;
            eprintln!("Starting placement wave {wave} of attempt {attempt}");
            let mut accepted = false;
            for _ in 0..MAX_CANDIDATE_ATTEMPTS {
                if let PlacementStep::Accepted = state.place_items(catalog, graph, inputs, rng)? {
                    accepted = true;
                    break;
                }
            }
            if !accepted {
                continue 'attempts;
            }
        }
        if state.global.collected_item_locations.contains(&false) {
            continue;
        }
        let mut placements = Vec::new();
        for (location, item) in catalog.item_locations.iter().zip(state.placements) {
            placements.push(ItemPlacement {
                item_location_id: location.id,
                item: item.unwrap(),
            });
        }
        return Ok(placements);
    }
    bail!(
        "logical item placement exhausted {MAX_GENERATION_ATTEMPTS} complete attempts ({MAX_CANDIDATE_ATTEMPTS} candidates per wave)",
    )
}
