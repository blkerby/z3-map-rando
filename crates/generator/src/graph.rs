//! Shared world topology for the preliminary key pass and normal traversal.

use common_types::allows_frog_dwarf;
use logic_catalog::{
    Connection, Edge, EdgeIndex, EntranceIndex, EntranceKind, EventEndpoint, Follower,
    FollowerMask, LogicCatalog, NodeType, Requirement, VertexIndex,
};

#[derive(Debug)]
pub struct TraversalGraph {
    /// Catalog vertices only; outgoing edges retain catalog/connection order.
    pub outgoing: Vec<Vec<TraversalEdge>>,
    pub start_vertex_idx: VertexIndex,
}

#[derive(Debug)]
pub struct TraversalEdge {
    pub edge: Edge,
    pub origin: TraversalEdgeOrigin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraversalEdgeOrigin {
    Catalog(EdgeIndex),
    /// Index into the selected connection slice, not necessarily vanilla connections.
    Connection(usize),
    /// Return to the start with follower loss, obstacle reset, and spawn-health reset
    /// expressed through requirements. Prepared resources belong to wave initialization.
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
    fn add_edge(&mut self, catalog: &LogicCatalog, mut edge: Edge, origin: TraversalEdgeOrigin) {
        let destination_room_idx = catalog.vertex(edge.to_vertex_idx).room_idx;
        if origin == TraversalEdgeOrigin::SaveAndQuit
            || catalog.vertex(edge.from_vertex_idx).room_idx != destination_room_idx
        {
            let reset = Requirement::ResetObstacles(destination_room_idx);
            edge.requirement = Requirement::make_and(vec![edge.requirement, reset]);
        }
        self.outgoing[edge.from_vertex_idx.0 as usize].push(TraversalEdge { edge, origin });
    }
}

/// Expand a selected layout without changing catalog vertices, edges, or connections.
/// Pass `catalog.vanilla_connections` for the current vanilla world layout.
pub fn build_graph(catalog: &LogicCatalog, connections: &[Connection]) -> TraversalGraph {
    let mut graph = TraversalGraph {
        outgoing: Vec::new(),
        start_vertex_idx: catalog.start_vertex_idx,
    };
    for _ in &catalog.vertices {
        graph.outgoing.push(Vec::new());
    }

    // Start with the edges in the catalog (room-local edges and flute travel).
    for (index, edge) in catalog.edges.iter().enumerate() {
        graph.add_edge(
            catalog,
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
            Edge {
                from_vertex_idx: VertexIndex(index as u32),
                to_vertex_idx: catalog.start_vertex_idx,
                requirement: Requirement::make_and(vec![
                    Requirement::LoseFollowers(FollowerMask::make([
                        Follower::Blind,
                        Follower::SuperBomb,
                    ])),
                    Requirement::ResetHealthToSpawn,
                ]),
                effects: Box::new([]),
            },
            TraversalEdgeOrigin::SaveAndQuit,
        );
    }
    graph
}
