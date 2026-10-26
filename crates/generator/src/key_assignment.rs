//! Preliminary Small Key assignment, settling one dungeon at a time.
//! This assumes a full item loadout, with the exception of the given
//! dungeon's keys while they are being assigned.

use crate::{
    graph::TraversalGraph,
    inputs::GenerationInputs,
    local_state_reducer::LocalStateReducer,
    state::{GlobalDependency, GlobalState, LocalState},
    traversal::compute_closure,
};
use anyhow::{Result, bail};
use common_types::Item;
use hashbrown::{HashMap, HashSet};
use logic_catalog::{DoorIndex, DungeonIndex, ItemLocationIndex, LockKind, LogicCatalog};
use rand::{Rng, seq::SliceRandom};

const MAX_DUNGEON_ATTEMPTS: usize = 1000;
const MAX_CANDIDATE_ATTEMPTS: usize = 100;

/// Retained routes and placements, including the shared no-key baseline.
#[derive(Clone)]
pub struct KeyPlacementState {
    global: GlobalState,
    local_states: Vec<LocalStateReducer>,
    placements: Vec<Option<Item>>,
}

impl KeyPlacementState {
    /// Build the shared full-inventory closure without dungeon keys or placed items.
    pub fn initialize(
        catalog: &LogicCatalog,
        graph: &TraversalGraph,
        inputs: &GenerationInputs,
    ) -> Result<Self> {
        let mut global = GlobalState::initialize(catalog);
        for &item in &inputs.ordinary_pool {
            global.grant_non_key_item(item);
        }
        for &item in inputs.fixed_items.values() {
            global.grant_non_key_item(item);
        }
        for &item in &inputs.prize_pool {
            global.grant_non_key_item(item);
        }
        let mut initial = Self {
            global,
            local_states: vec![LocalStateReducer::default(); graph.outgoing.len()],
            placements: vec![None; catalog.item_locations.len()],
        };
        initial.local_states[graph.start_vertex_idx.0 as usize].insert(LocalState::initialize());
        compute_closure(
            catalog,
            graph,
            inputs,
            &mut initial.global,
            &mut initial.local_states,
            &initial.placements,
            vec![graph.start_vertex_idx],
            HashSet::new(),
        )?;
        Ok(initial)
    }
}

/// Assign every dungeon's vanilla Small Key pickups to distinct same-dungeon doors.
/// Completed dungeons remain settled while a later dungeon retries. Returned items
/// carry stable door identities, ready for the later logical placement phases.
/// Reuse the baseline initialized for this catalog, graph, and inputs across attempts.
pub fn assign_small_keys(
    catalog: &LogicCatalog,
    graph: &TraversalGraph,
    inputs: &GenerationInputs,
    baseline: &KeyPlacementState,
    rng: &mut impl Rng,
) -> Result<HashMap<ItemLocationIndex, Item>> {
    let mut items = HashMap::new();
    for (index, dungeon) in inputs.dungeons.iter().enumerate() {
        let dungeon_idx = DungeonIndex(index as u32);
        let room_idx = catalog.dungeons[index];
        let mut required_locations = Vec::new();
        for (index, location) in catalog.item_locations.iter().enumerate() {
            if catalog.vertex(location.vertex_idx).room_idx == room_idx {
                required_locations.push(ItemLocationIndex(index as u32));
            }
        }
        let assignments = assign_dungeon_keys(
            catalog,
            graph,
            inputs,
            dungeon_idx,
            &required_locations,
            baseline,
            rng,
        )?;
        // Catalog order keeps even item construction independent of hash iteration.
        for &location_idx in &dungeon.small_key_locations {
            let door_idx = assignments[&location_idx];
            items.insert(
                location_idx,
                Item::SmallKey {
                    door_id: inputs.door_ids[door_idx.0 as usize],
                },
            );
        }
    }
    Ok(items)
}

fn assign_dungeon_keys(
    catalog: &LogicCatalog,
    graph: &TraversalGraph,
    inputs: &GenerationInputs,
    dungeon_idx: DungeonIndex,
    required_locations: &[ItemLocationIndex],
    baseline: &KeyPlacementState,
    rng: &mut impl Rng,
) -> Result<HashMap<ItemLocationIndex, DoorIndex>> {
    let dungeon = &inputs.dungeons[dungeon_idx.0 as usize];
    let mut initial = baseline.clone();
    let mut changes = HashSet::new();

    // Assume having all big and small keys except for the given dungeon.
    for index in 0..catalog.dungeons.len() {
        let other_dungeon_idx = DungeonIndex(index as u32);
        if other_dungeon_idx != dungeon_idx {
            initial.global.big_keys[index] = true;
            changes.insert(GlobalDependency::BigKey(other_dungeon_idx));
        }
    }
    for (index, door) in catalog.doors.iter().enumerate() {
        if door.lock == LockKind::SmallKey
            && catalog.room(door.room_idx).dungeon_idx != Some(dungeon_idx)
            && !initial.global.unlocked_doors[index]
        {
            initial.global.unlocked_doors[index] = true;
            changes.insert(GlobalDependency::Door(DoorIndex(index as u32)));
        }
    }
    for &location_idx in &dungeon.big_key_locations {
        initial.placements[location_idx.0 as usize] = Some(Item::BigKey);
    }
    compute_closure(
        catalog,
        graph,
        inputs,
        &mut initial.global,
        &mut initial.local_states,
        &initial.placements,
        Vec::new(),
        changes,
    )?;
    let mut inaccessible_locations = Vec::new();
    let mut failure_reason = "";
    for _ in 0..MAX_DUNGEON_ATTEMPTS {
        let mut assignments = HashMap::new();
        let mut state = initial.clone();

        while required_locations
            .iter()
            .any(|idx| !state.global.reachable_item_locations[idx.0 as usize])
        {
            let mut keys = Vec::new();
            for &location_idx in &dungeon.small_key_locations {
                if state.global.reachable_item_locations[location_idx.0 as usize]
                    && !assignments.contains_key(&location_idx)
                {
                    keys.push(location_idx);
                }
            }
            if keys.is_empty() {
                failure_reason = "no collectible unassigned Small Keys remain";
                break;
            }
            // Normal logic can open doors without keys; only assignments reserve identities.
            let mut doors = Vec::new();
            for &door_idx in &dungeon.small_key_doors {
                if !assignments.values().any(|&assigned| assigned == door_idx) {
                    doors.push(door_idx);
                }
            }

            let mut accepted = false;
            for _ in 0..MAX_CANDIDATE_ATTEMPTS {
                doors.shuffle(rng);
                let mut candidate = state.clone();
                for (&location_idx, &door_idx) in keys.iter().zip(&doors) {
                    candidate.placements[location_idx.0 as usize] = Some(Item::SmallKey {
                        door_id: inputs.door_ids[door_idx.0 as usize],
                    });
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
                )?;

                let mut found_new_key = false;
                for &location_idx in &dungeon.small_key_locations {
                    if candidate.global.reachable_item_locations[location_idx.0 as usize]
                        && !state.global.reachable_item_locations[location_idx.0 as usize]
                    {
                        found_new_key = true;
                        break;
                    }
                }
                if found_new_key
                    || required_locations
                        .iter()
                        .all(|idx| candidate.global.reachable_item_locations[idx.0 as usize])
                {
                    for (&location_idx, &door_idx) in keys.iter().zip(&doors) {
                        assignments.insert(location_idx, door_idx);
                    }
                    state = candidate;
                    accepted = true;
                    break;
                }
            }
            if !accepted {
                failure_reason = "candidate budget exhausted without exposing another Small Key or collecting every dungeon item";
                break;
            }
        }

        if required_locations
            .iter()
            .all(|idx| state.global.reachable_item_locations[idx.0 as usize])
        {
            let mut keys = Vec::new();
            for &location_idx in &dungeon.small_key_locations {
                if !assignments.contains_key(&location_idx) {
                    keys.push(location_idx);
                }
            }
            let mut doors = Vec::new();
            for &door_idx in &dungeon.small_key_doors {
                if !assignments.values().any(|&assigned| assigned == door_idx) {
                    doors.push(door_idx);
                }
            }
            // All remaining pickups are already collectible, so their pairing cannot
            // introduce a collection cycle, even if their doors were never needed.
            doors.shuffle(rng);
            for (location_idx, door_idx) in keys.into_iter().zip(doors) {
                assignments.insert(location_idx, door_idx);
            }
            return Ok(assignments);
        }

        inaccessible_locations.clear();
        for &location_idx in required_locations {
            if !state.global.reachable_item_locations[location_idx.0 as usize] {
                inaccessible_locations.push(location_idx);
            }
        }
    }

    let mut inaccessible_names = Vec::new();
    for location_idx in inaccessible_locations {
        let location = catalog.item_location(location_idx);
        inaccessible_names.push(format!("{} ({:?})", location.name, location.id));
    }
    let dungeon = catalog.room(catalog.dungeons[dungeon_idx.0 as usize]);
    bail!(
        "preliminary key assignment for {} ({:?}) exhausted {} dungeon attempts ({} candidates per wave); last attempt: {}; inaccessible locations: {}.",
        dungeon.name,
        dungeon.room_id,
        MAX_DUNGEON_ATTEMPTS,
        MAX_CANDIDATE_ATTEMPTS,
        failure_reason,
        inaccessible_names.join(", "),
    )
}
