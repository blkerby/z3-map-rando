//! Relaxed whole-world traversal for one dungeon's preliminary key assignment.

use crate::graph::TraversalGraph;
use hashbrown::HashSet;
use logic_catalog::{
    DoorIndex, DungeonIndex, Effect, ItemLocationIndex, LockKind, LogicCatalog, Requirement,
    VertexIndex,
};
use std::collections::VecDeque;

/// Accumulated reachability and inventory maintained by the key-assignment algorithm
/// for one dungeon's attempt.
/// Clone this state before trying a candidate so rejected discoveries can be discarded.
#[derive(Clone, Debug)]
pub struct KeyPassState {
    pub visited_vertices: Vec<bool>,
    /// The key-assignment algorithm adds doors when assigning previously collected Small Keys.
    /// Traversal leaves inventory and collected locations unchanged.
    pub available_doors: HashSet<DoorIndex>,
    pub has_big_key: bool,
    /// The key-assignment algorithm records accepted collections,
    /// including keys, ordinary items, and prizes.
    pub collected_item_locations: HashSet<ItemLocationIndex>,
}

impl KeyPassState {
    /// Begin an assignment attempt with only the world's start visited.
    pub fn initialize(graph: &TraversalGraph) -> Self {
        let mut visited_vertices = vec![false; graph.outgoing.len()];
        visited_vertices[graph.start_vertex_idx.0 as usize] = true;
        Self {
            visited_vertices,
            available_doors: HashSet::new(),
            has_big_key: false,
            collected_item_locations: HashSet::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct KeyPassResult {
    /// Successful collection actions not already recorded in the input state.
    pub new_item_locations: HashSet<ItemLocationIndex>,
}

/// Expand the input state's visited vertices using its unchanged inventory.
/// Only the selected dungeon's locks are enforced. The caller records collections,
/// applies key inventory updates, and initiates another traversal when needed.
/// Sort discoveries before using their iteration order to construct randomized candidates.
pub fn traverse_key_pass(
    catalog: &LogicCatalog,
    graph: &TraversalGraph,
    dungeon_idx: DungeonIndex,
    state: &mut KeyPassState,
) -> KeyPassResult {
    let mut result = KeyPassResult::default();
    let mut queue = VecDeque::new();
    for (index, &visited) in state.visited_vertices.iter().enumerate() {
        if visited {
            queue.push_back(VertexIndex(index as u32));
        }
    }

    while let Some(vertex_idx) = queue.pop_front() {
        for traversal_edge in &graph.outgoing[vertex_idx.0 as usize] {
            let edge = &traversal_edge.edge;
            if !check_requirement(catalog, &edge.requirement, dungeon_idx, state) {
                continue;
            }

            // Effects run even if the destination was already visited, including
            // self-loop collections. Reaching a pickup vertex alone collects nothing.
            for effect in &edge.effects {
                let Effect::CollectItem(location_idx) = *effect else {
                    continue;
                };
                let location = catalog.item_location(location_idx);
                let room = catalog.room(catalog.vertex(location.vertex_idx).room_idx);
                if room.dungeon_idx != Some(dungeon_idx)
                    || state.collected_item_locations.contains(&location_idx)
                {
                    continue;
                }
                result.new_item_locations.insert(location_idx);
            }

            let destination = edge.to_vertex_idx;
            if !state.visited_vertices[destination.0 as usize] {
                state.visited_vertices[destination.0 as usize] = true;
                queue.push_back(destination);
            }
        }
    }
    result
}

fn check_requirement(
    catalog: &LogicCatalog,
    requirement: &Requirement,
    dungeon_idx: DungeonIndex,
    state: &KeyPassState,
) -> bool {
    match requirement {
        Requirement::And(children) => {
            for child in children {
                if !check_requirement(catalog, child, dungeon_idx, state) {
                    return false;
                }
            }
            true
        }
        Requirement::Or(children) => {
            for child in children {
                if check_requirement(catalog, child, dungeon_idx, state) {
                    return true;
                }
            }
            false
        }
        Requirement::BigKey(required_dungeon_idx) => {
            *required_dungeon_idx != dungeon_idx || state.has_big_key
        }
        Requirement::Door(door_idx) => {
            let door = catalog.door(*door_idx);
            if catalog.room(door.room_idx).dungeon_idx != Some(dungeon_idx) {
                return true;
            }
            match door.lock {
                LockKind::SmallKey => state.available_doors.contains(door_idx),
                LockKind::BigKey => state.has_big_key,
                _ => true,
            }
        }
        // All non-key leaves, including Never, are relaxed for this pass.
        // Temporary state and non-collection effects do not constrain traversal.
        _ => true,
    }
}
