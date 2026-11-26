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
        for entrance in source.entrances {
            let outside = &room_indices[&entrance.overworld.room_id];
            let inside = &room_indices[&entrance.underworld.room_id];
            let kind = match entrance.kind {
                z3_json_data::EntranceType::Door => EntranceKind::Door,
                z3_json_data::EntranceType::Drop => EntranceKind::Drop,
            };
            catalog.vanilla_connections.push(Connection::Entrance {
                kind,
                entrance_idx: outside.entrances
                    [&(entrance.overworld.entrance_id, entrance.world.get_world())],
                interior_vertex_idx: inside.vertices[&(entrance.underworld.node_id, None)],
            });
        }
        for teleport in source.teleports {
            let inside = &room_indices[&teleport.underworld.room_id];
            let outside = &room_indices[&teleport.overworld.room_id];
            catalog.vanilla_connections.push(Connection::Teleport {
                from_vertex_idx: inside.vertices[&(teleport.underworld.node_id, None)],
                teleport_idx: outside.teleports[&(
                    teleport.overworld.teleport_id,
                    teleport.to_world.get_world(),
                )],
            });
        }
        for whirlpool in source.whirlpools {
            let first = &room_indices[&whirlpool.overworld.room_id];
            let second = &room_indices[&whirlpool.overworld2.room_id];
            let world = whirlpool.world.get_world();
            catalog.vanilla_connections.push(Connection::Whirlpool {
                endpoints: [
                    first.whirlpools[&(whirlpool.overworld.whirlpool_id, world)],
                    second.whirlpools[&(whirlpool.overworld2.whirlpool_id, world)],
                ],
            });
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
