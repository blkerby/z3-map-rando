//! Shared world topology for the preliminary key pass and normal traversal.

use crate::{interpreter::collect_requirement_dependencies, state::GlobalDependency};
use common_types::allows_frog_dwarf;
use hashbrown::{HashMap, HashSet};
use logic_catalog::{
    Connection, Edge, EdgeIndex, Effect, EntranceIndex, EntranceKind, EventEndpoint, Follower,
    FollowerMask, LogicCatalog, NodeType, Requirement, RoomKind, UniqueItem, VertexIndex, World,
};

#[derive(Debug)]
pub struct TraversalGraph {
    /// Catalog vertices only; outgoing edges retain catalog/connection order.
    pub outgoing: Vec<Vec<TraversalEdge>>,
    pub start_vertex_idx: VertexIndex,
    /// Reverse global-read index over the final adjacency, shared across candidates.
    pub(crate) dependencies: HashMap<GlobalDependency, Vec<TraversalEdgeIndex>>,
}

/// An edge's position in the expanded outgoing adjacency, independent of catalog IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TraversalEdgeIndex {
    pub from_vertex_idx: VertexIndex,
    pub outgoing_idx: usize,
}

#[derive(Debug)]
pub struct TraversalEdge {
    pub edge: Edge,
    pub origin: TraversalEdgeOrigin,
    pub world: World,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraversalEdgeOrigin {
    Catalog(EdgeIndex),
    /// Index into the selected connection slice, not necessarily vanilla connections.
    Connection(usize),
    /// Return to the start with follower loss, obstacle reset, and full health
    /// expressed through requirements. Other resource usage is preserved.
    SaveAndQuit,
}

#[derive(Clone, Copy)]
struct ConnectionEndpoint {
    vertex_idx: VertexIndex,
    entrance_idx: Option<EntranceIndex>,
}

impl ConnectionEndpoint {
    fn match_event(&self, catalog: &LogicCatalog, event: &EventEndpoint) -> bool {
        if self.entrance_idx != event.entrance_idx {
            return false;
        }
        // Entrance indices identify overworld endpoints. Interior endpoints are
        // identified by the owning room and node, not the extra event vertex index.
        self.entrance_idx.is_some()
            || (catalog.vertex(self.vertex_idx).room_idx
                == catalog.vertex(event.vertex_idx).room_idx
                && catalog.vertex_metadata(self.vertex_idx).node_idx
                    == catalog.vertex_metadata(event.vertex_idx).node_idx)
    }
}

impl TraversalGraph {
    fn add_edge(
        &mut self,
        catalog: &LogicCatalog,
        interior_worlds: &[Option<World>],
        mut edge: Edge,
        origin: TraversalEdgeOrigin,
    ) {
        let source_room_idx = catalog.vertex(edge.from_vertex_idx).room_idx;
        let world = catalog
            .vertex_metadata(edge.from_vertex_idx)
            .world
            .or(interior_worlds[source_room_idx.0 as usize])
            .unwrap_or(World::Light);
        let destination_room_idx = catalog.vertex(edge.to_vertex_idx).room_idx;
        if origin == TraversalEdgeOrigin::SaveAndQuit || source_room_idx != destination_room_idx {
            let reset = Requirement::ResetObstacles(destination_room_idx);
            edge.requirement = Requirement::make_and(vec![edge.requirement, reset]);
        }
        self.outgoing[edge.from_vertex_idx.0 as usize].push(TraversalEdge {
            edge,
            origin,
            world,
        });
    }
}

/// Expand a selected layout without changing catalog vertices, edges, or connections.
/// Pass `catalog.vanilla_connections` for the current vanilla world layout.
pub fn build_graph(catalog: &LogicCatalog, connections: &[Connection]) -> TraversalGraph {
    let mut graph = TraversalGraph {
        outgoing: Vec::new(),
        start_vertex_idx: catalog.start_vertex_idx,
        dependencies: HashMap::new(),
    };
    for _ in &catalog.vertices {
        graph.outgoing.push(Vec::new());
    }

    let mut interior_worlds = vec![None; catalog.rooms.len()];
    for connection in connections {
        if let Connection::Entrance {
            entrance_idx,
            interior_vertex_idx,
            ..
        } = connection
        {
            let room_idx = catalog.vertex(*interior_vertex_idx).room_idx;
            let outside_vertex_idx = catalog.entrance(*entrance_idx).vertex_idx;
            interior_worlds[room_idx.0 as usize] =
                catalog.vertex_metadata(outside_vertex_idx).world;
        }
    }

    // Start with the edges in the catalog (room-local edges and flute travel).
    for (index, edge) in catalog.edges.iter().enumerate() {
        graph.add_edge(
            catalog,
            &interior_worlds,
            edge.clone(),
            TraversalEdgeOrigin::Catalog(EdgeIndex(index as u32)),
        );
    }

    // Now add edges for cross-room connections.
    for (connection_idx, connection) in connections.iter().enumerate() {
        let (first, second, bidirectional, requirement) = match connection {
            Connection::Entrance {
                kind,
                entrance_idx,
                interior_vertex_idx,
            } => {
                let requirement = match kind {
                    EntranceKind::Door => {
                        let mut lost = FollowerMask::make([
                            Follower::OldMan,
                            Follower::Blind,
                            Follower::PurpleChest,
                            Follower::SuperBomb,
                        ]);
                        // Discard followers that cannot enter; they can be lost whenever needed.
                        if catalog
                            .entrance(*entrance_idx)
                            .rom_entrance_id
                            .is_some_and(|id| !allows_frog_dwarf(id))
                        {
                            lost.0 |= 1 << Follower::Dwarf as u8;
                        }
                        Requirement::LoseFollowers(lost)
                    }
                    EntranceKind::Drop => {
                        Requirement::LoseFollowers(FollowerMask::make([Follower::SuperBomb]))
                    }
                };
                let requirement = if catalog
                    .room(catalog.vertex(*interior_vertex_idx).room_idx)
                    .kind
                    == RoomKind::Dungeon
                {
                    // FIXME: Entering a dungeon temporarily requires IsNoBunny,
                    // because bunny logic is not yet complete in dungeons.
                    Requirement::make_and(vec![Requirement::IsNotBunny, requirement])
                } else {
                    requirement
                };
                (
                    ConnectionEndpoint {
                        vertex_idx: catalog.entrance(*entrance_idx).vertex_idx,
                        entrance_idx: Some(*entrance_idx),
                    },
                    ConnectionEndpoint {
                        vertex_idx: *interior_vertex_idx,
                        entrance_idx: None,
                    },
                    matches!(kind, EntranceKind::Door),
                    requirement,
                )
            }
            Connection::Teleport {
                from_vertex_idx,
                teleport_idx,
            } => (
                ConnectionEndpoint {
                    vertex_idx: *from_vertex_idx,
                    entrance_idx: None,
                },
                ConnectionEndpoint {
                    vertex_idx: catalog.teleports[teleport_idx.0 as usize].vertex_idx,
                    entrance_idx: None,
                },
                false,
                // Houlihan's door uses ordinary exit loading; the Agahnim event warps do not.
                if catalog
                    .room(catalog.vertex(*from_vertex_idx).room_idx)
                    .node(catalog.vertex_metadata(*from_vertex_idx).node_idx)
                    .node_type
                    == Some(NodeType::Door)
                {
                    Requirement::LoseFollowers(FollowerMask::make([Follower::Blind]))
                } else {
                    Requirement::Free
                },
            ),
            Connection::Whirlpool { endpoints } => (
                ConnectionEndpoint {
                    vertex_idx: catalog.whirlpools[endpoints[0].0 as usize].vertex_idx,
                    entrance_idx: None,
                },
                ConnectionEndpoint {
                    vertex_idx: catalog.whirlpools[endpoints[1].0 as usize].vertex_idx,
                    entrance_idx: None,
                },
                true,
                Requirement::Free,
            ),
            Connection::ScreenBoundary { endpoints } => (
                ConnectionEndpoint {
                    vertex_idx: catalog.screen_boundaries[endpoints[0].0 as usize].vertex_idx,
                    entrance_idx: None,
                },
                ConnectionEndpoint {
                    vertex_idx: catalog.screen_boundaries[endpoints[1].0 as usize].vertex_idx,
                    entrance_idx: None,
                },
                true,
                Requirement::Free,
            ),
        };
        let reverse_requirement = if matches!(
            connection,
            Connection::Entrance {
                kind: EntranceKind::Door,
                ..
            }
        ) {
            Requirement::LoseFollowers(FollowerMask::make([Follower::Blind]))
        } else {
            Requirement::Free
        };
        let directions = [
            (first, second, requirement),
            (second, first, reverse_requirement),
        ];
        for (direction, (from, to, requirement)) in directions.into_iter().enumerate() {
            if direction == 1 && !bidirectional {
                continue;
            }
            graph.add_edge(
                catalog,
                &interior_worlds,
                Edge {
                    from_vertex_idx: from.vertex_idx,
                    to_vertex_idx: to.vertex_idx,
                    requirement: requirement.clone(),
                    effects: Box::new([]),
                },
                TraversalEdgeOrigin::Connection(connection_idx),
            );
            if !matches!(connection, Connection::Entrance { .. }) {
                continue;
            }
            for exit in &catalog.event_exits {
                if !from.match_event(catalog, exit) {
                    continue;
                }
                for entry in &catalog.event_entries {
                    if exit.event_idx == entry.event_idx && to.match_event(catalog, entry) {
                        graph.add_edge(
                            catalog,
                            &interior_worlds,
                            Edge {
                                from_vertex_idx: exit.vertex_idx,
                                to_vertex_idx: entry.vertex_idx,
                                requirement: requirement.clone(),
                                effects: Box::new([]),
                            },
                            TraversalEdgeOrigin::Connection(connection_idx),
                        );
                    }
                }
            }
        }
    }

    for index in 0..catalog.vertices.len() {
        graph.add_edge(
            catalog,
            &interior_worlds,
            Edge {
                from_vertex_idx: VertexIndex(index as u32),
                to_vertex_idx: catalog.start_vertex_idx,
                requirement: Requirement::make_and(vec![
                    Requirement::LoseFollowers(FollowerMask::make([
                        Follower::Blind,
                        Follower::SuperBomb,
                    ])),
                    Requirement::ResetHealthToFull,
                ]),
                effects: Box::new([]),
            },
            TraversalEdgeOrigin::SaveAndQuit,
        );
    }
    for (vertex_idx, outgoing) in graph.outgoing.iter().enumerate() {
        for (outgoing_idx, edge) in outgoing.iter().enumerate() {
            let mut dependencies = HashSet::new();
            collect_requirement_dependencies(catalog, &edge.edge.requirement, &mut dependencies);
            // Capability checks read Moon Pearl implicitly in the Dark World.
            if edge.world == World::Dark {
                dependencies.insert(GlobalDependency::Item(UniqueItem::MoonPearl as usize));
            }
            for effect in &edge.edge.effects {
                let dependency = match *effect {
                    Effect::CollectItem(idx) => GlobalDependency::ItemLocation(idx),
                    Effect::SetFlag(idx) => GlobalDependency::Flag(idx),
                    Effect::OpenDoor(idx) => GlobalDependency::Door(idx),
                    Effect::ActivateOcarina => GlobalDependency::Ocarina,
                    Effect::ClearObstacle(_)
                    | Effect::ResetObstacle(_)
                    | Effect::SetFollower(_) => continue,
                };
                dependencies.insert(dependency);
            }
            for dependency in dependencies {
                graph
                    .dependencies
                    .entry(dependency)
                    .or_default()
                    .push(TraversalEdgeIndex {
                        from_vertex_idx: VertexIndex(vertex_idx as u32),
                        outgoing_idx,
                    });
            }
        }
    }
    graph
}
