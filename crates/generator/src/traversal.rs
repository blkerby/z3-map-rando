//! Normal route traversal and permanent-discovery closure for a partial placement.

use crate::{
    graph::TraversalGraph,
    inputs::GenerationInputs,
    interpreter::{GlobalDiscoveries, Interpreter},
    local_state_reducer::LocalStateReducer,
    state::{GlobalState, RefillFlags},
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
    initial_work: &[VertexIndex],
) -> GlobalDiscoveries {
    let mut discoveries = GlobalDiscoveries::default();
    let mut queue = VecDeque::new();
    let mut queued = vec![false; graph.outgoing.len()];
    for &vertex_idx in initial_work {
        if !queued[vertex_idx.0 as usize] {
            queue.push_back(vertex_idx);
            queued[vertex_idx.0 as usize] = true;
        }
    }

    while let Some(vertex_idx) = queue.pop_front() {
        queued[vertex_idx.0 as usize] = false;
        // Self-loops may improve the stored reducer and schedule another expansion.
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
/// Reevaluate all reached vertices when capabilities change; junk collection needs no wave.
pub fn compute_closure(
    catalog: &LogicCatalog,
    graph: &TraversalGraph,
    inputs: &GenerationInputs,
    global: &mut GlobalState,
    local_states: &mut [LocalStateReducer],
    placements: &[Option<Item>],
    mut initial_work: Vec<VertexIndex>,
) -> Result<()> {
    // A new placement may occupy a slot already found by an earlier closure.
    let mut changed = collect_reachable_items(catalog, inputs, global, placements)?;
    loop {
        if changed {
            initial_work.clear();
            for (index, states) in local_states.iter().enumerate() {
                if !states.get_states().is_empty() {
                    initial_work.push(VertexIndex(index as u32));
                }
            }
        }
        if initial_work.is_empty() {
            return Ok(());
        }
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
        changed = !discoveries.flags.is_empty()
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
    }
}

fn collect_reachable_items(
    catalog: &LogicCatalog,
    inputs: &GenerationInputs,
    global: &mut GlobalState,
    placements: &[Option<Item>],
) -> Result<bool> {
    let previous = global.clone();
    for (index, &item) in placements.iter().enumerate() {
        if global.reachable_item_locations[index] && !global.collected_item_locations[index] {
            if let Some(item) = item {
                global.collect_item(catalog, inputs, item, ItemLocationIndex(index as u32))?;
            }
        }
    }
    // Collection records and partial heart pieces do not change route capabilities.
    Ok(global.unique_items != previous.unique_items
        || global.sword_level != previous.sword_level
        || global.shield_level != previous.shield_level
        || global.glove_level != previous.glove_level
        || global.mail_level != previous.mail_level
        || global.boomerang_level != previous.boomerang_level
        || global.ocarina != previous.ocarina
        || global.health_capacity != previous.health_capacity
        || global.magic_usable_capacity != previous.magic_usable_capacity
        || global.bomb_usable_capacity != previous.bomb_usable_capacity
        || global.arrow_usable_capacity != previous.arrow_usable_capacity
        || global.bottles_owned != previous.bottles_owned
        || global.big_keys != previous.big_keys
        || global.unlocked_doors != previous.unlocked_doors)
}
