//! Replay a saved seed's logical traversal to group its item locations into spheres, and
//! trace routes to them.

use crate::{
    graph::{TraversalEdgeIndex, TraversalGraph, build_graph},
    inputs::{GenerationInputs, prepare_inputs},
    interpreter::{GlobalDiscoveries, Interpreter},
    local_state_reducer::LocalStateReducer,
    map::{build_connections, create_room_placement},
    state::{GlobalState, LocalState},
    traversal::compute_closure,
};
use anyhow::{Context, Result, bail};
use common_types::{Item, ItemLocationId, SourceRoomId};
use hashbrown::HashSet;
use logic_catalog::{Connection, Effect, ItemLocationIndex, LogicCatalog, VertexIndex};
use seed_data::{SeedData, WorldLayout};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap},
};

pub struct SpoilerReplay {
    /// Item locations first collectible in each sphere, in catalog order.
    pub spheres: Vec<Vec<ItemLocationIndex>>,
    /// Item locations that the current logic never reaches.
    pub unreachable: Vec<ItemLocationIndex>,
    /// The seed's item at each location, indexed by `ItemLocationIndex`.
    pub items: Vec<Option<Item>>,
    /// The seed's room connections, indexed by `TraversalEdgeOrigin::Connection`.
    pub connections: Vec<Connection>,
    pub graph: TraversalGraph,
    inputs: GenerationInputs,
    /// Global state at the closure fixpoint that first reaches each sphere, indexed like
    /// `spheres`: the items of all earlier spheres, with every discovery they allow.
    sphere_states: Vec<GlobalState>,
}

/// A route position: the edge taken to reach a vertex, and the local state on arrival.
#[derive(Clone)]
pub struct RouteStep {
    /// None for the route's start.
    pub edge: Option<TraversalEdgeIndex>,
    pub vertex_idx: VertexIndex,
    pub state: LocalState,
}

/// A search entry, linked to the entry it was reached from.
struct RouteEntry {
    step: RouteStep,
    parent_entry_idx: Option<usize>,
}

/// Replay the seed against the current logic catalog. Sphere 1 holds the locations
/// collectible with the starting inventory, and each later sphere holds the locations newly
/// collectible with the items of all earlier spheres.
pub fn replay_spoiler(catalog: &LogicCatalog, seed: &SeedData) -> Result<SpoilerReplay> {
    let inputs = prepare_inputs(catalog, &seed.settings)?;

    let mut catalog_rooms = BTreeMap::new();
    for room in &catalog.rooms {
        if let Some(room_id) = room.room_id {
            catalog_rooms.insert(room_id, room);
        }
    }
    let mut location_indices = BTreeMap::new();
    for (index, location) in catalog.item_locations.iter().enumerate() {
        location_indices.insert(location.id, ItemLocationIndex(index as u32));
    }

    let mut rooms = Vec::new();
    let mut seed_placements = vec![None; catalog.item_locations.len()];
    for room in &seed.rooms {
        if let (SourceRoomId::Overworld(_), Some([x, y]), Some(theme)) =
            (room.room_id, room.position, &room.theme)
        {
            let catalog_room = catalog_rooms.get(&room.room_id).with_context(|| {
                format!("seed room {:?} is not in the logic catalog", room.room_id)
            })?;
            rooms.push(create_room_placement(catalog_room, y * 8 + x, theme));
        }
        for placement in &room.items {
            let id = ItemLocationId {
                room_id: room.room_id,
                item_id: placement.item_id,
            };
            let Some(&index) = location_indices.get(&id) else {
                bail!("seed item location {id:?} is not in the logic catalog");
            };
            if let Item::SmallKey(door_id) = placement.item
                && !inputs.door_indices.contains_key(&door_id)
            {
                bail!("seed Small Key door {door_id:?} is not in the logic catalog");
            }
            seed_placements[index.0 as usize] = Some(placement.item);
        }
    }
    let layout = WorldLayout {
        vanilla: seed.vanilla_map,
        rooms,
        borders: seed.borders.clone(),
    };
    let connections = build_connections(catalog, &layout);
    let graph = build_graph(catalog, &connections);

    // Each closure collects only the placements of earlier spheres, so the locations it newly
    // reaches form the next sphere. Traversal state carries over, so each closure only
    // revisits edges affected by the newly collected items.
    let mut global = GlobalState::initialize(catalog, &inputs);
    let mut local_states = vec![LocalStateReducer::default(); graph.outgoing.len()];
    local_states[graph.start_vertex_idx.0 as usize].insert(LocalState::initialize());
    let mut placements = vec![None; catalog.item_locations.len()];
    compute_closure(
        catalog,
        &graph,
        &inputs,
        &mut global,
        &mut local_states,
        &placements,
        vec![graph.start_vertex_idx],
        HashSet::new(),
    )?;
    let mut assigned = vec![false; catalog.item_locations.len()];
    let mut spheres = Vec::new();
    let mut sphere_states = Vec::new();
    loop {
        let mut sphere = Vec::new();
        for index in 0..catalog.item_locations.len() {
            if global.reachable_item_locations[index] && !assigned[index] {
                assigned[index] = true;
                placements[index] = seed_placements[index];
                sphere.push(ItemLocationIndex(index as u32));
            }
        }
        if sphere.is_empty() {
            break;
        }
        spheres.push(sphere);
        sphere_states.push(global.clone());
        compute_closure(
            catalog,
            &graph,
            &inputs,
            &mut global,
            &mut local_states,
            &placements,
            Vec::new(),
            HashSet::new(),
        )?;
    }
    let mut unreachable = Vec::new();
    for (index, &assigned) in assigned.iter().enumerate() {
        if !assigned {
            unreachable.push(ItemLocationIndex(index as u32));
        }
    }
    Ok(SpoilerReplay {
        spheres,
        unreachable,
        items: seed_placements,
        connections,
        graph,
        inputs,
        sphere_states,
    })
}

impl SpoilerReplay {
    /// Trace a minimum-cost route from the start to collecting a location, with the global
    /// state from which its sphere is first reachable. Each edge costs 1. Return None for an
    /// unreachable location, or if the search's state pruning loses every route.
    pub fn trace_route(
        &self,
        catalog: &LogicCatalog,
        location_idx: ItemLocationIndex,
    ) -> Option<Vec<RouteStep>> {
        let mut global = None;
        for (sphere, state) in self.spheres.iter().zip(&self.sphere_states) {
            if sphere.contains(&location_idx) {
                global = Some(state);
            }
        }
        let interpreter = Interpreter {
            catalog,
            inputs: &self.inputs,
            global: global?,
        };
        // The global state is a closure fixpoint, so discoveries are ignored.
        let mut discoveries = GlobalDiscoveries::default();

        // Per-vertex reducers admit the states worth expanding, as in normal traversal, while
        // the entries keep each admitted state's predecessor. An entry that collects the
        // location is queued even when its reducer rejects it; popping it ends the search.
        let mut reducers = vec![LocalStateReducer::default(); self.graph.outgoing.len()];
        let start = RouteStep {
            edge: None,
            vertex_idx: self.graph.start_vertex_idx,
            state: LocalState::initialize(),
        };
        reducers[start.vertex_idx.0 as usize].insert(start.state.clone());
        let mut entries = vec![RouteEntry {
            step: start,
            parent_entry_idx: None,
        }];
        let mut queue = BinaryHeap::new();
        queue.push(Reverse((0_u32, 0_usize, false)));
        while let Some(Reverse((cost, entry_idx, collected))) = queue.pop() {
            if collected {
                let mut route = Vec::new();
                let mut next_entry_idx = Some(entry_idx);
                while let Some(idx) = next_entry_idx {
                    route.push(entries[idx].step.clone());
                    next_entry_idx = entries[idx].parent_entry_idx;
                }
                route.reverse();
                return Some(route);
            }
            let vertex_idx = entries[entry_idx].step.vertex_idx;
            for (outgoing_idx, edge) in self.graph.outgoing[vertex_idx.0 as usize]
                .iter()
                .enumerate()
            {
                let mut sources = LocalStateReducer::default();
                sources.insert(entries[entry_idx].step.state.clone());
                let mut outputs = LocalStateReducer::default();
                interpreter.evaluate_edge(edge, sources, &mut discoveries, &mut outputs);
                let mut collects = false;
                for effect in &edge.edge.effects {
                    if let Effect::CollectItem(idx) = *effect
                        && idx == location_idx
                    {
                        collects = true;
                    }
                }
                let destination = edge.edge.to_vertex_idx;
                for state in outputs.into_states() {
                    let retained = reducers[destination.0 as usize].insert(state.clone());
                    if retained || collects {
                        entries.push(RouteEntry {
                            step: RouteStep {
                                edge: Some(TraversalEdgeIndex {
                                    from_vertex_idx: vertex_idx,
                                    outgoing_idx,
                                }),
                                vertex_idx: destination,
                                state,
                            },
                            parent_entry_idx: Some(entry_idx),
                        });
                        queue.push(Reverse((cost + 1, entries.len() - 1, collects)));
                    }
                }
            }
        }
        None
    }
}
