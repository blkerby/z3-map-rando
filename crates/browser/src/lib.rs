use common_types::Item;
use generator::{
    graph::{TraversalEdge, TraversalEdgeOrigin},
    spoiler::SpoilerReplay,
    state::HealthState,
};
use logic_catalog::{
    Connection, EdgeOrigin, EntranceKind, Follower, ItemLocationIndex, LogicCatalog,
    MapBoundaryKind, OcarinaState, Requirement, TransitionState, VertexIndex, VertexOrigin, World,
};
use seed_data::SeedData;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Patch a vanilla ROM with a saved seed (JSON), customization settings (JSON), and the
/// catalogs matching this build.
#[wasm_bindgen]
pub fn patch_rom(
    rom: &[u8],
    seed_json: &[u8],
    customization_json: &[u8],
    retiling_catalog: &[u8],
    patch_catalog: &[u8],
) -> Result<Vec<u8>, JsError> {
    let patch = || -> anyhow::Result<Vec<u8>> {
        let seed = SeedData::read(seed_json)?;
        let settings = patcher::PatchSettings {
            customization: serde_json::from_slice(customization_json)?,
            transition_asset_phase: Default::default(),
        };
        let retiling = retiling_catalog::decode_catalog(retiling_catalog)?;
        let patches = patch_catalog::decode_catalog(patch_catalog)?;
        patcher::patch_rom(rom, &seed, &retiling, &patches, &settings)
    };
    patch().map_err(|error| JsError::new(&format!("{error:#}")))
}

#[derive(Serialize)]
struct SpoilerOutput {
    spheres: Vec<Vec<SpoilerEntry>>,
    unreachable: Vec<SpoilerEntry>,
}

#[derive(Serialize)]
struct SpoilerEntry {
    /// Catalog item location index, for route queries.
    location_idx: u32,
    location: String,
    /// Item kind token, or None for a location without a saved item.
    item: Option<String>,
    /// For a Small Key, the name of the door it opens.
    door: Option<String>,
    /// For a Map, Compass, or Big Key, the name of its dungeon.
    dungeon: Option<String>,
}

#[derive(Serialize)]
struct RouteRow {
    /// The vertex reached, as room and node names.
    destination: String,
    /// The strat or connection taken, or "Start" for the first row.
    strat: String,
    /// Flags, opened doors, and Ocarina activation read by the edge's requirement, including
    /// alternatives the route may not have used.
    dependencies: Vec<String>,
    state: RouteState,
}

/// Local state on arrival. Health, magic, and ammunition are counted from the start.
#[derive(Serialize)]
struct RouteState {
    form: String,
    /// Eighths of a heart consumed, or remaining after a Fairy revival; exactly one is set.
    health_used: Option<u8>,
    health_remaining: Option<u8>,
    /// Base magic points; a full meter holds 128.
    magic_used: u8,
    bombs_used: u8,
    arrows_used: u8,
    bottles_used: u8,
    /// Followers that may be present; having none is always possible.
    followers: Vec<&'static str>,
}

/// A replayed seed kept in WebAssembly memory, so routes can be traced on demand.
#[wasm_bindgen]
pub struct SpoilerSession {
    catalog: LogicCatalog,
    replay: SpoilerReplay,
}

#[wasm_bindgen]
impl SpoilerSession {
    /// Replay a saved seed (JSON) against the logic catalog matching this build.
    #[wasm_bindgen(constructor)]
    pub fn new(seed_json: &[u8], logic_catalog: &[u8]) -> Result<SpoilerSession, JsError> {
        let replay = || -> anyhow::Result<SpoilerSession> {
            let seed = SeedData::read(seed_json)?;
            let catalog = logic_catalog::decode_catalog(logic_catalog)?;
            let replay = generator::spoiler::replay_spoiler(&catalog, &seed)?;
            Ok(SpoilerSession { catalog, replay })
        };
        replay().map_err(|error| JsError::new(&format!("{error:#}")))
    }

    /// Return the spheres and unreachable locations as JSON, with display names from the
    /// catalog.
    pub fn get_spheres(&self) -> Result<String, JsError> {
        let describe = || -> anyhow::Result<String> {
            let mut spheres = Vec::new();
            for sphere in &self.replay.spheres {
                let mut entries = Vec::new();
                for &location_idx in sphere {
                    entries.push(create_spoiler_entry(
                        &self.catalog,
                        &self.replay.items,
                        location_idx,
                    )?);
                }
                spheres.push(entries);
            }
            let mut unreachable = Vec::new();
            for &location_idx in &self.replay.unreachable {
                unreachable.push(create_spoiler_entry(
                    &self.catalog,
                    &self.replay.items,
                    location_idx,
                )?);
            }
            Ok(serde_json::to_string(&SpoilerOutput {
                spheres,
                unreachable,
            })?)
        };
        describe().map_err(|error| JsError::new(&format!("{error:#}")))
    }

    /// Trace a route to an item location, returning its rows as JSON, or null if none is found.
    pub fn trace_route(&self, location_idx: u32) -> Result<String, JsError> {
        let catalog = &self.catalog;
        let mut rows = None;
        if let Some(route) = self
            .replay
            .trace_route(catalog, ItemLocationIndex(location_idx))
        {
            let mut route_rows = Vec::new();
            for step in route {
                let mut strat = "Start".to_owned();
                let mut dependencies = Vec::new();
                if let Some(index) = step.edge {
                    let edge = &self.replay.graph.outgoing[index.from_vertex_idx.0 as usize]
                        [index.outgoing_idx];
                    strat = describe_edge(catalog, &self.replay.connections, edge);
                    collect_dependencies(catalog, &edge.edge.requirement, &mut dependencies);
                }
                let state = step.state;
                let mut followers = Vec::new();
                for (follower, name) in FOLLOWER_NAMES {
                    if state.possible_followers.0 & (1 << follower as u8) != 0 {
                        followers.push(name);
                    }
                }
                let (health_used, health_remaining) = match state.health {
                    HealthState::Consumed(used) => (Some(used), None),
                    HealthState::Remaining(remaining) => (None, Some(remaining)),
                };
                route_rows.push(RouteRow {
                    destination: describe_vertex(catalog, step.vertex_idx),
                    strat,
                    dependencies,
                    state: RouteState {
                        form: format!("{:?}", state.form),
                        health_used,
                        health_remaining,
                        magic_used: state.magic_used,
                        bombs_used: state.bombs_used,
                        arrows_used: state.arrows_used,
                        bottles_used: state.bottles_used,
                        followers,
                    },
                });
            }
            rows = Some(route_rows);
        }
        serde_json::to_string(&rows).map_err(|error| JsError::new(&error.to_string()))
    }
}

const FOLLOWER_NAMES: [(Follower, &str); 6] = [
    (Follower::Zelda, "Zelda"),
    (Follower::OldMan, "Old Man"),
    (Follower::Blind, "Blind"),
    (Follower::Dwarf, "Dwarf"),
    (Follower::PurpleChest, "Purple Chest"),
    (Follower::SuperBomb, "Super Bomb"),
];

fn describe_vertex(catalog: &LogicCatalog, vertex_idx: VertexIndex) -> String {
    let room = catalog.room(catalog.vertex(vertex_idx).room_idx);
    let metadata = catalog.vertex_metadata(vertex_idx);
    let mut name = format!("{}: {}", room.name, room.node(metadata.node_idx).name);
    if metadata.world == Some(World::Dark) {
        name += " (Dark World)";
    }
    if let VertexOrigin::EventEntry { event_idx } | VertexOrigin::EventExit { event_idx } =
        metadata.origin
        && let TransitionState::Event(event) = catalog.get_transition_state(event_idx)
    {
        name += &format!(" [{event}]");
    }
    name
}

fn describe_edge(
    catalog: &LogicCatalog,
    connections: &[Connection],
    edge: &TraversalEdge,
) -> String {
    match edge.origin {
        TraversalEdgeOrigin::Catalog(edge_idx) => match catalog.edge_metadata(edge_idx).origin {
            EdgeOrigin::Strat {
                room_idx,
                strat_idx,
            } => catalog.room(room_idx).strat(strat_idx).name.clone(),
            EdgeOrigin::ImplicitMirror => "Mirror".to_owned(),
            EdgeOrigin::ImplicitFlute => "Flute".to_owned(),
            EdgeOrigin::LakeHyliaBottleExchange => "Bottle exchange".to_owned(),
        },
        TraversalEdgeOrigin::Connection(connection_idx) => match &connections[connection_idx] {
            Connection::MapBoundary { kind, .. } => match kind {
                MapBoundaryKind::Ordinary => "Cross room boundary".to_owned(),
                MapBoundaryKind::Mirror => "Mirror across room boundary".to_owned(),
                MapBoundaryKind::Return => "Leave and return".to_owned(),
            },
            Connection::Entrance {
                kind, entrance_idx, ..
            } => {
                let entrance = catalog.entrance(*entrance_idx);
                let outside = catalog.vertex(entrance.vertex_idx).room_idx
                    == catalog.vertex(edge.edge.from_vertex_idx).room_idx;
                match kind {
                    EntranceKind::Drop => format!("Drop into {}", entrance.name),
                    EntranceKind::Door if outside => format!("Enter {}", entrance.name),
                    EntranceKind::Door => format!("Exit {}", entrance.name),
                }
            }
            Connection::Teleport { .. } => "Teleport".to_owned(),
            Connection::Whirlpool { .. } => "Whirlpool".to_owned(),
            Connection::ScreenBoundary { .. } => "Cross screen boundary".to_owned(),
        },
        TraversalEdgeOrigin::SaveAndQuit => "Save and quit".to_owned(),
    }
}

/// Collect the progress a requirement reads that earlier exploration, rather than inventory,
/// provides.
fn collect_dependencies(
    catalog: &LogicCatalog,
    requirement: &Requirement,
    dependencies: &mut Vec<String>,
) {
    let dependency = match requirement {
        Requirement::And(children) | Requirement::Or(children) => {
            for child in children {
                collect_dependencies(catalog, child, dependencies);
            }
            return;
        }
        Requirement::Flag(idx) => catalog.flag(*idx).to_owned(),
        Requirement::Door(idx) => {
            let door = catalog.door(*idx);
            format!("{} - {}", catalog.room(door.room_idx).name, door.name)
        }
        Requirement::Ocarina(OcarinaState::Active) => "Ocarina activated".to_owned(),
        _ => return,
    };
    if !dependencies.contains(&dependency) {
        dependencies.push(dependency);
    }
}

fn create_spoiler_entry(
    catalog: &LogicCatalog,
    items: &[Option<Item>],
    location_idx: ItemLocationIndex,
) -> anyhow::Result<SpoilerEntry> {
    let item = items[location_idx.0 as usize];
    let mut item_name = None;
    let mut door = None;
    let mut dungeon = None;
    if let Some(item) = item {
        // An item kind serializes as its token string.
        item_name = Some(
            serde_json::to_value(item.get_kind())?
                .as_str()
                .unwrap()
                .to_owned(),
        );
        if let Item::Map(room_id) | Item::Compass(room_id) | Item::BigKey(room_id) = item {
            for room in &catalog.rooms {
                if room.room_id == Some(room_id) {
                    dungeon = Some(room.name.clone());
                }
            }
        }
        if let Item::SmallKey(door_id) = item {
            for catalog_door in &catalog.doors {
                let room = &catalog.rooms[catalog_door.room_idx.0 as usize];
                if room.room_id == Some(door_id.room_id) && catalog_door.door_id == door_id.door_id
                {
                    door = Some(format!("{} - {}", room.name, catalog_door.name));
                }
            }
        }
    }
    Ok(SpoilerEntry {
        location_idx: location_idx.0,
        location: catalog.item_locations[location_idx.0 as usize].name.clone(),
        item: item_name,
        door,
        dungeon,
    })
}
