//! One candidate of logical item placement, with Map/Compass priority.

use crate::{
    graph::TraversalGraph,
    inputs::{GenerationInputs, InvalidSettings},
    local_state_reducer::LocalStateReducer,
    state::{GlobalState, LocalState},
    traversal::compute_closure,
};
use anyhow::Result;
use common_types::Item;
use hashbrown::{HashMap, HashSet};
use logic_catalog::{ItemLocationIndex, LogicCatalog};
use rand::{Rng, seq::SliceRandom};
use seed_data::{
    StartingBoomerang, StartingBow, StartingGlove, StartingMail, StartingShield, StartingSword,
};

const FAIRY_UPGRADE_ITEMS: [Item; 8] = [
    Item::Boomerang,
    Item::Mail,
    Item::Shield,
    Item::Sword,
    Item::Bow,
    Item::Glove,
    Item::BombCapacityUpgrade,
    Item::ArrowCapacityUpgrade,
];

/// Choose an upgrade item for each Great Fairy location sequentially and uniformly from the
/// eligible upgrade classes, allowing a class to repeat when enough copies remain to satisfy its
/// prerequisite outside the fairies. Fall back to `Item::Nothing` when no eligible upgrade remains.
pub fn select_fairy_upgrades(
    inputs: &GenerationInputs,
    rng: &mut impl Rng,
) -> Result<Vec<(ItemLocationIndex, Item)>> {
    let equipment = &inputs.starting_state.equipment;
    let resources = &inputs.starting_state.resources;
    let mut remaining = [0_usize; 8];
    for (index, &upgrade) in FAIRY_UPGRADE_ITEMS.iter().enumerate() {
        let mut pool_count = 0_usize;
        for &item in &inputs.ordinary_pool {
            if item == upgrade {
                pool_count += 1;
            }
        }
        let has_prereq = match upgrade {
            Item::Boomerang => equipment.boomerang != StartingBoomerang::None,
            Item::Mail => equipment.mail != StartingMail::Green,
            Item::Shield => equipment.shield != StartingShield::None,
            Item::Sword => equipment.sword != StartingSword::None,
            Item::Bow => equipment.bow != StartingBow::None,
            Item::Glove => equipment.glove != StartingGlove::None,
            Item::BombCapacityUpgrade => resources.bomb_capacity > 0,
            Item::ArrowCapacityUpgrade => resources.arrow_capacity > 0,
            _ => false,
        };
        remaining[index] = if has_prereq {
            pool_count
        } else {
            pool_count.saturating_sub(1)
        };
    }
    let mut nothing_count = 0_usize;
    for &item in &inputs.ordinary_pool {
        if item == Item::Nothing {
            nothing_count += 1;
        }
    }

    let mut locations = inputs.fairy_locations.clone();
    locations.shuffle(rng);
    let mut selected = Vec::with_capacity(locations.len());
    for location_idx in locations {
        let mut eligible = Vec::new();
        for (index, &count) in remaining.iter().enumerate() {
            if count > 0 {
                eligible.push(index);
            }
        }
        let item = if !eligible.is_empty() {
            let class_index = eligible[rng.random_range(0..eligible.len())];
            remaining[class_index] -= 1;
            FAIRY_UPGRADE_ITEMS[class_index]
        } else if nothing_count > 0 {
            nothing_count -= 1;
            Item::Nothing
        } else {
            return Err(InvalidSettings(
                "item pool has no eligible Great Fairy upgrades or empty slots".to_owned(),
            )
            .into());
        };
        selected.push((location_idx, item));
    }
    Ok(selected)
}

/// An accepted candidate updates the state; rejection leaves it unchanged.
pub enum PlacementStep {
    Accepted,
    Failed(&'static str),
}

#[derive(Clone, Debug)]
pub struct PlacementState {
    /// Already-placed items indexed by ItemLocationIndex; None is an unfilled slot.
    pub placements: Vec<Option<Item>>,
    /// Remaining unplaced ordinary items; starts as the full pool for each attempt.
    pub ordinary_pool: Vec<Item>,
    /// Outstanding dungeon items (maps/compasses), indexed by DungeonIndex.
    pub dungeon_pools: Vec<Vec<Item>>,
    pub global: GlobalState,
    /// Retained routes indexed by VertexIndex, cloned and committed with inventory.
    pub local_states: Vec<LocalStateReducer>,
}

impl PlacementState {
    /// Start a fresh attempt with the supplied keys, Great Fairy upgrades, and newly shuffled
    /// prizes. Assigned items enter inventory only through normal collection closure.
    pub fn initialize(
        catalog: &LogicCatalog,
        graph: &TraversalGraph,
        inputs: &GenerationInputs,
        small_keys: &HashMap<ItemLocationIndex, Item>,
        fairy_items: &[(ItemLocationIndex, Item)],
        rng: &mut impl Rng,
    ) -> Result<Self> {
        let mut placements = vec![None; catalog.item_locations.len()];
        let mut ordinary_pool = inputs.ordinary_pool.clone();
        for (&location_idx, &item) in &inputs.fixed_items {
            placements[location_idx.0 as usize] = Some(item);
        }
        for &(location_idx, item) in fairy_items {
            placements[location_idx.0 as usize] = Some(item);
            let pool_index = ordinary_pool
                .iter()
                .position(|&pool_item| pool_item == item)
                .unwrap();
            ordinary_pool.swap_remove(pool_index);
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
        let mut global = GlobalState::initialize(catalog, inputs);
        let mut local_states = vec![LocalStateReducer::default(); graph.outgoing.len()];
        local_states[graph.start_vertex_idx.0 as usize].insert(LocalState::initialize());
        compute_closure(
            catalog,
            graph,
            inputs,
            &mut global,
            &mut local_states,
            &placements,
            vec![graph.start_vertex_idx],
            HashSet::new(),
            |_, _| {},
        )?;
        Ok(Self {
            placements,
            ordinary_pool,
            dungeon_pools: map_compass_pools,
            global,
            local_states,
        })
    }

    /// Sample and evaluate one candidate from committed closure state.
    /// The caller owns candidate retries, wave sequencing, and attempts with fresh keys.
    pub fn place_items(
        &mut self,
        catalog: &LogicCatalog,
        graph: &TraversalGraph,
        inputs: &GenerationInputs,
        rng: &mut impl Rng,
    ) -> Result<PlacementStep> {
        let mut candidate = self.clone();
        let dungeon_count = candidate.place_dungeon_items(inputs, rng);
        let ordinary_count = candidate.place_ordinary_items(inputs, rng);
        if dungeon_count == 0 && ordinary_count == 0 {
            return Ok(PlacementStep::Failed(
                "no collectible unfilled ordinary locations remain",
            ));
        }
        compute_closure(
            catalog,
            graph,
            inputs,
            &mut candidate.global,
            &mut candidate.local_states,
            &candidate.placements,
            Vec::new(),
            HashSet::new(),
            |_, _| {},
        )?;
        if check_candidate_progression(self, &candidate) {
            *self = candidate;
            Ok(PlacementStep::Accepted)
        } else if ordinary_count == 0 {
            Ok(PlacementStep::Failed(
                "mandatory Map/Compass placements consumed every collectible slot without exposing another item location",
            ))
        } else {
            Ok(PlacementStep::Failed(
                "candidate did not expose another collectible item location",
            ))
        }
    }

    /// Place outstanding Maps and Compasses into collectible empty dungeon slots; return the number filled.
    /// Call on a clone of committed closure state before sampling ordinary items.
    /// Items are collected when the complete candidate is evaluated.
    pub fn place_dungeon_items(&mut self, inputs: &GenerationInputs, rng: &mut impl Rng) -> usize {
        let mut placed_count = 0;
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
            placed_count += count;
        }
        placed_count
    }

    /// Fill remaining collectible ordinary slots in catalog order; return the number filled.
    fn place_ordinary_items(&mut self, inputs: &GenerationInputs, rng: &mut impl Rng) -> usize {
        let mut locations = Vec::new();
        for &location_idx in &inputs.ordinary_locations {
            let index = location_idx.0 as usize;
            if self.global.reachable_item_locations[index] && self.placements[index].is_none() {
                locations.push(location_idx);
            }
        }
        let count = locations.len();
        let items = sample_items(&mut self.ordinary_pool, count, rng);
        for (location_idx, item) in locations.into_iter().zip(items) {
            self.placements[location_idx.0 as usize] = Some(item);
        }
        count
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

/// Reachability only grows; accept new collection actions or a fully collectible world.
fn check_candidate_progression(committed: &PlacementState, candidate: &PlacementState) -> bool {
    !candidate.global.reachable_item_locations.contains(&false)
        || candidate.global.reachable_item_locations != committed.global.reachable_item_locations
}
