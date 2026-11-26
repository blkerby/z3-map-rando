//! Shared world topology for the preliminary key pass and normal traversal.

use crate::{interpreter::collect_requirement_dependencies, state::GlobalDependency};
use common_types::allows_frog_dwarf;
use hashbrown::{HashMap, HashSet};
use logic_catalog::{
    Connection, Edge, EdgeIndex, Effect, EntranceIndex, EntranceKind, EventEndpoint, Follower,
    FollowerMask, LogicCatalog, MapBoundaryKind, OverheadClass, Requirement, SourceRoomId,
    TransitionState, UniqueItem, VertexIndex, VertexOrigin, World,
};

pub(crate) const BUNNY_WORLD: World = World::Dark;

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
        // State vertices share the ordinary endpoint's room, node, and world.
        self.entrance_idx.is_some()
            || (catalog.vertex(self.vertex_idx).room_idx
                == catalog.vertex(event.vertex_idx).room_idx
                && catalog.vertex_metadata(self.vertex_idx).node_idx
                    == catalog.vertex_metadata(event.vertex_idx).node_idx
                && catalog.vertex_metadata(self.vertex_idx).world
                    == catalog.vertex_metadata(event.vertex_idx).world)
    }
}

impl TraversalGraph {
    fn add_edge(&mut self, catalog: &LogicCatalog, mut edge: Edge, origin: TraversalEdgeOrigin) {
        let source_room_idx = catalog.vertex(edge.from_vertex_idx).room_idx;
        let destination_room_idx = catalog.vertex(edge.to_vertex_idx).room_idx;
        if origin == TraversalEdgeOrigin::SaveAndQuit || source_room_idx != destination_room_idx {
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
        dependencies: HashMap::new(),
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
            Connection::MapBoundary {
                endpoints, kind, ..
            } => (
                ConnectionEndpoint {
                    vertex_idx: endpoints[0],
                    entrance_idx: None,
                },
                ConnectionEndpoint {
                    vertex_idx: endpoints[1],
                    entrance_idx: None,
                },
                matches!(kind, MapBoundaryKind::Ordinary),
                match kind {
                    MapBoundaryKind::Ordinary => Requirement::Free,
                    MapBoundaryKind::Mirror => Requirement::make_and(vec![
                        Requirement::Item(UniqueItem::MagicMirror),
                        Requirement::ResetForm {
                            world: catalog.vertex_metadata(endpoints[1]).world.unwrap(),
                            only_with_pearl: false,
                        },
                    ]),
                    MapBoundaryKind::Return => {
                        Requirement::ResetObstacles(catalog.vertex(endpoints[0]).room_idx)
                    }
                },
            ),
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
                    EntranceKind::Drop => Requirement::make_and(vec![
                        catalog.fall_to_lower_floor.clone(),
                        Requirement::LoseFollowers(FollowerMask::make([Follower::SuperBomb])),
                    ]),
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
            } => {
                let destination = catalog.teleports[teleport_idx.0 as usize].vertex_idx;
                let world = catalog.vertex_metadata(destination).world.unwrap();
                let room = catalog.room(catalog.vertex(*from_vertex_idx).room_idx);
                let requirement = match room.room_id {
                    // Chris Houlihan Room
                    Some(SourceRoomId(60)) => Requirement::make_and(vec![
                        Requirement::LoseFollowers(FollowerMask::make([Follower::Blind])),
                        Requirement::ResetForm {
                            world,
                            only_with_pearl: true,
                        },
                    ]),
                    // Agahnim 2 in Ganon's Tower
                    Some(SourceRoomId(140)) => Requirement::SetLink,
                    _ => Requirement::ResetForm {
                        world,
                        only_with_pearl: false,
                    },
                };
                (
                    ConnectionEndpoint {
                        vertex_idx: *from_vertex_idx,
                        entrance_idx: None,
                    },
                    ConnectionEndpoint {
                        vertex_idx: destination,
                        entrance_idx: None,
                    },
                    false,
                    requirement,
                )
            }
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
        let reverse_requirement = match connection {
            Connection::Entrance {
                kind: EntranceKind::Door,
                ..
            } => {
                let world = catalog.vertex_metadata(first.vertex_idx).world.unwrap();
                Requirement::make_and(vec![
                    Requirement::LoseFollowers(FollowerMask::make([Follower::Blind])),
                    Requirement::ResetForm {
                        world,
                        only_with_pearl: world != BUNNY_WORLD,
                    },
                ])
            }
            _ => Requirement::Free,
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
            if !matches!(
                connection,
                Connection::Entrance { .. }
                    | Connection::ScreenBoundary { .. }
                    | Connection::MapBoundary { .. }
            ) {
                continue;
            }
            for exit in &catalog.event_exits {
                if !from.match_event(catalog, exit) {
                    continue;
                }
                let state = catalog.get_transition_state(exit.event_idx);
                match (state, connection) {
                    (TransitionState::Event(_), Connection::Entrance { .. })
                    | (
                        TransitionState::Overhead(_),
                        Connection::ScreenBoundary { .. } | Connection::MapBoundary { .. },
                    ) => {}
                    (
                        TransitionState::LeaveNormally,
                        Connection::ScreenBoundary { .. } | Connection::MapBoundary { .. },
                    ) => {
                        graph.add_edge(
                            catalog,
                            Edge {
                                from_vertex_idx: exit.vertex_idx,
                                to_vertex_idx: to.vertex_idx,
                                requirement: requirement.clone(),
                                effects: Box::new([]),
                            },
                            TraversalEdgeOrigin::Connection(connection_idx),
                        );
                        continue;
                    }
                    _ => continue,
                }
                for entry in &catalog.event_entries {
                    let compatible = exit.event_idx == entry.event_idx
                        || matches!(
                            (state, catalog.get_transition_state(entry.event_idx)),
                            (
                                TransitionState::Overhead(OverheadClass::Class3),
                                TransitionState::Overhead(OverheadClass::Class1)
                            )
                        );
                    if compatible && to.match_event(catalog, entry) {
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
        // A forced-exit strat cannot finish until it crosses the screen boundary.
        if let VertexOrigin::EventExit { event_idx } = catalog.vertex_metadata[index].origin {
            if matches!(
                catalog.get_transition_state(event_idx),
                TransitionState::LeaveNormally
            ) {
                continue;
            }
        }
        graph.add_edge(
            catalog,
            Edge {
                from_vertex_idx: VertexIndex(index as u32),
                to_vertex_idx: catalog.start_vertex_idx,
                requirement: Requirement::make_and(vec![
                    Requirement::LoseFollowers(FollowerMask::make([
                        Follower::Zelda,
                        Follower::OldMan,
                        Follower::Blind,
                        Follower::Dwarf,
                        Follower::PurpleChest,
                        Follower::SuperBomb,
                    ])),
                    Requirement::ResetHealthToFull,
                    Requirement::ResetForm {
                        world: World::Light,
                        only_with_pearl: false,
                    },
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
