//! Normal route traversal and permanent-discovery closure for a partial placement.

use crate::{
    graph::{TraversalEdgeIndex, TraversalGraph},
    inputs::GenerationInputs,
    interpreter::{GlobalDiscoveries, Interpreter},
    local_state_reducer::LocalStateReducer,
    state::GlobalState,
};
use anyhow::Result;
use common_types::Item;
use logic_catalog::{ItemLocationIndex, LogicCatalog, OcarinaState, VertexIndex};
use std::collections::VecDeque;

/// Extend retained routes from the supplied work set without applying discoveries.
pub fn traverse(
    graph: &TraversalGraph,
    interpreter: &Interpreter<'_>,
    local_states: &mut [LocalStateReducer],
    initial_work: &[TraversalEdgeIndex],
) -> GlobalDiscoveries {
    let mut discoveries = GlobalDiscoveries::default();
    let mut queue = VecDeque::new();
    let mut queued = Vec::with_capacity(graph.outgoing.len());
    for outgoing in &graph.outgoing {
        queued.push(vec![false; outgoing.len()]);
    }
    for &index in initial_work {
        if !queued[index.from_vertex_idx.0 as usize][index.outgoing_idx] {
            queue.push_back(index);
            queued[index.from_vertex_idx.0 as usize][index.outgoing_idx] = true;
        }
    }

    while let Some(index) = queue.pop_front() {
        // Clear before evaluation so self-loop improvements can schedule this edge again.
        queued[index.from_vertex_idx.0 as usize][index.outgoing_idx] = false;
        let edge = &graph.outgoing[index.from_vertex_idx.0 as usize][index.outgoing_idx];
        let states = local_states[index.from_vertex_idx.0 as usize].clone();
        let destination = edge.edge.to_vertex_idx;
        let improved = interpreter.evaluate_edge(
            edge,
            states,
            &mut discoveries,
            &mut local_states[destination.0 as usize],
        );
        if improved {
            for outgoing_idx in 0..graph.outgoing[destination.0 as usize].len() {
                if !queued[destination.0 as usize][outgoing_idx] {
                    queue.push_back(TraversalEdgeIndex {
                        from_vertex_idx: destination,
                        outgoing_idx,
                    });
                    queued[destination.0 as usize][outgoing_idx] = true;
                }
            }
        }
    }
    discoveries
}

/// Extend retained routes, applying discoveries and reachable placed items between waves.
/// Placements have one entry per catalog item location, indexed by ItemLocationIndex.
/// None denotes an unfilled location.
/// Global state retains reachable and collected locations, including earlier waves.
/// Global changes seed only dependent edges with retained source states.
pub fn compute_closure(
    catalog: &LogicCatalog,
    graph: &TraversalGraph,
    inputs: &GenerationInputs,
    global: &mut GlobalState,
    local_states: &mut [LocalStateReducer],
    placements: &[Option<Item>],
    initial_vertices: Vec<VertexIndex>,
) -> Result<()> {
    let mut initial_work = Vec::new();
    for vertex_idx in initial_vertices {
        for outgoing_idx in 0..graph.outgoing[vertex_idx.0 as usize].len() {
            initial_work.push(TraversalEdgeIndex {
                from_vertex_idx: vertex_idx,
                outgoing_idx,
            });
        }
    }
    // A new placement may occupy a slot already found by an earlier closure.
    let mut previous = global.clone();
    collect_reachable_items(catalog, inputs, global, placements)?;
    loop {
        for (&dependency, edges) in &graph.dependencies {
            if dependency.has_changed(&previous, global) {
                for &index in edges {
                    if !local_states[index.from_vertex_idx.0 as usize]
                        .get_states()
                        .is_empty()
                    {
                        initial_work.push(index);
                    }
                }
            }
        }
        if initial_work.is_empty() {
            return Ok(());
        }
        // Hash-map iteration must not affect metric ties or queue order.
        initial_work.sort_unstable();
        initial_work.dedup();
        previous = global.clone();
        let discoveries = traverse(
            graph,
            &Interpreter {
                catalog,
                inputs,
                global,
            },
            local_states,
            &initial_work,
        );
        initial_work.clear();
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
        collect_reachable_items(catalog, inputs, global, placements)?;
    }
}

fn collect_reachable_items(
    catalog: &LogicCatalog,
    inputs: &GenerationInputs,
    global: &mut GlobalState,
    placements: &[Option<Item>],
) -> Result<()> {
    for (index, &item) in placements.iter().enumerate() {
        if global.reachable_item_locations[index] && !global.collected_item_locations[index] {
            if let Some(item) = item {
                global.collect_item(catalog, inputs, item, ItemLocationIndex(index as u32))?;
            }
        }
    }
    Ok(())
}
