//! Normal route traversal and permanent-discovery closure for a partial placement.

use crate::{
    graph::{TraversalEdgeIndex, TraversalGraph},
    inputs::GenerationInputs,
    interpreter::{GlobalDiscoveries, Interpreter},
    local_state_reducer::LocalStateReducer,
    state::{GlobalDependency, GlobalState},
};
use anyhow::Result;
use common_types::Item;
use hashbrown::HashSet;
use logic_catalog::{ItemLocationIndex, LogicCatalog, OcarinaState, VertexIndex};
use std::collections::VecDeque;

/// Evaluate the initial edges, then propagate improved vertices without applying discoveries.
pub fn traverse(
    graph: &TraversalGraph,
    interpreter: &Interpreter<'_>,
    local_states: &mut [LocalStateReducer],
    initial_work: &[TraversalEdgeIndex],
) -> GlobalDiscoveries {
    let mut discoveries = GlobalDiscoveries::default();
    let mut queue = VecDeque::new();
    let mut queued = vec![false; graph.outgoing.len()];
    for &index in initial_work {
        let edge = &graph.outgoing[index.from_vertex_idx.0 as usize][index.outgoing_idx];
        let states = local_states[index.from_vertex_idx.0 as usize].clone();
        let destination = edge.edge.to_vertex_idx;
        let improved = interpreter.evaluate_edge(
            edge,
            states,
            &mut discoveries,
            &mut local_states[destination.0 as usize],
        );
        if improved && !queued[destination.0 as usize] {
            queue.push_back(destination);
            queued[destination.0 as usize] = true;
        }
    }

    while let Some(vertex_idx) = queue.pop_front() {
        // Self-loops may improve the stored reducer and schedule another expansion.
        queued[vertex_idx.0 as usize] = false;
        let states = local_states[vertex_idx.0 as usize].clone();
        for edge in &graph.outgoing[vertex_idx.0 as usize] {
            let destination = edge.edge.to_vertex_idx;
            let improved = interpreter.evaluate_edge(
                edge,
                states.clone(),
                &mut discoveries,
                &mut local_states[destination.0 as usize],
            );
            if improved && !queued[destination.0 as usize] {
                queue.push_back(destination);
                queued[destination.0 as usize] = true;
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
    let mut changes = HashSet::new();
    collect_reachable_items(catalog, inputs, global, placements, &mut changes)?;
    loop {
        for dependency in changes.drain() {
            if let Some(edges) = graph.dependencies.get(&dependency) {
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
        // Discovery-set iteration must not affect metric ties or queue order.
        initial_work.sort_unstable();
        initial_work.dedup();
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
            changes.insert(GlobalDependency::ItemLocation(idx));
        }
        for idx in discoveries.flags {
            global.flags[idx.0 as usize] = true;
            changes.insert(GlobalDependency::Flag(idx));
        }
        for idx in discoveries.unlocked_doors {
            global.unlocked_doors[idx.0 as usize] = true;
            changes.insert(GlobalDependency::Door(idx));
        }
        if discoveries.activate_ocarina {
            global.ocarina = OcarinaState::Active;
            changes.insert(GlobalDependency::Ocarina);
        }
        for (discovered, dependency) in [
            (discoveries.refills.magic, GlobalDependency::Magic),
            (discoveries.refills.bombs, GlobalDependency::Bombs),
            (discoveries.refills.arrows, GlobalDependency::Arrows),
            (discoveries.refills.red_potion, GlobalDependency::RedPotion),
            (
                discoveries.refills.green_potion,
                GlobalDependency::GreenPotion,
            ),
            (
                discoveries.refills.blue_potion,
                GlobalDependency::BluePotion,
            ),
            (discoveries.refills.fairy, GlobalDependency::Fairy),
        ] {
            if discovered {
                changes.insert(dependency);
            }
        }
        global.refills.merge(discoveries.refills);
        global.update_usable_capacities();
        collect_reachable_items(catalog, inputs, global, placements, &mut changes)?;
    }
}

fn collect_reachable_items(
    catalog: &LogicCatalog,
    inputs: &GenerationInputs,
    global: &mut GlobalState,
    placements: &[Option<Item>],
    changes: &mut HashSet<GlobalDependency>,
) -> Result<()> {
    for (index, &item) in placements.iter().enumerate() {
        if global.reachable_item_locations[index] && !global.collected_item_locations[index] {
            if let Some(item) = item {
                if let Some(dependency) =
                    global.collect_item(catalog, inputs, item, ItemLocationIndex(index as u32))?
                {
                    changes.insert(dependency);
                }
            }
        }
    }
    Ok(())
}
