//! Pool selection and variant-resolved overworld connections.

use std::collections::BTreeMap;

use anyhow::{Result, ensure};
use common_types::SourceRoomId;
use logic_catalog::{
    BoundaryEndpoint, Connection, Direction, LogicCatalog, MapBoundaryKind, Room, VertexIndex,
    World,
};
use rand::Rng;
use seed_data::{BorderSide, MapBorder, OverworldRoomPlacement, WorldLayout};

pub struct MapPool<'a> {
    records: &'a [u8],
    room_indices: [usize; 40],
}

impl<'a> MapPool<'a> {
    pub fn decode(bytes: &'a [u8], catalog: &LogicCatalog) -> Result<Self> {
        ensure!(
            bytes.len() >= 124 && &bytes[..8] == b"Z3MAPPL1",
            "invalid map-pool header"
        );
        let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        ensure!(
            count > 0 && (bytes.len() - 124) % 40 == 0 && (bytes.len() - 124) / 40 == count,
            "invalid map-pool length"
        );
        let mut room_indices = [0; 40];
        for index in 0..40 {
            let offset = 44 + index * 2;
            let id = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
            let room_index = catalog
                .rooms
                .iter()
                .position(|room| room.room_id == Some(SourceRoomId::Overworld(u32::from(id))))
                .ok_or_else(|| {
                    anyhow::anyhow!("map-pool room {id} is absent from the logic catalog")
                })?;
            ensure!(
                !room_indices[..index].contains(&room_index),
                "duplicate map-pool room {id}"
            );
            room_indices[index] = room_index;
        }
        Ok(Self {
            records: &bytes[124..],
            room_indices,
        })
    }

    pub fn select_rooms(
        &self,
        catalog: &LogicCatalog,
        theme: &str,
        rng: &mut impl Rng,
    ) -> Result<Vec<OverworldRoomPlacement>> {
        let index = rng.random_range(0..self.records.len() / 40);
        let record = &self.records[index * 40..index * 40 + 40];
        log::info!("Selected pool map {index}");
        let mut rooms = Vec::new();
        let mut occupied = [false; 64];
        for (&room_index, &position) in self.room_indices.iter().zip(record) {
            let placement = create_room_placement(&catalog.rooms[room_index], position, theme);
            let size = placement.size;
            let x = position % 8;
            let y = position / 8;
            ensure!(
                x + size[0] <= 8 && y + size[1] <= 8,
                "map-pool room is outside the board"
            );
            for dy in 0..size[1] {
                for dx in 0..size[0] {
                    let cell = usize::from(position + dx + dy * 8);
                    ensure!(!occupied[cell], "map-pool rooms overlap");
                    occupied[cell] = true;
                }
            }
            rooms.push(placement);
        }
        ensure!(
            occupied.into_iter().all(|cell| cell),
            "map-pool layout has empty cells"
        );
        Ok(rooms)
    }
}

/// Place every overworld room at its vanilla position.
pub fn build_vanilla_rooms(catalog: &LogicCatalog, theme: &str) -> Vec<OverworldRoomPlacement> {
    let mut rooms = Vec::new();
    for room in &catalog.rooms {
        if let (Some(SourceRoomId::Overworld(_)), Some([x, y])) = (room.room_id, room.position) {
            rooms.push(create_room_placement(room, y * 8 + x, theme));
        }
    }
    rooms
}

pub(crate) fn create_room_placement(
    room: &Room,
    position: u8,
    theme: &str,
) -> OverworldRoomPlacement {
    let Some(SourceRoomId::Overworld(room_id)) = room.room_id else {
        unreachable!()
    };
    let [source_x, source_y] = room.position.unwrap();
    let [width, height] = room.size.unwrap();
    OverworldRoomPlacement {
        room_id,
        vanilla_map_id: source_y * 8 + source_x,
        position,
        size: [(width / 2) as u8, (height / 2) as u8],
        theme: theme.to_owned(),
    }
}

/// Two different layout rooms touching across the east or south side of the
/// cell at `position`, with each room's edge variants for that segment.
struct RoomContact<'a> {
    position: [u8; 2],
    side: BorderSide,
    variants: [&'a BTreeMap<String, Vec<BoundaryEndpoint>>; 2],
}

fn find_room_contacts<'a>(catalog: &'a LogicCatalog, layout: &WorldLayout) -> Vec<RoomContact<'a>> {
    let mut cells = [0; 64];
    let mut catalog_rooms = Vec::new();
    for (index, room) in layout.rooms.iter().enumerate() {
        catalog_rooms.push(
            catalog
                .rooms
                .iter()
                .position(|source| source.room_id == Some(SourceRoomId::Overworld(room.room_id)))
                .unwrap(),
        );
        for dy in 0..room.size[1] {
            for dx in 0..room.size[0] {
                cells[usize::from(room.position + dx + dy * 8)] = index;
            }
        }
    }
    let mut contacts = Vec::new();
    for cell in 0..64u8 {
        let first_index = cells[usize::from(cell)];
        let first = &layout.rooms[first_index];
        for (delta, side, opposite, border_side) in
            [(1, 2, 3, BorderSide::East), (8, 1, 0, BorderSide::South)]
        {
            if (delta == 1 && cell % 8 == 7) || (delta == 8 && cell / 8 == 7) {
                continue;
            }
            let next = cell + delta;
            let second_index = cells[usize::from(next)];
            if first_index == second_index {
                continue;
            }
            let second = &layout.rooms[second_index];
            let first_segment = if delta == 1 {
                cell / 8 - first.position / 8
            } else {
                cell % 8 - first.position % 8
            };
            let second_segment = if delta == 1 {
                next / 8 - second.position / 8
            } else {
                next % 8 - second.position % 8
            };
            contacts.push(RoomContact {
                position: [cell % 8, cell / 8],
                side: border_side,
                variants: [
                    &catalog.rooms[catalog_rooms[first_index]].edge_variants[side]
                        [usize::from(first_segment)],
                    &catalog.rooms[catalog_rooms[second_index]].edge_variants[opposite]
                        [usize::from(second_segment)],
                ],
            });
        }
    }
    contacts
}

/// Choose a random shared edge variant for each room contact of a pool layout.
pub fn select_borders(
    catalog: &LogicCatalog,
    layout: &mut WorldLayout,
    rng: &mut impl Rng,
) -> Result<()> {
    let contacts = find_room_contacts(catalog, layout);
    layout.borders.clear();
    for contact in contacts {
        let [first_variants, second_variants] = contact.variants;
        let mut choices = Vec::new();
        for name in first_variants.keys() {
            if name != "closed" && second_variants.contains_key(name) {
                choices.push(name);
            }
        }
        if choices.is_empty() {
            ensure!(
                first_variants.contains_key("closed") && second_variants.contains_key("closed"),
                "pool contact has no shared edge variant"
            );
            continue;
        }
        layout.borders.push(MapBorder {
            position: contact.position,
            side: contact.side,
            variant: choices[rng.random_range(0..choices.len())].clone(),
        });
    }
    Ok(())
}

/// Build the traversal connections of a layout whose borders are already chosen.
pub fn build_connections(catalog: &LogicCatalog, layout: &WorldLayout) -> Vec<Connection> {
    if layout.vanilla {
        return catalog.vanilla_connections.clone();
    }
    let mut connections = Vec::new();
    for connection in &catalog.vanilla_connections {
        if !matches!(connection, Connection::ScreenBoundary { .. }) {
            connections.push(connection.clone());
        }
    }
    for contact in find_room_contacts(catalog, layout) {
        let mut name = "closed";
        for border in &layout.borders {
            if border.position == contact.position && border.side == contact.side {
                name = &border.variant;
            }
        }
        let a = &contact.variants[0][name];
        let b = &contact.variants[1][name];
        // Directions of travel from the first room to the second, and back.
        let (forward, backward) = match contact.side {
            BorderSide::East => (Direction::East, Direction::West),
            BorderSide::South => (Direction::South, Direction::North),
        };
        if name == "light-and-mirror" {
            if let (Some(from), Some(to)) = (
                get_vertex(&a[0], World::Light),
                get_vertex(&b[0], World::Light),
            ) {
                connections.push(Connection::MapBoundary {
                    endpoints: [from, to],
                    kind: MapBoundaryKind::Ordinary,
                    direction: forward,
                });
            }
            for (source, target, direction) in [(a, b, forward), (b, a, backward)] {
                if let (Some(from), Some(to)) = (
                    get_vertex(&source[1], World::Dark),
                    get_vertex(&target[2], World::Light),
                ) {
                    connections.push(Connection::MapBoundary {
                        endpoints: [from, to],
                        kind: MapBoundaryKind::Mirror,
                        direction,
                    });
                }
            }
            continue;
        }
        for (a, b) in a.iter().zip(b) {
            for world in [World::Light, World::Dark] {
                if name == "light-only" && world == World::Dark {
                    continue;
                }
                match (get_vertex(a, world), get_vertex(b, world)) {
                    (Some(from), Some(to)) => connections.push(Connection::MapBoundary {
                        endpoints: [from, to],
                        kind: MapBoundaryKind::Ordinary,
                        direction: forward,
                    }),
                    (Some(vertex), None) if matches!(b, BoundaryEndpoint::Return) => connections
                        .push(Connection::MapBoundary {
                            endpoints: [vertex, vertex],
                            kind: MapBoundaryKind::Return,
                            direction: forward,
                        }),
                    (None, Some(vertex)) if matches!(a, BoundaryEndpoint::Return) => connections
                        .push(Connection::MapBoundary {
                            endpoints: [vertex, vertex],
                            kind: MapBoundaryKind::Return,
                            direction: backward,
                        }),
                    _ => {}
                }
            }
        }
    }
    connections
}

fn get_vertex(endpoint: &BoundaryEndpoint, world: World) -> Option<VertexIndex> {
    match endpoint {
        BoundaryEndpoint::Node { light, dark } => {
            if world == World::Light {
                *light
            } else {
                *dark
            }
        }
        _ => None,
    }
}
