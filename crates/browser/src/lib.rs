use common_types::{Item, SourceRoomId};
use generator::{
    graph::{TraversalEdge, TraversalEdgeOrigin},
    inputs::prepare_skill_inputs,
    spoiler::{RouteTarget, SpoilerReplay},
    state::{GlobalState, HealthState, LocalState},
    strat_check::check_room_strats,
};
use logic_catalog::{
    BoundaryEndpoint, Connection, Direction, DoorIndex, EdgeOrigin, EntranceKind, FlagIndex,
    Follower, ItemLocationIndex, LogicCatalog, MapBoundaryKind, OcarinaState, Requirement, Room,
    RoomIndex, TransitionState, VertexIndex, VertexOrigin, World,
};
use patcher::{
    import::{OverworldEntrance, SPECIAL_TRANSITIONS},
    retiling::get_edge_openings,
};
use seed_data::{BorderSide, SeedData, Settings};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Patch a vanilla ROM with a saved seed (JSON), optional override generation settings (empty to
/// use the seed's recorded settings), customization settings (JSON), and the catalogs matching
/// this build.
#[wasm_bindgen]
pub fn patch_rom(
    rom: &[u8],
    seed_json: &[u8],
    settings_json: &[u8],
    customization_json: &[u8],
    retiling_catalog: &[u8],
    patch_catalog: &[u8],
) -> Result<Vec<u8>, JsError> {
    let patch = || -> anyhow::Result<Vec<u8>> {
        let mut seed = SeedData::read(seed_json)?;
        if !settings_json.is_empty() {
            seed.settings = serde_json::from_slice(settings_json)?;
        }
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

/// One step of a route: the node it reaches and the edge taken to reach it.
#[derive(Serialize)]
struct RouteRow {
    /// For a step entering a room or changing world, the connection and the room entered,
    /// such as "West to Swamp Ruins" or "Portal to Ice Lake"; such a step starts a new group of
    /// rows.
    entered: Option<String>,
    /// The node reached, as "<node ID>: <node name>", so it can be found on the room diagrams.
    node: String,
    /// Whether the step returns to the node it leaves from, in the same world.
    stays: bool,
    /// Whether the node is in the Dark World, or in an interior entered from it.
    dark_world: bool,
    /// The strat name, or a name synthesized from the connection kind, such as "Cross West" or
    /// "Flute"; None for the route's start.
    strat: Option<String>,
    /// The room ID of the node reached, for the room's logic page; the synthetic flute hub, the
    /// only room without one, is never reached by a row.
    room_id: u32,
    /// `(room_id, strat_id)` when this step is an authored strat with a logic page.
    #[serde(skip_serializing_if = "Option::is_none")]
    strat_link: Option<(u32, u32)>,
    /// Flags, opened doors, and Ocarina activation read by the edge's requirement, including
    /// alternatives the route may not have used.
    dependencies: Vec<Dependency>,
    state: RouteState,
    /// Where the edge crosses an overworld entrance, or for a map boundary, points just before
    /// and after it; then for the route's last edge, where its target item sits on the
    /// overworld. Empty for other edges, or for a door before the world maps are loaded.
    points: Vec<MapPoint>,
    /// Whether the edge is a warp or starts in an interior, so travel since the previous point
    /// is not drawn as overworld travel.
    indirect: bool,
}

/// A position on the cropped world maps, in their pixels.
#[derive(Clone, Copy, Serialize)]
struct MapPoint {
    world: &'static str,
    x: f32,
    y: f32,
}

/// Pixels cropped from each edge of the 512x512 world maps: three quarters of the clouds around
/// the room grid, which starts 128 pixels in, as on the Logic index page's map. Each 16x16
/// overworld tile is one map pixel.
const MAP_MARGIN: usize = 96;

/// Distance in map pixels of a map boundary crossing's points from the boundary, so the line
/// between them shows the direction of the crossing.
const CROSSING_OFFSET: f32 = 6.0;

/// Overworld item locations as (room ID, item ID, x, y), with the item's position in 16x16
/// tiles from its room's top-left, from vanilla sprite lists and tile checks. Other locations
/// are in interiors or special areas, and use the route's last point.
const OVERWORLD_ITEM_TILES: [(u32, u32, u8, u8); 18] = [
    (1, 1, 30, 21),  // Mushroom
    (3, 2, 34, 22),  // Spectacle Rock
    (3, 3, 11, 4),   // Ether Tablet
    (4, 1, 47, 3),   // Floating Island
    (6, 1, 24, 8),   // Bumper Cave Ledge
    (7, 1, 4, 11),   // Catfish
    (16, 1, 32, 26), // Weathervane
    (16, 2, 24, 22), // Bottle Merchant
    (18, 1, 52, 18), // Pyramid Ledge
    (23, 1, 7, 19),  // Maze Race
    (23, 2, 14, 17), // Digging Game
    (25, 1, 9, 9),   // Flute dig spot
    (25, 2, 14, 15), // Stumpy
    (31, 1, 6, 42),  // Desert Ledge
    (31, 2, 55, 43), // Bombos Tablet
    (35, 1, 25, 19), // Lake Hylia Island
    (37, 1, 23, 5),  // Purple Chest
    (38, 1, 20, 14), // Sunken Item
];

/// Overworld pit entrances as (ROM entrance ID, x, y), with the hole's center in 16x16 tiles
/// from its area's top-left. Vanilla's pit tables hold Link's position after the fall moves him
/// to the top of the screen, 120 to 144 pixels above the hole, so these are measured instead:
/// from the hidden-item table for holes under bushes, from the overlay and gravestone routines
/// for the Lumberjack tree, Pyramid, and Cemetery grave, and from the Map16 grid otherwise.
const PIT_TILES: [(u8, f32, f32); 12] = [
    (0x76, 48.5, 32.5), // Skull Woods bush
    (0x77, 40.0, 45.0), // Skull Woods bottom left
    (0x78, 50.0, 43.0), // Skull Woods bottom right
    (0x79, 31.0, 23.0), // Skull Woods top left
    (0x7a, 48.5, 32.5), // Lost Woods Hideout bush
    (0x7b, 31.5, 8.5),  // Ganon drop in the Pyramid
    (0x7c, 4.5, 14.5),  // North Great Fairy bush
    (0x7d, 56.5, 10.5), // Hyrule Castle secret entrance bush
    (0x7e, 19.0, 16.0), // Magic Bat Cave
    (0x7f, 13.0, 19.0), // Lumberjack tree
    (0x80, 6.0, 13.0),  // Kakariko Well
    (0x81, 5.0, 11.0),  // Hyrule Castle Sewers grave
];

#[derive(Serialize)]
struct Dependency {
    name: String,
    /// The route to follow for it, or None for progress held from the start.
    target: Option<TargetJson>,
}

/// A route target as JSON, such as `{"kind": "location", "index": 3}` or `{"kind": "ocarina"}`.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "kind", content = "index", rename_all = "snake_case")]
enum TargetJson {
    Location(u32),
    Flag(u32),
    Door(u32),
    Ocarina,
}

/// Local state after the step. Health, magic, and ammunition are counted from the start.
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

/// The logic catalog kept in WebAssembly memory, so the Logic pages can check strats against
/// skill settings.
#[wasm_bindgen]
pub struct LogicSession {
    catalog: LogicCatalog,
    global: GlobalState,
}

#[wasm_bindgen]
impl LogicSession {
    #[wasm_bindgen(constructor)]
    pub fn new(logic_catalog: &[u8]) -> Result<LogicSession, JsError> {
        let catalog = logic_catalog::decode_catalog(logic_catalog)
            .map_err(|error| JsError::new(&format!("{error:#}")))?;
        let global = GlobalState::initialize_full(&catalog);
        Ok(LogicSession { catalog, global })
    }

    /// IDs of the room's strats that pass with the skill settings (JSON), as checked by
    /// `check_room_strats` with everything owned.
    pub fn check_room_strats(&self, room_id: u32, skill_json: &[u8]) -> Result<Vec<u32>, JsError> {
        let check = || -> anyhow::Result<Vec<u32>> {
            let skill = prepare_skill_inputs(&self.catalog, &serde_json::from_slice(skill_json)?)?;
            let mut room_idx = RoomIndex(0);
            while self.catalog.room(room_idx).room_id != Some(SourceRoomId(room_id)) {
                room_idx.0 += 1;
            }
            let passes = check_room_strats(&self.catalog, &self.global, &skill, room_idx);
            let mut strat_ids = Vec::new();
            for (strat, passes) in self.catalog.room(room_idx).strats.iter().zip(passes) {
                if let (Some(strat_id), true) = (strat.strat_id, passes) {
                    strat_ids.push(strat_id);
                }
            }
            Ok(strat_ids)
        };
        check().map_err(|error| JsError::new(&format!("{error:#}")))
    }
}

/// A replayed seed kept in WebAssembly memory, so routes can be traced on demand.
#[wasm_bindgen]
pub struct SpoilerSession {
    catalog: LogicCatalog,
    replay: SpoilerReplay,
    seed: SeedData,
    /// Vanilla overworld door entrances, by vanilla area, once the world maps are loaded from
    /// a ROM.
    doors: Option<Vec<Vec<OverworldEntrance>>>,
}

#[wasm_bindgen]
impl SpoilerSession {
    /// Replay a saved seed (JSON) against the logic catalog matching this build, using optional
    /// override generation settings (empty to use the seed's recorded settings).
    #[wasm_bindgen(constructor)]
    pub fn new(
        seed_json: &[u8],
        settings_json: &[u8],
        logic_catalog: &[u8],
    ) -> Result<SpoilerSession, JsError> {
        let replay = || -> anyhow::Result<SpoilerSession> {
            let seed = SeedData::read(seed_json)?;
            let catalog = logic_catalog::decode_catalog(logic_catalog)?;
            let replay = if settings_json.is_empty() {
                generator::spoiler::replay_spoiler(&catalog, &seed, &seed.settings)?
            } else {
                let settings: Settings = serde_json::from_slice(settings_json)?;
                generator::spoiler::replay_spoiler(&catalog, &seed, &settings)?
            };
            Ok(SpoilerSession {
                catalog,
                replay,
                seed,
                doors: None,
            })
        };
        replay().map_err(|error| JsError::new(&format!("{error:#}")))
    }

    /// Recompute spheres and routes under new generation settings (empty to use the seed's
    /// recorded settings), keeping the loaded catalog, graph, and map door positions.
    pub fn replay(&mut self, settings_json: &[u8]) -> Result<(), JsError> {
        let mut update = || -> anyhow::Result<()> {
            if settings_json.is_empty() {
                self.replay.replay(&self.catalog, &self.seed.settings)
            } else {
                let settings: Settings = serde_json::from_slice(settings_json)?;
                self.replay.replay(&self.catalog, &settings)
            }
        };
        update().map_err(|error| JsError::new(&format!("{error:#}")))
    }

    /// Render the seed's Light and Dark World maps as 320x320 PNGs from a vanilla ROM, the
    /// seed's filtered retiling catalog, and the patch catalog, cropping three quarters of the
    /// clouds around the room grid. Also read the ROM's door positions, so later routes include
    /// door points.
    pub fn load_world_maps(
        &mut self,
        rom: &[u8],
        retiling_catalog: &[u8],
        patch_catalog: &[u8],
    ) -> Result<Vec<js_sys::Uint8Array>, JsError> {
        let mut render = || -> anyhow::Result<Vec<Vec<u8>>> {
            let retiling = retiling_catalog::decode_catalog(retiling_catalog)?;
            let patches = patch_catalog::decode_catalog(patch_catalog)?;
            let layout = patcher::resolve_world_layout(&self.seed, &patches)?;
            let importer = patcher::import::Importer::new(rom.to_vec())?;
            let map = patcher::retiling::compile_world_map(&retiling, &importer, &layout, true)?;
            self.doors = Some(importer.load_overworld_entrances()?.0);
            map.encode_pngs(MAP_MARGIN)
        };
        let pngs = render().map_err(|error| JsError::new(&format!("{error:#}")))?;
        let mut output = Vec::new();
        for png in pngs {
            output.push(js_sys::Uint8Array::from(png.as_slice()));
        }
        Ok(output)
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

    /// Trace a route to a target (JSON, as in a dependency's target), returning its rows as
    /// JSON, or null if none is found.
    pub fn trace_route(&self, target_json: &str) -> Result<String, JsError> {
        let catalog = &self.catalog;
        let target = match serde_json::from_str(target_json)
            .map_err(|error| JsError::new(&error.to_string()))?
        {
            TargetJson::Location(idx) => RouteTarget::ItemLocation(ItemLocationIndex(idx)),
            TargetJson::Flag(idx) => RouteTarget::Flag(FlagIndex(idx)),
            TargetJson::Door(idx) => RouteTarget::Door(DoorIndex(idx)),
            TargetJson::Ocarina => RouteTarget::Ocarina,
        };
        // Interior rooms take the world of the overworld side of their entrances.
        let mut interior_worlds = vec![None; catalog.rooms.len()];
        for connection in &self.replay.connections {
            if let Connection::Entrance {
                entrance_idx,
                interior_vertex_idx,
                ..
            } = connection
            {
                let room_idx = catalog.vertex(*interior_vertex_idx).room_idx;
                let entrance_vertex_idx = catalog.entrance(*entrance_idx).vertex_idx;
                interior_worlds[room_idx.0 as usize] =
                    catalog.vertex_metadata(entrance_vertex_idx).world;
            }
        }
        let mut rows = None;
        if let Some(route) = self.replay.trace_route(catalog, target) {
            let mut route_rows = Vec::new();
            // The edge into the synthetic flute hub, whose step is merged into the next one, out
            // of the hub, so the warp is one step.
            let mut flute_edge = None;
            for step in route {
                let (room_name, node) = describe_vertex(catalog, step.vertex_idx);
                let room_id = catalog
                    .room(catalog.vertex(step.vertex_idx).room_idx)
                    .room_id
                    .map(|id| id.0);
                let world = catalog
                    .vertex_metadata(step.vertex_idx)
                    .world
                    .or(interior_worlds[catalog.vertex(step.vertex_idx).room_idx.0 as usize]);
                let dark_world = world == Some(World::Dark);
                let Some(index) = step.edge else {
                    route_rows.push(RouteRow {
                        entered: Some(format!("Start at {room_name}")),
                        node,
                        dark_world,
                        stays: false,
                        strat: None,
                        room_id: room_id.unwrap(),
                        strat_link: None,
                        dependencies: Vec::new(),
                        state: describe_state(&step.state),
                        points: Vec::new(),
                        indirect: false,
                    });
                    continue;
                };
                let edge = &self.replay.graph.outgoing[index.from_vertex_idx.0 as usize]
                    [index.outgoing_idx];
                if step.vertex_idx == catalog.flute_hub_vertex_idx {
                    flute_edge = Some(edge);
                    continue;
                }
                let first_edge = flute_edge.take().unwrap_or(edge);
                let stays = catalog.vertex(index.from_vertex_idx).room_idx
                    == catalog.vertex(step.vertex_idx).room_idx
                    && catalog.vertex_metadata(index.from_vertex_idx).node_idx
                        == catalog.vertex_metadata(step.vertex_idx).node_idx
                    && catalog.vertex_metadata(index.from_vertex_idx).world
                        == catalog.vertex_metadata(step.vertex_idx).world;
                // For a flute warp, both edges' requirements count; repeated names are skipped.
                let mut dependencies = Vec::new();
                for requirement in [&first_edge.edge.requirement, &edge.edge.requirement] {
                    collect_dependencies(catalog, &self.replay, requirement, &mut dependencies);
                }
                let strat_link = match first_edge.origin {
                    TraversalEdgeOrigin::Catalog(edge_idx) => {
                        match catalog.edge_metadata(edge_idx).origin {
                            EdgeOrigin::Strat {
                                room_idx,
                                strat_idx,
                            } => {
                                let room = catalog.room(room_idx);
                                match (room.room_id, room.strat(strat_idx).strat_id) {
                                    (Some(room_id), Some(strat_id)) => Some((room_id.0, strat_id)),
                                    _ => None,
                                }
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                };
                let (strat, entered) =
                    describe_edge(catalog, &self.replay.connections, first_edge, room_name);
                route_rows.push(RouteRow {
                    entered,
                    node,
                    dark_world,
                    stays,
                    strat: Some(strat),
                    room_id: room_id.unwrap(),
                    strat_link,
                    dependencies,
                    state: describe_state(&step.state),
                    points: self.locate_edge(edge).unwrap_or_default(),
                    indirect: check_indirect(catalog, &self.replay.connections, first_edge),
                });
            }
            if let RouteTarget::ItemLocation(location_idx) = target
                && let Some(point) = self.locate_item(location_idx)
                && let Some(last) = route_rows.last_mut()
            {
                last.points.push(point);
            }
            rows = Some(route_rows);
        }
        serde_json::to_string(&rows).map_err(|error| JsError::new(&error.to_string()))
    }
}

impl SpoilerSession {
    /// Place a position in 16x16 tiles from an overworld room's top-left on the maps, at the
    /// room's position in the seed's layout.
    fn locate_tile(&self, vertex_idx: VertexIndex, tile_x: f32, tile_y: f32) -> Option<MapPoint> {
        let room = self.catalog.room(self.catalog.vertex(vertex_idx).room_idx);
        let world = match self.catalog.vertex_metadata(vertex_idx).world? {
            World::Light => "light",
            World::Dark => "dark",
        };
        let [x, y] = self.find_room_cell(room)?;
        // A map cell is one 32x32-tile area.
        let origin = 128 - MAP_MARGIN;
        Some(MapPoint {
            world,
            x: (origin + usize::from(x) * 32) as f32 + tile_x,
            y: (origin + usize::from(y) * 32) as f32 + tile_y,
        })
    }

    /// Find an overworld room's top-left cell in the seed's layout.
    fn find_room_cell(&self, room: &Room) -> Option<[u8; 2]> {
        for seed_room in &self.seed.rooms {
            if Some(seed_room.room_id) == room.room_id {
                return seed_room.position;
            }
        }
        None
    }

    /// Locate where an edge crosses a map boundary or overworld entrance.
    fn locate_edge(&self, edge: &TraversalEdge) -> Option<Vec<MapPoint>> {
        let TraversalEdgeOrigin::Connection(connection_idx) = edge.origin else {
            return None;
        };
        match &self.replay.connections[connection_idx] {
            Connection::MapBoundary {
                endpoints,
                kind,
                direction,
            } => {
                let vertex_idx = edge.edge.from_vertex_idx;
                let room_idx = self.catalog.vertex(vertex_idx).room_idx;
                let room = self.catalog.room(room_idx);
                let side =
                    match get_travel_direction(&self.catalog, endpoints, *direction, room_idx) {
                        Direction::North => 0,
                        Direction::South => 1,
                        Direction::East => 2,
                        Direction::West => 3,
                    };
                // The position along the side, in 16x16 tiles: the center of the source vertex's
                // opening in its segment's selected variant, or the segment's middle where the
                // variant's geometry is unknown. A segment spans one map cell, and only segments
                // facing the other endpoint's room count. Where the vertex still has several
                // openings, such as both of a dual-small variant's, the first is used.
                let [cell_x, cell_y] = self.find_room_cell(room)?;
                let [width, height] = room.size?;
                // Sizes count 16x16-tile screens, two per cell.
                let cells_wide = u8::try_from(width / 2).ok()?;
                let cells_high = u8::try_from(height / 2).ok()?;
                let other_idx = if endpoints[0] == vertex_idx {
                    endpoints[1]
                } else {
                    endpoints[0]
                };
                let other_room = self.catalog.room(self.catalog.vertex(other_idx).room_idx);
                let [other_x, other_y] = self.find_room_cell(other_room)?;
                let [other_width, other_height] = other_room.size?;
                let mut along = None;
                'segments: for (segment, variants) in room.edge_variants[side].iter().enumerate() {
                    let segment_cells = u8::try_from(segment).ok()?;
                    // The cell across the border; a return leads back into the same room.
                    let [across_x, across_y] = match side {
                        0 => [cell_x + segment_cells, cell_y.wrapping_sub(1)],
                        1 => [cell_x + segment_cells, cell_y + cells_high],
                        2 => [cell_x + cells_wide, cell_y + segment_cells],
                        _ => [cell_x.wrapping_sub(1), cell_y + segment_cells],
                    };
                    if !matches!(kind, MapBoundaryKind::Return)
                        && !((other_x..other_x + u8::try_from(other_width / 2).ok()?)
                            .contains(&across_x)
                            && (other_y..other_y + u8::try_from(other_height / 2).ok()?)
                                .contains(&across_y))
                    {
                        continue;
                    }
                    // The cell west or north of the border, as the seed records it.
                    let border = match side {
                        0 => cell_y
                            .checked_sub(1)
                            .map(|y| ([cell_x + segment_cells, y], BorderSide::South)),
                        1 => Some((
                            [cell_x + segment_cells, cell_y + cells_high - 1],
                            BorderSide::South,
                        )),
                        2 => Some((
                            [cell_x + cells_wide - 1, cell_y + segment_cells],
                            BorderSide::East,
                        )),
                        _ => cell_x
                            .checked_sub(1)
                            .map(|x| ([x, cell_y + segment_cells], BorderSide::East)),
                    };
                    let mut selected = None;
                    for map_border in &self.seed.borders {
                        if Some((map_border.position, map_border.side)) == border {
                            selected = Some(map_border.variant.as_str());
                        }
                    }
                    let holds_vertex = |endpoint: &BoundaryEndpoint| {
                        matches!(endpoint, BoundaryEndpoint::Node { light, dark }
                            if *light == Some(vertex_idx) || *dark == Some(vertex_idx))
                    };
                    let segment_start = segment as f32 * 32.0;
                    if let Some(selected) = selected {
                        let openings = get_edge_openings(side < 2, selected);
                        for (index, endpoint) in variants[selected].iter().enumerate() {
                            // A light-and-mirror crossing starts from the ordinary Light World
                            // role, or for a Mirror crossing, the Dark World Mirror spot.
                            let role = match kind {
                                MapBoundaryKind::Mirror => 1,
                                _ => 0,
                            };
                            if !holds_vertex(endpoint)
                                || (selected == "light-and-mirror" && index != role)
                            {
                                continue;
                            }
                            // Openings are measured in 8x8 tiles.
                            along = Some(match openings.get(index) {
                                Some(&(start, end)) => segment_start + (start + end) as f32 / 4.0,
                                None => segment_start + 16.0,
                            });
                            break 'segments;
                        }
                    } else {
                        // A vanilla layout has no borders, so use the segment's middle.
                        let mut holds = false;
                        for endpoints in variants.values() {
                            holds |= endpoints.iter().any(holds_vertex);
                        }
                        if holds {
                            along = Some(segment_start + 16.0);
                            break;
                        }
                    }
                }
                let along = along?;
                let (width, height) = (f32::from(width) * 16.0, f32::from(height) * 16.0);
                // The boundary's position and the direction of travel across it.
                let (x, y, dx, dy) = match side {
                    0 => (along, 0.0, 0.0, -1.0),
                    1 => (along, height, 0.0, 1.0),
                    2 => (width, along, 1.0, 0.0),
                    _ => (0.0, along, -1.0, 0.0),
                };
                let boundary = self.locate_tile(vertex_idx, x, y)?;
                let mut before = boundary;
                before.x -= dx * CROSSING_OFFSET;
                before.y -= dy * CROSSING_OFFSET;
                // A route leaving and returning comes straight back, so it has no point beyond.
                if let MapBoundaryKind::Return = kind {
                    return Some(vec![before]);
                }
                let mut after = boundary;
                after.x += dx * CROSSING_OFFSET;
                after.y += dy * CROSSING_OFFSET;
                Some(vec![before, after])
            }
            Connection::Entrance {
                kind, entrance_idx, ..
            } => {
                let entrance = self.catalog.entrance(*entrance_idx);
                if let EntranceKind::Drop = kind {
                    let entrance_id = entrance.rom_entrance_id?;
                    for (pit_entrance_id, x, y) in PIT_TILES {
                        if pit_entrance_id == entrance_id {
                            return self
                                .locate_tile(entrance.vertex_idx, x, y)
                                .map(|point| vec![point]);
                        }
                    }
                    return None;
                }
                let room = self
                    .catalog
                    .room(self.catalog.vertex(entrance.vertex_idx).room_idx);
                let [x, y] = room.position?;
                let mut area = usize::from(y) * 8 + usize::from(x);
                if self.catalog.vertex_metadata(entrance.vertex_idx).world == Some(World::Dark) {
                    area += 0x40;
                }
                let mut offsets = Vec::new();
                if let Some(entrance_id) = entrance.rom_entrance_id {
                    for door in &self.doors.as_ref()?[area] {
                        if door.entrance_id == entrance_id {
                            offsets.push(door.map16_offset);
                        }
                    }
                } else {
                    // A special area's entrance; each area leads to at most one.
                    for (transition_area, offset, special_area, _) in SPECIAL_TRANSITIONS {
                        if transition_area == area && special_area != 0 {
                            offsets.push(offset);
                        }
                    }
                }
                if offsets.is_empty() {
                    return None;
                }
                // The center of the matching tiles. Map16 buffer rows are 64 two-byte entries.
                let mut x_sum = 0.0;
                let mut y_sum = 0.0;
                for &offset in &offsets {
                    x_sum += f32::from(offset % 0x80 / 2) + 0.5;
                    y_sum += f32::from(offset / 0x80) + 0.5;
                }
                let count = offsets.len() as f32;
                self.locate_tile(entrance.vertex_idx, x_sum / count, y_sum / count)
                    .map(|point| vec![point])
            }
            _ => None,
        }
    }

    /// Locate an item that sits on the overworld.
    fn locate_item(&self, location_idx: ItemLocationIndex) -> Option<MapPoint> {
        let location = self.catalog.item_location(location_idx);
        for (room_id, item_id, x, y) in OVERWORLD_ITEM_TILES {
            if location.id.room_id == SourceRoomId(room_id) && location.id.item_id == item_id {
                return self.locate_tile(
                    location.vertex_idx,
                    f32::from(x) + 0.5,
                    f32::from(y) + 0.5,
                );
            }
        }
        None
    }
}

/// The direction of travel across a map boundary from a room, given the direction from its
/// first endpoint to its second.
fn get_travel_direction(
    catalog: &LogicCatalog,
    endpoints: &[VertexIndex; 2],
    direction: Direction,
    from_room_idx: RoomIndex,
) -> Direction {
    // A reverse edge starts in the second endpoint's room.
    if catalog.vertex(endpoints[0]).room_idx == from_room_idx {
        return direction;
    }
    match direction {
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::East => Direction::West,
        Direction::West => Direction::East,
    }
}

/// Whether an edge is a warp or starts in an interior.
fn check_indirect(
    catalog: &LogicCatalog,
    connections: &[Connection],
    edge: &TraversalEdge,
) -> bool {
    if catalog
        .vertex_metadata(edge.edge.from_vertex_idx)
        .world
        .is_none()
    {
        return true;
    }
    match edge.origin {
        TraversalEdgeOrigin::Catalog(edge_idx) => matches!(
            catalog.edge_metadata(edge_idx).origin,
            EdgeOrigin::ImplicitMirror | EdgeOrigin::ImplicitFlute
        ),
        TraversalEdgeOrigin::Connection(connection_idx) => matches!(
            connections[connection_idx],
            Connection::Teleport { .. } | Connection::Whirlpool { .. }
        ),
        TraversalEdgeOrigin::SaveAndQuit => true,
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

/// Describe the local state after a route step.
fn describe_state(state: &LocalState) -> RouteState {
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
    RouteState {
        form: format!("{:?}", state.form),
        health_used,
        health_remaining,
        magic_used: state.magic_used,
        bombs_used: state.bombs_used,
        arrows_used: state.arrows_used,
        bottles_used: state.bottles_used,
        followers,
    }
}

/// Return a vertex's room name, using the Dark World name in the Dark World, and its node as
/// "<node ID>: <node name>", with any event it belongs to.
fn describe_vertex(catalog: &LogicCatalog, vertex_idx: VertexIndex) -> (&str, String) {
    let room = catalog.room(catalog.vertex(vertex_idx).room_idx);
    let metadata = catalog.vertex_metadata(vertex_idx);
    let room_name = match (metadata.world, &room.dark_world_name) {
        (Some(World::Dark), Some(dark_world_name)) => dark_world_name,
        _ => &room.name,
    };
    let node_metadata = room.node(metadata.node_idx);
    let mut node = format!("{}: {}", node_metadata.node_id, node_metadata.name);
    if let VertexOrigin::EventEntry { event_idx } | VertexOrigin::EventExit { event_idx } =
        metadata.origin
        && let TransitionState::Event(event) = catalog.get_transition_state(event_idx)
    {
        node += &format!(" [{event}]");
    }
    (room_name, node)
}

/// Describe an edge by the strat name, or a name synthesized from the connection kind, and for
/// an edge entering a room or changing world, by the connection and the room it enters.
fn describe_edge(
    catalog: &LogicCatalog,
    connections: &[Connection],
    edge: &TraversalEdge,
    room_name: &str,
) -> (String, Option<String>) {
    let from_room_idx = catalog.vertex(edge.edge.from_vertex_idx).room_idx;
    let (name, entered) = match edge.origin {
        TraversalEdgeOrigin::Catalog(edge_idx) => match catalog.edge_metadata(edge_idx).origin {
            EdgeOrigin::Strat {
                room_idx,
                strat_idx,
            } => {
                let name = catalog.room(room_idx).strat(strat_idx).name.clone();
                // Strats changing world within a room are portals into the Dark World and Mirror
                // warps back.
                let to_world = catalog.vertex_metadata(edge.edge.to_vertex_idx).world;
                if catalog.vertex_metadata(edge.edge.from_vertex_idx).world == to_world {
                    return (name, None);
                }
                if to_world == Some(World::Dark) {
                    (name, format!("Portal to {room_name}"))
                } else {
                    (name, format!("Mirror to {room_name}"))
                }
            }
            EdgeOrigin::ImplicitMirror => ("Mirror".to_owned(), format!("Mirror to {room_name}")),
            EdgeOrigin::ImplicitFlute => ("Flute".to_owned(), format!("Flute to {room_name}")),
            EdgeOrigin::LakeHyliaBottleExchange => return ("Bottle exchange".to_owned(), None),
        },
        TraversalEdgeOrigin::Connection(connection_idx) => match &connections[connection_idx] {
            Connection::MapBoundary {
                endpoints,
                kind,
                direction,
            } => {
                let direction = get_travel_direction(catalog, endpoints, *direction, from_room_idx);
                match kind {
                    MapBoundaryKind::Ordinary => (
                        format!("Cross {direction:?}"),
                        format!("{direction:?} to {room_name}"),
                    ),
                    MapBoundaryKind::Mirror => (
                        format!("Mirror {direction:?}"),
                        format!("Mirror {direction:?} to {room_name}"),
                    ),
                    MapBoundaryKind::Return => {
                        return (format!("Leave {direction:?} and return"), None);
                    }
                }
            }
            Connection::Entrance {
                kind, entrance_idx, ..
            } => {
                let entrance = catalog.entrance(*entrance_idx);
                let outside = catalog.vertex(entrance.vertex_idx).room_idx == from_room_idx;
                match kind {
                    EntranceKind::Drop => ("Drop".to_owned(), format!("Drop into {room_name}")),
                    EntranceKind::Door if outside => {
                        ("Door".to_owned(), format!("Enter {room_name}"))
                    }
                    EntranceKind::Door => ("Door".to_owned(), format!("Exit to {room_name}")),
                }
            }
            Connection::Teleport { .. } => {
                ("Teleport".to_owned(), format!("Teleport to {room_name}"))
            }
            Connection::Whirlpool { .. } => {
                ("Whirlpool".to_owned(), format!("Whirlpool to {room_name}"))
            }
            Connection::ScreenBoundary { endpoints } => {
                // Each boundary's direction is the side of its own room, so take the source's.
                let mut direction = None;
                for endpoint in endpoints {
                    let boundary = &catalog.screen_boundaries[endpoint.0 as usize];
                    if catalog.vertex(boundary.vertex_idx).room_idx == from_room_idx {
                        direction = Some(boundary.direction);
                    }
                }
                let direction = direction.unwrap();
                (
                    format!("Cross {direction:?}"),
                    format!("{direction:?} to {room_name}"),
                )
            }
        },
        TraversalEdgeOrigin::SaveAndQuit => (
            "Save and quit".to_owned(),
            format!("Save and quit to {room_name}"),
        ),
    };
    (name, Some(entered))
}

/// Collect the progress a requirement reads that earlier exploration, rather than inventory,
/// provides.
fn collect_dependencies(
    catalog: &LogicCatalog,
    replay: &SpoilerReplay,
    requirement: &Requirement,
    dependencies: &mut Vec<Dependency>,
) {
    let (name, target) = match requirement {
        Requirement::And(children) | Requirement::Or(children) => {
            for child in children {
                collect_dependencies(catalog, replay, child, dependencies);
            }
            return;
        }
        Requirement::Flag(idx) => (catalog.flag(*idx).to_owned(), Some(RouteTarget::Flag(*idx))),
        Requirement::Door(idx) => {
            let door = catalog.door(*idx);
            (
                format!("{} - {}", catalog.room(door.room_idx).name, door.name),
                replay.resolve_door(catalog, *idx),
            )
        }
        Requirement::Ocarina(OcarinaState::Active) => {
            ("Ocarina activated".to_owned(), Some(RouteTarget::Ocarina))
        }
        _ => return,
    };
    for dependency in dependencies.iter() {
        if dependency.name == name {
            return;
        }
    }
    let mut target_json = None;
    if let Some(target) = target
        && replay.get_route_state(target).is_some()
    {
        target_json = Some(match target {
            RouteTarget::ItemLocation(idx) => TargetJson::Location(idx.0),
            RouteTarget::Flag(idx) => TargetJson::Flag(idx.0),
            RouteTarget::Door(idx) => TargetJson::Door(idx.0),
            RouteTarget::Ocarina => TargetJson::Ocarina,
        });
    }
    dependencies.push(Dependency {
        name,
        target: target_json,
    });
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
