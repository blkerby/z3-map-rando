//! Normal route traversal and permanent-discovery closure for a partial placement.

use crate::{
    graph::TraversalGraph,
    inputs::GenerationInputs,
    interpreter::{GlobalDiscoveries, Interpreter},
    local_state_reducer::LocalStateReducer,
    state::{GlobalState, HealthState, LocalState, RefillFlags},
};
use anyhow::Result;
use common_types::Item;
use logic_catalog::{FollowerMask, ItemLocationIndex, LogicCatalog, OcarinaState};
use std::collections::VecDeque;

/// Metric-selected states at each vertex and discoveries from one fixed-inventory wave.
#[derive(Debug)]
pub struct TraversalResult {
    /// Indexed by VertexIndex; an empty reducer means no route reaches the vertex.
    pub local_states: Vec<LocalStateReducer>,
    pub discoveries: GlobalDiscoveries,
}

/// Start at the graph's starting vertex and expand routes without applying discoveries.
pub fn traverse(graph: &TraversalGraph, interpreter: &Interpreter<'_>) -> TraversalResult {
    let mut result = TraversalResult {
        local_states: vec![LocalStateReducer::default(); graph.outgoing.len()],
        discoveries: GlobalDiscoveries::default(),
    };
    result.local_states[graph.start_vertex_idx.0 as usize].insert(LocalState {
        health: HealthState::Consumed(0),
        magic_used: 0,
        bombs_used: 0,
        arrows_used: 0,
        bottles_used: 0,
        possible_followers: FollowerMask(0),
        cleared_obstacles: 0,
    });
    let mut queue = VecDeque::from([graph.start_vertex_idx]);
    let mut queued = vec![false; graph.outgoing.len()];
    queued[graph.start_vertex_idx.0 as usize] = true;

    while let Some(vertex_idx) = queue.pop_front() {
        queued[vertex_idx.0 as usize] = false;
        // Self-loops may improve the stored reducer and schedule another expansion.
        let states = result.local_states[vertex_idx.0 as usize].clone();
        for edge in &graph.outgoing[vertex_idx.0 as usize] {
            let destination = edge.edge.to_vertex_idx;
            let improved = interpreter.evaluate_edge(
                edge,
                states.clone(),
                &mut result.discoveries,
                &mut result.local_states[destination.0 as usize],
            );
            if improved && !queued[destination.0 as usize] {
                queue.push_back(destination);
                queued[destination.0 as usize] = true;
            }
        }
    }
    result
}

/// Apply discoveries and reachable placed items between fresh traversal waves.
/// Placements have one entry per catalog item location, indexed by ItemLocationIndex.
/// None denotes an unfilled location.
/// Global state retains reachable and collected locations, including earlier waves.
/// Return the final wave's per-vertex reducers; reaching an unfilled location grants no item.
pub fn compute_closure(
    catalog: &LogicCatalog,
    graph: &TraversalGraph,
    inputs: &GenerationInputs,
    global: &mut GlobalState,
    placements: &[Option<Item>],
) -> Result<Vec<LocalStateReducer>> {
    // A new placement may occupy a slot already found by an earlier closure.
    collect_reachable_items(catalog, inputs, global, placements)?;
    loop {
        let result = traverse(
            graph,
            &Interpreter {
                catalog,
                inputs,
                global,
            },
        );
        let discoveries = result.discoveries;
        let mut changed = !discoveries.flags.is_empty()
            || !discoveries.unlocked_doors.is_empty()
            || discoveries.activate_ocarina
            || discoveries.refills != RefillFlags::default();
        for idx in discoveries.item_locations {
            global.reachable_item_locations[idx.0 as usize] = true;
        }
        for idx in discoveries.flags {
            global.flags[idx.0 as usize] = true;
        }
        for idx in discoveries.unlocked_doors {
            global.unlocked_doors[idx.0 as usize] = true;
        }
        if discoveries.activate_ocarina {
            global.ocarina = OcarinaState::Active;
        }
        global.refills.merge(discoveries.refills);
        global.update_usable_capacities();
        changed |= collect_reachable_items(catalog, inputs, global, placements)?;
        if !changed {
            return Ok(result.local_states);
        }
    }
}

fn collect_reachable_items(
    catalog: &LogicCatalog,
    inputs: &GenerationInputs,
    global: &mut GlobalState,
    placements: &[Option<Item>],
) -> Result<bool> {
    let mut collected = false;
    for (index, &item) in placements.iter().enumerate() {
        if global.reachable_item_locations[index] && !global.collected_item_locations[index] {
            if let Some(item) = item {
                global.collect_item(catalog, inputs, item, ItemLocationIndex(index as u32))?;
                collected = true;
            }
        }
    }
    Ok(collected)
}
