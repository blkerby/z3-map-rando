//! Replay a saved seed's logical traversal to group its item locations into spheres.

use crate::{
    graph::build_graph,
    inputs::prepare_inputs,
    local_state_reducer::LocalStateReducer,
    map::{build_connections, create_room_placement},
    state::{GlobalState, LocalState},
    traversal::compute_closure,
};
use anyhow::{Context, Result, bail};
use common_types::{Item, ItemLocationId, SourceRoomId};
use hashbrown::HashSet;
use logic_catalog::{ItemLocationIndex, LogicCatalog};
use seed_data::{SeedData, WorldLayout};
use std::collections::BTreeMap;

pub struct Spheres {
    /// Item locations first collectible in each sphere, in catalog order.
    pub spheres: Vec<Vec<ItemLocationIndex>>,
    /// Item locations that the current logic never reaches.
    pub unreachable: Vec<ItemLocationIndex>,
    /// The seed's item at each location, indexed by `ItemLocationIndex`.
    pub items: Vec<Option<Item>>,
}

/// Replay the seed against the current logic catalog. Sphere 1 holds the locations
/// collectible with the starting inventory, and each later sphere holds the locations newly
/// collectible with the items of all earlier spheres.
pub fn replay_spheres(catalog: &LogicCatalog, seed: &SeedData) -> Result<Spheres> {
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
    let graph = build_graph(catalog, &build_connections(catalog, &layout));

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
    Ok(Spheres {
        spheres,
        unreachable,
        items: seed_placements,
    })
}
