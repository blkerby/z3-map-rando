use super::{
    RoomIndices,
    requirements::{Compiler, compose_requirements},
};
use crate::z3_json_data::{self, collect_source_files, read_source};
use anyhow::{Context, Result};
use logic_catalog::*;
use std::{collections::BTreeMap, path::Path};

#[derive(Default)]
struct EventVertices {
    indices: BTreeMap<String, EventIndex>,
    entry_vertices: BTreeMap<(VertexIndex, EventIndex, Option<EntranceIndex>), VertexIndex>,
    exit_vertices: BTreeMap<(VertexIndex, EventIndex, Option<EntranceIndex>), VertexIndex>,
}

pub(super) fn compile_rooms(
    source_directory: &Path,
    catalog: &mut LogicCatalog,
    compiler: &Compiler,
) -> Result<BTreeMap<SourceRoomId, RoomIndices>> {
    let mut paths = Vec::new();
    collect_source_files(&source_directory.join("rooms"), &mut paths)?;
    paths.sort();
    let mut source_rooms: Vec<z3_json_data::Room> = Vec::new();
    for path in paths {
        source_rooms.push(read_source(&path)?);
    }
    source_rooms.sort_by_key(|room| {
        if room.room_type == RoomKind::Overworld {
            SourceRoomId::Overworld(room.id)
        } else {
            SourceRoomId::Underworld(room.id)
        }
    });
    let mut events = EventVertices::default();
    let mut room_indices = BTreeMap::new();
    for source_room in source_rooms {
        let room_id = if source_room.room_type == RoomKind::Overworld {
            SourceRoomId::Overworld(source_room.id)
        } else {
            SourceRoomId::Underworld(source_room.id)
        };
        let mut room = Room {
            room_id,
            name: source_room.name.clone(),
            kind: source_room.room_type,
            position: source_room.position,
            size: source_room.size,
            obstacles: Vec::new(),
            nodes: Vec::new(),
            strats: Vec::new(),
        };
        let mut indices = RoomIndices::default();
        for obstacle in &source_room.obstacles {
            let obstacle_idx = ObstacleIndex(room.obstacles.len() as u32);
            indices.obstacles.insert(obstacle.id.clone(), obstacle_idx);
            room.obstacles.push(Obstacle {
                name: obstacle.id.clone(),
            });
        }
        compile_room_nodes(&source_room, &mut room, &mut indices, catalog);
        compile_room_items(&source_room, room_id, &mut indices, catalog, compiler)?;
        compile_room_doors(&source_room, &mut indices, catalog);
        compile_room_strats(
            &source_room,
            &mut room,
            &indices,
            catalog,
            compiler,
            &mut events,
        );
        for node in &source_room.nodes {
            if matches!(node.world, Some(z3_json_data::NodeWorld::Both)) {
                catalog.edges.push(Edge {
                    from_vertex_idx: indices.vertices[&(node.id, Some(World::Dark))],
                    to_vertex_idx: indices.vertices[&(node.id, Some(World::Light))],
                    requirement: Requirement::Item(compiler.items["MagicMirror"]),
                    effects: Box::new([]),
                });
                catalog.edge_metadata.push(EdgeMetadata {
                    origin: EdgeOrigin::ImplicitMirror,
                });
            }
        }
        room_indices.insert(room.room_id, indices);
        catalog.rooms.push(room);
    }
    Ok(room_indices)
}

fn compile_room_nodes(
    source_room: &z3_json_data::Room,
    room: &mut Room,
    indices: &mut RoomIndices,
    catalog: &mut LogicCatalog,
) {
    let room_idx = RoomIndex(catalog.rooms.len() as u32);
    for node in &source_room.nodes {
        let node_idx = NodeIndex(room.nodes.len() as u32);
        room.nodes.push(NodeMetadata {
            node_id: node.id,
            name: node.name.clone(),
            node_type: node
                .node_type
                .map(z3_json_data::SourceNodeType::get_node_type),
        });
        let worlds: &[Option<World>] = match node.world {
            Some(z3_json_data::NodeWorld::Light) => &[Some(World::Light)],
            Some(z3_json_data::NodeWorld::Dark) => &[Some(World::Dark)],
            Some(z3_json_data::NodeWorld::Both) => &[Some(World::Light), Some(World::Dark)],
            None => &[None],
        };
        for &world in worlds {
            let vertex_idx = VertexIndex(catalog.vertices.len() as u32);
            indices.vertices.insert((node.id, world), vertex_idx);
            catalog.vertices.push(Vertex { room_idx });
            catalog.vertex_metadata.push(VertexMetadata {
                node_idx,
                world,
                origin: VertexOrigin::Node,
            });
            if node.spawn_point.as_deref() == Some("house") {
                catalog.start_vertex_idx = vertex_idx;
            }
            for entrance in &node.entrances {
                if Some(entrance.world.get_world()) == world {
                    let entrance_idx = EntranceIndex(catalog.entrances.len() as u32);
                    indices
                        .entrances
                        .insert((entrance.id, world.unwrap()), entrance_idx);
                    catalog.entrances.push(Entrance {
                        vertex_idx,
                        entrance_id: entrance.id,
                        name: entrance.name.clone(),
                    });
                }
            }
            for transition in &node.transitions {
                if transition.world.is_none()
                    || transition.world.map(z3_json_data::SourceWorld::get_world) == world
                {
                    catalog.screen_boundaries.push(ScreenBoundary {
                        vertex_idx,
                        direction: transition.edge.get_direction(),
                        span: transition.span,
                        terrain: transition.terrain.clone(),
                    });
                }
            }
            for teleport in &node.teleports {
                if Some(teleport.world.get_world()) == world {
                    indices.teleports.insert(
                        (teleport.id, world.unwrap()),
                        TeleportIndex(catalog.teleports.len() as u32),
                    );
                    catalog.teleports.push(Teleport {
                        vertex_idx,
                        teleport_id: teleport.id,
                        name: teleport.name.clone(),
                    });
                }
            }
            for whirlpool in &node.whirlpools {
                if Some(whirlpool.world.get_world()) == world {
                    indices.whirlpools.insert(
                        (whirlpool.id, world.unwrap()),
                        WhirlpoolIndex(catalog.whirlpools.len() as u32),
                    );
                    catalog.whirlpools.push(Whirlpool {
                        vertex_idx,
                        whirlpool_id: whirlpool.id,
                        name: whirlpool.name.clone(),
                    });
                }
            }
            if let Some(location) = node.flute_location {
                if world == Some(World::Light) {
                    catalog.flute_spots.push(FluteSpot {
                        vertex_idx,
                        location,
                    });
                }
            }
        }
    }
}

fn compile_room_items(
    source_room: &z3_json_data::Room,
    room_id: SourceRoomId,
    indices: &mut RoomIndices,
    catalog: &mut LogicCatalog,
    compiler: &Compiler,
) -> Result<()> {
    for item in &source_room.items {
        if item.item == "OcarinaActive" {
            indices
                .items
                .insert(item.id, Effect::GrantItem(compiler.items[&item.item]));
            continue;
        }
        let item_location_idx = ItemLocationIndex(catalog.item_locations.len() as u32);
        indices
            .items
            .insert(item.id, Effect::CollectItem(item_location_idx));
        let addresses = match &item.item_address {
            z3_json_data::Addresses::One(address) => std::slice::from_ref(address),
            z3_json_data::Addresses::Many(addresses) => addresses.as_slice(),
        };
        let mut rom_addresses = Vec::new();
        for address in addresses {
            if address != "unknown" {
                rom_addresses.push(u32::from_str_radix(address.trim_start_matches("0x"), 16)?);
            }
        }
        let world = item.world.map(z3_json_data::SourceWorld::get_world);
        catalog.item_locations.push(ItemLocation {
            name: format!("{} - {}", source_room.name, item.location_name),
            vertex_idx: *indices
                .vertices
                .get(&(item.item_location, world))
                .with_context(|| {
                    format!(
                        "missing item-location vertex in room {:?} {:?}, item {} {:?}: node {}, world {:?}",
                        room_id,
                        source_room.name,
                        item.id,
                        item.location_name,
                        item.item_location,
                        world,
                    )
                })?,
            rom_addresses,
        });
    }
    Ok(())
}

fn compile_room_doors(
    source_room: &z3_json_data::Room,
    indices: &mut RoomIndices,
    catalog: &mut LogicCatalog,
) {
    let room_idx = RoomIndex(catalog.rooms.len() as u32);
    for door in &source_room.locked_doors {
        let door_idx = DoorIndex(catalog.doors.len() as u32);
        indices.doors.insert(door.id, door_idx);
        let lock = match door.key_type {
            z3_json_data::KeyType::Small => LockKind::SmallKey,
            z3_json_data::KeyType::Big => LockKind::BigKey,
            z3_json_data::KeyType::Bomb => LockKind::Bomb,
            z3_json_data::KeyType::BombOrBoots => LockKind::BombOrBoots,
            z3_json_data::KeyType::Boots => LockKind::Boots,
            z3_json_data::KeyType::Glove => LockKind::Glove,
        };
        catalog.doors.push(Door {
            room_idx,
            door_id: door.id,
            name: door.location_name.clone(),
            world: door.world.map(z3_json_data::SourceWorld::get_world),
            lock,
        });
    }
}

fn compile_room_strats(
    source_room: &z3_json_data::Room,
    room: &mut Room,
    indices: &RoomIndices,
    catalog: &mut LogicCatalog,
    compiler: &Compiler,
    events: &mut EventVertices,
) {
    let room_idx = RoomIndex(catalog.rooms.len() as u32);
    let overworld = source_room.room_type == RoomKind::Overworld;
    for strat in &source_room.strats {
        let strat_idx = StratIndex(room.strats.len() as u32);
        room.strats.push(StratMetadata {
            name: strat.name.clone(),
        });
        let world_pairs = get_strat_world_pairs(strat, overworld, indices);
        let effects = collect_strat_effects(strat, indices, compiler);
        let requirement = compiler.compile_requirements(&strat.requires, indices);
        for (from_world, to_world) in world_pairs {
            let ordinary_from = indices.vertices[&(strat.link[0], from_world)];
            let ordinary_to = indices.vertices[&(strat.link[1], to_world)];
            let mut from_vertex_idx = ordinary_from;
            let mut to_vertex_idx = ordinary_to;
            for (event, ordinary_idx, world, is_entry) in [
                (&strat.entrance_state, ordinary_from, from_world, true),
                (&strat.exit_state, ordinary_to, to_world, false),
            ] {
                if let Some(event) = event {
                    let event_idx =
                        *events.indices.entry(event.name.clone()).or_insert_with(|| {
                            let idx = EventIndex(catalog.events.len() as u32);
                            catalog.events.push(event.name.clone());
                            idx
                        });
                    let entrance_idx = event
                        .entrance_id
                        .map(|id| indices.entrances[&(id, world.unwrap())]);
                    let vertex_idx = add_event_vertex(
                        catalog,
                        if is_entry {
                            &mut events.entry_vertices
                        } else {
                            &mut events.exit_vertices
                        },
                        ordinary_idx,
                        event_idx,
                        entrance_idx,
                        is_entry,
                    );
                    if is_entry {
                        from_vertex_idx = vertex_idx;
                    } else {
                        to_vertex_idx = vertex_idx;
                    }
                }
            }
            let bunny_requirement = match strat.is_bunny {
                z3_json_data::Bunny::Yes => Requirement::Never,
                z3_json_data::Bunny::No if from_world == Some(World::Dark) => {
                    Requirement::Item(compiler.items["MoonPearl"])
                }
                _ => Requirement::Free,
            };
            catalog.edges.push(Edge {
                from_vertex_idx,
                to_vertex_idx,
                requirement: compose_requirements(vec![bunny_requirement, requirement.clone()]),
                effects: effects.clone().into_boxed_slice(),
            });
            catalog.edge_metadata.push(EdgeMetadata {
                origin: EdgeOrigin::Strat {
                    room_idx,
                    strat_idx,
                },
            });
        }
    }
}

fn get_strat_world_pairs(
    strat: &z3_json_data::Strat,
    overworld: bool,
    indices: &RoomIndices,
) -> Vec<(Option<World>, Option<World>)> {
    if !overworld {
        vec![(None, None)]
    } else if let Some(from_world) = strat.from_world {
        vec![(
            Some(from_world.get_world()),
            strat.to_world.map(z3_json_data::SourceWorld::get_world),
        )]
    } else {
        match strat.world.unwrap() {
            z3_json_data::StratWorld::Light => {
                vec![(Some(World::Light), Some(World::Light))]
            }
            z3_json_data::StratWorld::Dark => vec![(Some(World::Dark), Some(World::Dark))],
            z3_json_data::StratWorld::Any => {
                let mut pairs = Vec::new();
                for world in [Some(World::Light), Some(World::Dark)] {
                    if indices.vertices.contains_key(&(strat.link[0], world))
                        && indices.vertices.contains_key(&(strat.link[1], world))
                    {
                        pairs.push((world, world));
                    }
                }
                pairs
            }
        }
    }
}

fn collect_strat_effects(
    strat: &z3_json_data::Strat,
    indices: &RoomIndices,
    compiler: &Compiler,
) -> Vec<Effect> {
    let mut effects = Vec::new();
    for id in &strat.collects_items {
        effects.push(indices.items[id]);
    }
    for name in &strat.sets_flags {
        effects.push(Effect::SetFlag(compiler.flags[name]));
    }
    for name in &strat.clears_obstacles {
        effects.push(Effect::ClearObstacle(indices.obstacles[name]));
    }
    for name in &strat.resets_obstacles {
        effects.push(Effect::ResetObstacle(indices.obstacles[name]));
    }
    for id in &strat.unlocks_door {
        effects.push(Effect::OpenDoor(indices.doors[id]));
    }
    if let Some(follower) = strat.sets_follower {
        effects.push(Effect::SetFollower(follower.get_follower()));
    }
    if strat.follower_complete.is_some() {
        effects.push(Effect::SetFollower(None));
    }
    effects
}

fn add_event_vertex(
    catalog: &mut LogicCatalog,
    vertices: &mut BTreeMap<(VertexIndex, EventIndex, Option<EntranceIndex>), VertexIndex>,
    ordinary_idx: VertexIndex,
    event_idx: EventIndex,
    entrance_idx: Option<EntranceIndex>,
    is_entry: bool,
) -> VertexIndex {
    let key = (ordinary_idx, event_idx, entrance_idx);
    if let Some(vertex_idx) = vertices.get(&key) {
        return *vertex_idx;
    }
    let vertex_idx = VertexIndex(catalog.vertices.len() as u32);
    let mut metadata = *catalog.vertex_metadata(ordinary_idx);
    metadata.origin = if is_entry {
        VertexOrigin::EventEntry { event_idx }
    } else {
        VertexOrigin::EventExit { event_idx }
    };
    catalog.vertices.push(*catalog.vertex(ordinary_idx));
    catalog.vertex_metadata.push(metadata);
    let endpoint = EventEndpoint {
        vertex_idx,
        event_idx,
        entrance_idx,
    };
    if is_entry {
        catalog.event_entries.push(endpoint);
    } else {
        catalog.event_exits.push(endpoint);
    }
    vertices.insert(key, vertex_idx);
    vertex_idx
}
