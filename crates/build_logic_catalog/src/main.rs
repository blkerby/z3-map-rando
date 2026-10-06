mod requirements;
mod source;

use anyhow::{Context, Result};
use clap::Parser;
use logic_catalog::*;
use requirements::{Compiler, compose_requirements};
use serde::de::DeserializeOwned;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(about = "Build the logic catalog from z3-json-data")]
struct Args {
    source_directory: PathBuf,
    output_catalog: PathBuf,
}

#[derive(Default)]
struct RoomIndices {
    vertices: BTreeMap<(u32, Option<World>), VertexIndex>,
    entrances: BTreeMap<(u32, World), EntranceIndex>,
    items: BTreeMap<u32, Effect>,
    doors: BTreeMap<u32, DoorIndex>,
    obstacles: BTreeMap<String, ObstacleIndex>,
}

fn read_source<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

fn collect_source_files(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_source_files(&path, paths)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            paths.push(path);
        }
    }
    Ok(())
}

fn collect_techs(techs: Vec<source::Tech>, catalog: &mut LogicCatalog, compiler: &mut Compiler) {
    for mut tech in techs {
        let extensions = std::mem::take(&mut tech.extension_techs);
        let tech_idx = TechIndex(catalog.techs.len() as u32);
        catalog.techs.push(TechDefinition {
            tech_id: tech.id,
            name: tech.name.clone(),
            tiers: std::mem::take(&mut tech.tiers),
        });
        compiler.techs.insert(tech.name.clone(), (tech_idx, tech));
        collect_techs(extensions, catalog, compiler);
    }
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

fn main() -> Result<()> {
    let args = Args::parse();
    let mut catalog = LogicCatalog {
        vertices: Vec::new(),
        vertex_metadata: Vec::new(),
        edges: Vec::new(),
        edge_metadata: Vec::new(),
        item_locations: Vec::new(),
        start_vertex_idx: VertexIndex(0),
        rooms: Vec::new(),
        items: Vec::new(),
        techs: Vec::new(),
        flags: Vec::new(),
        doors: Vec::new(),
        events: Vec::new(),
        entrances: Vec::new(),
        screen_boundaries: Vec::new(),
        teleports: Vec::new(),
        whirlpools: Vec::new(),
        flute_spots: Vec::new(),
        event_entries: Vec::new(),
        event_exits: Vec::new(),
    };
    let mut compiler = Compiler {
        items: BTreeMap::new(),
        flags: BTreeMap::new(),
        helpers: BTreeMap::new(),
        techs: BTreeMap::new(),
        damage: BTreeMap::new(),
    };
    let items: source::Items = read_source(&args.source_directory.join("items.json"))?;
    for category in [
        items.inventory,
        items.refills,
        items.bottle_contents,
        items.currency,
        items.dungeon_items,
        items.dungeon_prizes,
        items.goal_items,
        items.expansions,
    ] {
        for item in category {
            let item_idx = ItemIndex(catalog.items.len() as u32);
            let receipt_id = u8::from_str_radix(item.item_receipt_id.trim_start_matches("0x"), 16)?;
            let prize_patch_bytes = if let Some(source_bytes) = item.prize_patch_bytes {
                let mut bytes = [0; 6];
                for (i, byte) in source_bytes.iter().enumerate() {
                    bytes[i] = u8::from_str_radix(byte.trim_start_matches("0x"), 16)?;
                }
                Some(bytes)
            } else {
                None
            };
            compiler.items.insert(item.name.clone(), item_idx);
            catalog.items.push(ItemDefinition {
                name: item.name,
                receipt_id,
                prize_patch_bytes,
            });
        }
    }
    for name in items.flags {
        let flag_idx = FlagIndex(catalog.flags.len() as u32);
        compiler.flags.insert(name.clone(), flag_idx);
        catalog.flags.push(name);
    }
    let helpers: Vec<source::Helper> = read_source(&args.source_directory.join("helpers.json"))?;
    for helper in helpers {
        compiler.helpers.insert(helper.name.clone(), helper);
    }
    let techs: source::Techs = read_source(&args.source_directory.join("tech.json"))?;
    for category in techs.tech_categories {
        collect_techs(category.techs, &mut catalog, &mut compiler);
    }
    let mut enemy_paths = Vec::new();
    collect_source_files(&args.source_directory.join("enemies"), &mut enemy_paths)?;
    enemy_paths.sort();
    for path in enemy_paths {
        let enemies: source::Enemies = read_source(&path)?;
        for enemy in enemies.enemies {
            let damage = [
                enemy.dmg_to_link.green,
                enemy.dmg_to_link.blue,
                enemy.dmg_to_link.red,
            ];
            for name in enemy.names {
                compiler.damage.insert(name, damage);
            }
        }
    }

    let mut paths = Vec::new();
    collect_source_files(&args.source_directory.join("rooms"), &mut paths)?;
    paths.sort();
    let mut source_rooms: Vec<source::Room> = Vec::new();
    for path in paths {
        source_rooms.push(read_source(&path)?);
    }
    source_rooms.sort_by_key(|room| {
        let namespace = if room.room_type == RoomKind::Overworld {
            RoomNamespace::Overworld
        } else {
            RoomNamespace::Underworld
        };
        (namespace, room.id)
    });
    let mut event_indices = BTreeMap::new();
    let mut entry_vertices = BTreeMap::new();
    let mut exit_vertices = BTreeMap::new();
    for source_room in source_rooms {
        let room_idx = RoomIndex(catalog.rooms.len() as u32);
        let overworld = source_room.room_type == RoomKind::Overworld;
        let namespace = if overworld {
            RoomNamespace::Overworld
        } else {
            RoomNamespace::Underworld
        };
        let mut room = Room {
            room_id: SourceRoomId {
                namespace,
                id: source_room.id,
            },
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
        for node in &source_room.nodes {
            let node_idx = NodeIndex(room.nodes.len() as u32);
            room.nodes.push(NodeMetadata {
                node_id: node.id,
                name: node.name.clone(),
                node_type: node.node_type.map(source::SourceNodeType::get_node_type),
            });
            let worlds: &[Option<World>] = match node.world {
                Some(source::NodeWorld::Light) => &[Some(World::Light)],
                Some(source::NodeWorld::Dark) => &[Some(World::Dark)],
                Some(source::NodeWorld::Both) => &[Some(World::Light), Some(World::Dark)],
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
                        || transition.world.map(source::SourceWorld::get_world) == world
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
                        catalog.teleports.push(Teleport {
                            vertex_idx,
                            teleport_id: teleport.id,
                            name: teleport.name.clone(),
                        });
                    }
                }
                for whirlpool in &node.whirlpools {
                    if Some(whirlpool.world.get_world()) == world {
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
                source::Addresses::One(address) => std::slice::from_ref(address),
                source::Addresses::Many(addresses) => addresses.as_slice(),
            };
            let mut rom_addresses = Vec::new();
            for address in addresses {
                if address != "unknown" {
                    rom_addresses.push(u32::from_str_radix(address.trim_start_matches("0x"), 16)?);
                }
            }
            catalog.item_locations.push(ItemLocation {
                name: format!("{} - {}", source_room.name, item.location_name),
                vertex_idx: indices.vertices[&(
                    item.item_location,
                    item.world.map(source::SourceWorld::get_world),
                )],
                rom_addresses,
            });
        }
        for door in &source_room.locked_doors {
            let door_idx = DoorIndex(catalog.doors.len() as u32);
            indices.doors.insert(door.id, door_idx);
            let lock = match door.key_type {
                source::KeyType::Small => LockKind::SmallKey,
                source::KeyType::Big => LockKind::BigKey,
                source::KeyType::Bomb => LockKind::Bomb,
                source::KeyType::BombOrBoots => LockKind::BombOrBoots,
                source::KeyType::Boots => LockKind::Boots,
                source::KeyType::Glove => LockKind::Glove,
            };
            catalog.doors.push(Door {
                room_idx,
                door_id: door.id,
                name: door.location_name.clone(),
                world: door.world.map(source::SourceWorld::get_world),
                lock,
            });
        }
        for strat in source_room.strats {
            let strat_idx = StratIndex(room.strats.len() as u32);
            room.strats.push(StratMetadata { name: strat.name });
            let world_pairs: Vec<(Option<World>, Option<World>)> = if !overworld {
                vec![(None, None)]
            } else if let Some(from_world) = strat.from_world {
                vec![(
                    Some(from_world.get_world()),
                    strat.to_world.map(source::SourceWorld::get_world),
                )]
            } else {
                match strat.world.unwrap() {
                    source::StratWorld::Light => vec![(Some(World::Light), Some(World::Light))],
                    source::StratWorld::Dark => vec![(Some(World::Dark), Some(World::Dark))],
                    source::StratWorld::Any => {
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
            };
            let mut effects = Vec::new();
            for id in strat.collects_items {
                effects.push(indices.items[&id]);
            }
            for name in strat.sets_flags {
                effects.push(Effect::SetFlag(compiler.flags[&name]));
            }
            for name in strat.clears_obstacles {
                effects.push(Effect::ClearObstacle(indices.obstacles[&name]));
            }
            for name in strat.resets_obstacles {
                effects.push(Effect::ResetObstacle(indices.obstacles[&name]));
            }
            for id in strat.unlocks_door {
                effects.push(Effect::OpenDoor(indices.doors[&id]));
            }
            if let Some(follower) = strat.sets_follower {
                effects.push(Effect::SetFollower(follower.get_follower()));
            }
            if strat.follower_complete.is_some() {
                effects.push(Effect::SetFollower(None));
            }
            let requirement = compiler.compile_requirements(&strat.requires, &indices);
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
                            *event_indices.entry(event.name.clone()).or_insert_with(|| {
                                let idx = EventIndex(catalog.events.len() as u32);
                                catalog.events.push(event.name.clone());
                                idx
                            });
                        let entrance_idx = event
                            .entrance_id
                            .map(|id| indices.entrances[&(id, world.unwrap())]);
                        let vertex_idx = add_event_vertex(
                            &mut catalog,
                            if is_entry {
                                &mut entry_vertices
                            } else {
                                &mut exit_vertices
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
                    source::Bunny::Yes => Requirement::Never,
                    source::Bunny::No if from_world == Some(World::Dark) => {
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
        for node in &source_room.nodes {
            if matches!(node.world, Some(source::NodeWorld::Both)) {
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
        catalog.rooms.push(room);
    }
    let bytes = encode_catalog(&catalog)?;
    fs::write(&args.output_catalog, &bytes)
        .with_context(|| format!("failed to write {}", args.output_catalog.display()))?;
    println!(
        "Wrote {} rooms, {} vertices, {} edges, and {} item locations ({} bytes) to {}",
        catalog.rooms.len(),
        catalog.vertices.len(),
        catalog.edges.len(),
        catalog.item_locations.len(),
        bytes.len(),
        args.output_catalog.display()
    );
    Ok(())
}
