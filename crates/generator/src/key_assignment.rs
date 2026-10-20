//! Preliminary Small Key assignment, settling one dungeon at a time.

use crate::{
    graph::TraversalGraph,
    inputs::{DungeonInputs, GenerationInputs},
    key_traversal::{KeyPassState, traverse_key_pass},
};
use anyhow::{Result, bail};
use common_types::Item;
use hashbrown::HashMap;
use logic_catalog::{DoorIndex, DungeonIndex, ItemLocationIndex, LogicCatalog};
use rand::{Rng, seq::SliceRandom};

const MAX_DUNGEON_ATTEMPTS: usize = 1000;
const MAX_CANDIDATE_ATTEMPTS: usize = 100;

/// Assign every dungeon's vanilla Small Key pickups to distinct same-dungeon doors.
/// Completed dungeons remain settled while a later dungeon retries. Returned items
/// carry stable door identities, ready for the later logical placement phases.
pub fn assign_small_keys(
    catalog: &LogicCatalog,
    graph: &TraversalGraph,
    inputs: &GenerationInputs,
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
            dungeon_idx,
            dungeon,
            &required_locations,
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
    dungeon_idx: DungeonIndex,
    dungeon: &DungeonInputs,
    required_locations: &[ItemLocationIndex],
    rng: &mut impl Rng,
) -> Result<HashMap<ItemLocationIndex, DoorIndex>> {
    let mut inaccessible_locations = Vec::new();
    let mut failure_reason = "";
    for _ in 0..MAX_DUNGEON_ATTEMPTS {
        let mut assignments = HashMap::new();
        let mut state = KeyPassState::initialize(graph);
        collect_dungeon_items(catalog, graph, dungeon_idx, dungeon, &mut state);

        while state.collected_item_locations.len() < required_locations.len() {
            let mut keys = Vec::new();
            for &location_idx in &dungeon.small_key_locations {
                if state.collected_item_locations.contains(&location_idx)
                    && !assignments.contains_key(&location_idx)
                {
                    keys.push(location_idx);
                }
            }
            if keys.is_empty() {
                failure_reason = "no collectible unassigned Small Keys remain";
                break;
            }
            let mut doors = Vec::new();
            for &door_idx in &dungeon.small_key_doors {
                if !state.available_doors.contains(&door_idx) {
                    doors.push(door_idx);
                }
            }

            let mut accepted = false;
            for _ in 0..MAX_CANDIDATE_ATTEMPTS {
                doors.shuffle(rng);
                let mut candidate = state.clone();
                for &door_idx in &doors[..keys.len()] {
                    candidate.available_doors.insert(door_idx);
                }
                collect_dungeon_items(catalog, graph, dungeon_idx, dungeon, &mut candidate);

                let mut found_new_key = false;
                for &location_idx in &dungeon.small_key_locations {
                    if candidate.collected_item_locations.contains(&location_idx)
                        && !state.collected_item_locations.contains(&location_idx)
                    {
                        found_new_key = true;
                        break;
                    }
                }
                if found_new_key
                    || candidate.collected_item_locations.len() == required_locations.len()
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

        if state.collected_item_locations.len() == required_locations.len() {
            let mut keys = Vec::new();
            for &location_idx in &dungeon.small_key_locations {
                if !assignments.contains_key(&location_idx) {
                    keys.push(location_idx);
                }
            }
            let mut doors = Vec::new();
            for &door_idx in &dungeon.small_key_doors {
                if !state.available_doors.contains(&door_idx) {
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
            if !state.collected_item_locations.contains(&location_idx) {
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

/// Apply collection discoveries outside traversal and repeat only for a new Big Key.
fn collect_dungeon_items(
    catalog: &LogicCatalog,
    graph: &TraversalGraph,
    dungeon_idx: DungeonIndex,
    dungeon: &DungeonInputs,
    state: &mut KeyPassState,
) {
    loop {
        let result = traverse_key_pass(catalog, graph, dungeon_idx, state);
        state
            .collected_item_locations
            .extend(result.new_item_locations);
        if state.has_big_key {
            return;
        }
        for location_idx in &dungeon.big_key_locations {
            if state.collected_item_locations.contains(location_idx) {
                state.has_big_key = true;
                break;
            }
        }
        if !state.has_big_key {
            return;
        }
    }
}
