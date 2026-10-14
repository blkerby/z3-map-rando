//! Relaxed whole-world traversal for one dungeon's preliminary key assignment.

use crate::graph::TraversalGraph;
use hashbrown::HashSet;
use logic_catalog::{
    DoorIndex, Effect, ItemIndex, ItemLocationIndex, LockKind, LogicCatalog, Requirement,
    RoomIndex, VertexIndex,
};
use std::collections::VecDeque;

/// Accumulated reachability and caller-maintained inventory for one dungeon's attempt.
/// Clone this state before trying a candidate so rejected discoveries can be discarded.
#[derive(Clone, Debug)]
pub struct KeyPassState {
    pub visited_vertices: Vec<bool>,
    /// The caller adds doors when assigning previously collected Small Keys.
    /// Traversal leaves inventory and collected locations unchanged.
    pub available_doors: HashSet<DoorIndex>,
    pub has_big_key: bool,
    /// The caller records accepted collections, including keys, ordinary items, and prizes.
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
    dungeon_idx: RoomIndex,
    state: &mut KeyPassState,
) -> KeyPassResult {
    let big_key_item_idx = ItemIndex(
        catalog
            .items
            .iter()
            .position(|item| item.name == "BigKey")
            .unwrap() as u32,
    );
    let mut result = KeyPassResult::default();
    let mut queue = VecDeque::new();
    for (index, &visited) in state.visited_vertices.iter().enumerate() {
        if visited {
            queue.push_back(VertexIndex(index as u32));
        }
    }

    while let Some(vertex_idx) = queue.pop_front() {
        let room_idx = catalog.vertex(vertex_idx).room_idx;
        for traversal_edge in &graph.outgoing[vertex_idx.0 as usize] {
            let edge = &traversal_edge.edge;
            if !check_requirement(
                catalog,
                &edge.requirement,
                room_idx,
                dungeon_idx,
                state,
                big_key_item_idx,
            ) {
                continue;
            }

            // Effects run even if the destination was already visited, including
            // self-loop collections. Reaching a pickup vertex alone collects nothing.
            for effect in &edge.effects {
                let Effect::CollectItem(location_idx) = *effect else {
                    continue;
                };
                let location = catalog.item_location(location_idx);
                if catalog.vertex(location.vertex_idx).room_idx != dungeon_idx
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
    room_idx: RoomIndex,
    dungeon_idx: RoomIndex,
    state: &KeyPassState,
    big_key_item_idx: ItemIndex,
) -> bool {
    match requirement {
        Requirement::And(children) => {
            for child in children {
                if !check_requirement(
                    catalog,
                    child,
                    room_idx,
                    dungeon_idx,
                    state,
                    big_key_item_idx,
                ) {
                    return false;
                }
            }
            true
        }
        Requirement::Or(children) => {
            for child in children {
                if check_requirement(
                    catalog,
                    child,
                    room_idx,
                    dungeon_idx,
                    state,
                    big_key_item_idx,
                ) {
                    return true;
                }
            }
            false
        }
        Requirement::Item(item_idx) => {
            room_idx != dungeon_idx || *item_idx != big_key_item_idx || state.has_big_key
        }
        Requirement::Door(door_idx) => {
            let door = catalog.door(*door_idx);
            if door.room_idx != dungeon_idx {
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
