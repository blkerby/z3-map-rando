//! Main-placement attempt setup and Map/Compass priority for candidate waves.

use crate::{
    graph::TraversalGraph, inputs::GenerationInputs, state::GlobalState, traversal::compute_closure,
};
use anyhow::Result;
use common_types::Item;
use hashbrown::HashMap;
use logic_catalog::{ItemLocationIndex, LogicCatalog};
use rand::{Rng, seq::SliceRandom};

#[derive(Clone, Debug)]
pub struct PlacementState {
    /// Already-placed items indexed by ItemLocationIndex; None is an unfilled slot.
    pub placements: Vec<Option<Item>>,
    /// Remaining unplaced ordinary items; starts as the full pool for each attempt.
    pub ordinary_pool: Vec<Item>,
    /// Outstanding dungeon items (maps/compasses), indexed by DungeonIndex.
    pub dungeon_pools: Vec<Vec<Item>>,
    pub global: GlobalState,
}

impl PlacementState {
    /// Start a fresh attempt with frozen keys and newly shuffled prizes.
    /// Assigned items enter inventory only through normal collection closure.
    pub fn initialize(
        catalog: &LogicCatalog,
        graph: &TraversalGraph,
        inputs: &GenerationInputs,
        small_keys: &HashMap<ItemLocationIndex, Item>,
        rng: &mut impl Rng,
    ) -> Result<Self> {
        let mut placements = vec![None; catalog.item_locations.len()];
        for (&location_idx, &item) in &inputs.fixed_items {
            placements[location_idx.0 as usize] = Some(item);
        }
        for dungeon in &inputs.dungeons {
            for &location_idx in &dungeon.small_key_locations {
                placements[location_idx.0 as usize] = Some(small_keys[&location_idx]);
            }
        }
        let mut prizes = inputs.prize_pool.clone();
        prizes.shuffle(rng);
        for (&location_idx, item) in inputs.prize_locations.iter().zip(prizes) {
            placements[location_idx.0 as usize] = Some(item);
        }
        let mut map_compass_pools = Vec::new();
        for dungeon in &inputs.dungeons {
            map_compass_pools.push(dungeon.map_compass_pool.clone());
        }
        let mut global = GlobalState::initialize(catalog);
        compute_closure(catalog, graph, inputs, &mut global, &placements)?;
        Ok(Self {
            placements,
            ordinary_pool: inputs.ordinary_pool.clone(),
            dungeon_pools: map_compass_pools,
            global,
        })
    }

    /// Place outstanding Maps and Compasses into collectible empty dungeon slots.
    /// Call on a clone of committed closure state before sampling ordinary items.
    /// Items are collected when the complete candidate is evaluated.
    pub fn place_dungeon_items(&mut self, inputs: &GenerationInputs, rng: &mut impl Rng) {
        for (index, dungeon) in inputs.dungeons.iter().enumerate() {
            let pool = &mut self.dungeon_pools[index];
            if pool.is_empty() {
                continue;
            }
            let mut locations = Vec::new();
            for &location_idx in &dungeon.ordinary_locations {
                let index = location_idx.0 as usize;
                if self.global.reachable_item_locations[index] && self.placements[index].is_none() {
                    locations.push(location_idx);
                }
            }
            locations.shuffle(rng);
            let count = pool.len().min(locations.len());
            for (item, location_idx) in sample_items(pool, count, rng).into_iter().zip(locations) {
                self.placements[location_idx.0 as usize] = Some(item);
            }
        }
    }
}

/// Sample `count` items without replacement and remove them from the pool.
/// Requires `count <= pool.len()`; the remaining items may be reordered.
fn sample_items(pool: &mut Vec<Item>, count: usize, rng: &mut impl Rng) -> Vec<Item> {
    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let index = rng.random_range(0..pool.len());
        items.push(pool.swap_remove(index));
    }
    items
}
