use super::{RoomIndices, read_source, z3_json_data};
use anyhow::{Result, bail};
use logic_catalog::*;
use std::{collections::BTreeMap, path::Path};

pub fn build_connections(
    source_directory: &Path,
    room_indices: &BTreeMap<SourceRoomId, RoomIndices>,
    catalog: &mut LogicCatalog,
) -> Result<()> {
    for filename in ["entrances.json", "teleports.json", "whirlpools.json"] {
        let source: z3_json_data::Connections =
            read_source(&source_directory.join("connections").join(filename))?;
        for connection in source.connections {
            let entrance_kind = match &connection {
                z3_json_data::Connection::Door { .. } => Some(EntranceKind::Door),
                z3_json_data::Connection::Drop { .. } => Some(EntranceKind::Drop),
                _ => None,
            };
            let connection = match connection {
                z3_json_data::Connection::Door {
                    world,
                    overworld,
                    underworld,
                }
                | z3_json_data::Connection::Drop {
                    world,
                    overworld,
                    underworld,
                } => {
                    let outside = &room_indices[&SourceRoomId::Overworld(overworld.room_id)];
                    let inside = &room_indices[&SourceRoomId::Underworld(underworld.room_id)];
                    Connection::Entrance {
                        kind: entrance_kind.unwrap(),
                        entrance_idx: outside.entrances
                            [&(overworld.entrance_id, world.get_world())],
                        interior_vertex_idx: inside.vertices[&(underworld.node_id, None)],
                    }
                }
                z3_json_data::Connection::Teleport {
                    to_world,
                    underworld,
                    overworld,
                } => {
                    let inside = &room_indices[&SourceRoomId::Underworld(underworld.room_id)];
                    let outside = &room_indices[&SourceRoomId::Overworld(overworld.room_id)];
                    Connection::Teleport {
                        from_vertex_idx: inside.vertices[&(underworld.node_id, None)],
                        teleport_idx: outside.teleports
                            [&(overworld.teleport_id, to_world.get_world())],
                    }
                }
                z3_json_data::Connection::Whirlpool {
                    world,
                    overworld,
                    overworld2,
                } => {
                    let first = &room_indices[&SourceRoomId::Overworld(overworld.room_id)];
                    let second = &room_indices[&SourceRoomId::Overworld(overworld2.room_id)];
                    Connection::Whirlpool {
                        endpoints: [
                            first.whirlpools[&(overworld.whirlpool_id, world.get_world())],
                            second.whirlpools[&(overworld2.whirlpool_id, world.get_world())],
                        ],
                    }
                }
            };
            catalog.vanilla_connections.push(connection);
        }
    }

    // Positions use the 8x8 area grid (32 tiles per cell); sizes use
    // screens (16 tiles per screen). Compare spans in global tile coordinates.
    let mut geometry = Vec::new();
    for boundary in &catalog.screen_boundaries {
        let room = catalog.room(catalog.vertex(boundary.vertex_idx).room_idx);
        let [x, y] = room.position.unwrap();
        let [width, height] = room.size.unwrap();
        let x = f32::from(x) * 32.0;
        let y = f32::from(y) * 32.0;
        let width = f32::from(width) * 16.0;
        let height = f32::from(height) * 16.0;
        let (line, offset, opposite) = match boundary.direction {
            Direction::North => (y, x, Direction::South),
            Direction::South => (y + height, x, Direction::North),
            Direction::West => (x, y, Direction::East),
            Direction::East => (x + width, y, Direction::West),
        };
        geometry.push((
            line,
            [boundary.span[0] + offset, boundary.span[1] + offset],
            opposite,
        ));
    }
    let mut errors = Vec::new();
    for (i, boundary) in catalog.screen_boundaries.iter().enumerate() {
        let room_idx = catalog.vertex(boundary.vertex_idx).room_idx;
        let world = catalog.vertex_metadata(boundary.vertex_idx).world;
        let (line, span, opposite) = geometry[i];
        let mut matches = Vec::new();
        for (j, neighbor) in catalog.screen_boundaries.iter().enumerate() {
            if catalog.vertex(neighbor.vertex_idx).room_idx != room_idx
                && catalog.vertex_metadata(neighbor.vertex_idx).world == world
                && neighbor.direction == opposite
                && geometry[j].0 == line
                && geometry[j].1 == span
            {
                matches.push(j);
            }
        }
        if matches.len() != 1 {
            let room = catalog.room(room_idx);
            let node = room.node(catalog.vertex_metadata(boundary.vertex_idx).node_idx);
            errors.push(format!(
                "{} / {} ({world:?}): {:?} span {:?} has {} matching neighboring spans; expected exactly one",
                room.name, node.name, boundary.direction, boundary.span, matches.len(),
            ));
        }
        for j in matches {
            if i < j {
                catalog
                    .vanilla_connections
                    .push(Connection::ScreenBoundary {
                        endpoints: [ScreenBoundaryIndex(i as u32), ScreenBoundaryIndex(j as u32)],
                    });
            }
        }
    }
    if !errors.is_empty() {
        bail!("overworld boundary matching failed:\n{}", errors.join("\n"));
    }
    Ok(())
}
