//! A compact catalog of the z3-json-data data, processed into the
//! randomizer's graph-oriented format
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_reflection::{Tracer, TracerConfig};
use sha2::{Digest, Sha256};
use std::io::{BufReader, BufWriter, Read, Write};

const CATALOG_MAGIC: [u8; 8] = *b"Z3LOGIC\0";

/// Write the uncompressed envelope followed by a Zstd-compressed Serde bincode payload.
pub fn encode_catalog(
    catalog: &LogicCatalog,
    mut writer: impl Write,
    compression_level: i32,
) -> Result<()> {
    writer.write_all(&CATALOG_MAGIC)?;
    writer.write_all(&compute_schema_hash()?.to_le_bytes())?;
    let mut encoder = zstd::stream::write::Encoder::new(writer, compression_level)?;
    {
        let mut buffered = BufWriter::new(&mut encoder);
        bincode_next::serde::encode_into_std_write(
            catalog,
            &mut buffered,
            bincode_next::config::standard(),
        )?;
        buffered.flush()?;
    }
    encoder.finish()?;
    Ok(())
}

/// Read the envelope, then stream the compressed payload into the catalog.
pub fn decode_catalog(mut reader: impl Read) -> Result<LogicCatalog> {
    let mut envelope = [0; 16];
    reader.read_exact(&mut envelope)?;
    let decoder = zstd::stream::read::Decoder::new(reader)?;
    let mut buffered = BufReader::new(decoder);
    let catalog =
        bincode_next::serde::decode_from_std_read(&mut buffered, bincode_next::config::standard())?;
    Ok(catalog)
}

/// Index into `LogicCatalog::vertices` and `LogicCatalog::vertex_metadata`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VertexIndex(pub u32);

/// Index into `LogicCatalog::edges` and `LogicCatalog::edge_metadata`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EdgeIndex(pub u32);

/// Index into `LogicCatalog::rooms`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RoomIndex(pub u32);

/// Index into `Room::nodes`, local to a room.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeIndex(pub u32);

/// Index into `Room::strats`, matching the source `strats` array.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StratIndex(pub u32);

/// Index into `LogicCatalog::item_locations`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ItemLocationIndex(pub u32);

/// Index into `LogicCatalog::items`; generated door keys are assigned separately.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ItemIndex(pub u32);

/// Index into `LogicCatalog::techs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TechIndex(pub u32);

/// Index into `LogicCatalog::flags`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FlagIndex(pub u32);

/// Index into `LogicCatalog::doors`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DoorIndex(pub u32);

/// Index into `Room::obstacles`, scoped to the edge's source room.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObstacleIndex(pub u32);

/// Index into `LogicCatalog::entrances`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntranceIndex(pub u32);

/// Index into `LogicCatalog::screen_boundaries`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ScreenBoundaryIndex(pub u32);

/// Index into `LogicCatalog::teleports`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TeleportIndex(pub u32);

/// Index into `LogicCatalog::whirlpools`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WhirlpoolIndex(pub u32);

/// Index into `LogicCatalog::events`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EventIndex(pub u32);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogicCatalog {
    pub vertices: Vec<Vertex>,
    /// One metadata record per vertex, in the same order as `vertices`.
    pub vertex_metadata: Vec<VertexMetadata>,
    pub edges: Vec<Edge>,
    /// One metadata record per edge, in the same order as `edges`.
    pub edge_metadata: Vec<EdgeMetadata>,
    pub item_locations: Vec<ItemLocation>,
    /// Light World Link's House. Other restart destinations are deferred.
    pub start_vertex_idx: VertexIndex,
    pub rooms: Vec<Room>,
    pub items: Vec<ItemDefinition>,
    pub techs: Vec<TechDefinition>,
    pub flags: Vec<String>,
    pub doors: Vec<Door>,
    /// Names used to match event-exit and event-entry endpoints across connections.
    pub events: Vec<String>,
    pub entrances: Vec<Entrance>,
    pub screen_boundaries: Vec<ScreenBoundary>,
    pub teleports: Vec<Teleport>,
    pub whirlpools: Vec<Whirlpool>,
    pub flute_spots: Vec<FluteSpot>,
    pub vanilla_connections: Vec<Connection>,
    /// Extra vertices reached only by incoming connections carrying the event.
    pub event_entries: Vec<EventEndpoint>,
    /// Extra vertices whose outgoing edges are only matching room connections.
    pub event_exits: Vec<EventEndpoint>,
}

impl LogicCatalog {
    pub fn vertex(&self, vertex_idx: VertexIndex) -> &Vertex {
        &self.vertices[vertex_idx.0 as usize]
    }

    pub fn vertex_metadata(&self, vertex_idx: VertexIndex) -> &VertexMetadata {
        &self.vertex_metadata[vertex_idx.0 as usize]
    }

    pub fn edge(&self, edge_idx: EdgeIndex) -> &Edge {
        &self.edges[edge_idx.0 as usize]
    }

    pub fn edge_metadata(&self, edge_idx: EdgeIndex) -> &EdgeMetadata {
        &self.edge_metadata[edge_idx.0 as usize]
    }

    pub fn item_location(&self, item_location_idx: ItemLocationIndex) -> &ItemLocation {
        &self.item_locations[item_location_idx.0 as usize]
    }

    pub fn room(&self, room_idx: RoomIndex) -> &Room {
        &self.rooms[room_idx.0 as usize]
    }

    pub fn item(&self, item_idx: ItemIndex) -> &ItemDefinition {
        &self.items[item_idx.0 as usize]
    }

    pub fn tech(&self, tech_idx: TechIndex) -> &TechDefinition {
        &self.techs[tech_idx.0 as usize]
    }

    pub fn flag(&self, flag_idx: FlagIndex) -> &str {
        &self.flags[flag_idx.0 as usize]
    }

    pub fn door(&self, door_idx: DoorIndex) -> &Door {
        &self.doors[door_idx.0 as usize]
    }

    pub fn event(&self, event_idx: EventIndex) -> &str {
        &self.events[event_idx.0 as usize]
    }

    pub fn entrance(&self, entrance_idx: EntranceIndex) -> &Entrance {
        &self.entrances[entrance_idx.0 as usize]
    }
}

/// Authored room identity. Cave, dungeon, house, and special IDs share a namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceRoomId {
    pub namespace: RoomNamespace,
    pub id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RoomNamespace {
    Overworld,
    Underworld,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Room {
    pub room_id: SourceRoomId,
    pub name: String,
    pub kind: RoomKind,
    /// Source overworld grid position and size; absent for interiors.
    pub position: Option<[u8; 2]>,
    pub size: Option<[u16; 2]>,
    pub obstacles: Vec<Obstacle>,
    /// Authored nodes, shared by Light and Dark World vertices in overworld rooms.
    pub nodes: Vec<NodeMetadata>,
    /// Source strats in authored order, including their names.
    pub strats: Vec<StratMetadata>,
}

impl Room {
    pub fn obstacle(&self, obstacle_idx: ObstacleIndex) -> &Obstacle {
        &self.obstacles[obstacle_idx.0 as usize]
    }

    pub fn node(&self, node_idx: NodeIndex) -> &NodeMetadata {
        &self.nodes[node_idx.0 as usize]
    }

    pub fn strat(&self, strat_idx: StratIndex) -> &StratMetadata {
        &self.strats[strat_idx.0 as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomKind {
    Overworld,
    Cave,
    Dungeon,
    House,
    Special,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum World {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Vertex {
    pub room_idx: RoomIndex,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct VertexMetadata {
    /// Index into the vertex's room; event vertices share their endpoint's node.
    pub node_idx: NodeIndex,
    /// Light/Dark for overworld nodes; `None` for all interior vertices.
    pub world: Option<World>,
    pub origin: VertexOrigin,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum VertexOrigin {
    Node,
    EventEntry { event_idx: EventIndex },
    EventExit { event_idx: EventIndex },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeMetadata {
    /// Authored node ID, local to the source room.
    pub node_id: u32,
    pub name: String,
    pub node_type: Option<NodeType>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StratMetadata {
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Door,
    Drop,
    Junction,
    PreBoss,
    Boss,
    Event,
    Fortune,
    Minigame,
    Shop,
}

/// An overworld entrance. Its source ID is local to the vertex's room.
/// Underworld doors/drops use the vertex's room and its metadata's node index.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entrance {
    pub vertex_idx: VertexIndex,
    pub entrance_id: u32,
    pub name: String,
}

/// Default endpoint pairings. Generation expands the selected connections into edges.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Connection {
    Entrance {
        kind: EntranceKind,
        entrance_idx: EntranceIndex,
        interior_vertex_idx: VertexIndex,
    },
    Teleport {
        /// One-way departure from an interior; the destination determines the world.
        from_vertex_idx: VertexIndex,
        teleport_idx: TeleportIndex,
    },
    Whirlpool {
        /// Traversable in both directions.
        endpoints: [WhirlpoolIndex; 2],
    },
    ScreenBoundary {
        /// Traversable in both directions.
        endpoints: [ScreenBoundaryIndex; 2],
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum EntranceKind {
    /// Traversable in both directions.
    Door,
    /// Traversable only from the overworld to the interior.
    Drop,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScreenBoundary {
    pub vertex_idx: VertexIndex,
    pub direction: Direction,
    pub span: [f32; 2],
    pub terrain: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Teleport {
    pub vertex_idx: VertexIndex,
    pub teleport_id: u32,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Whirlpool {
    pub vertex_idx: VertexIndex,
    pub whirlpool_id: u32,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct FluteSpot {
    pub vertex_idx: VertexIndex,
    pub location: u8,
}

/// An extra entry or exit vertex for one event at a physical room endpoint.
/// Its room and metadata's node and world match the ordinary endpoint vertex,
/// but its index is distinct. Generation matches the physical connection and event.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct EventEndpoint {
    pub vertex_idx: VertexIndex,
    pub event_idx: EventIndex,
    /// Overworld entrance; `None` identifies the interior door by the vertex's
    /// room and its metadata's node index.
    pub entrance_idx: Option<EntranceIndex>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from_vertex_idx: VertexIndex,
    pub to_vertex_idx: VertexIndex,
    pub requirement: Requirement,
    pub effects: Box<[Effect]>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct EdgeMetadata {
    pub origin: EdgeOrigin,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum EdgeOrigin {
    Strat {
        room_idx: RoomIndex,
        strat_idx: StratIndex,
    },
    ImplicitMirror,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Requirement {
    Free,
    Never,
    /// Execute children in order, passing successful local states to the next.
    And(Box<[Requirement]>),
    /// Each branch starts with the same input; retain its successful local states.
    Or(Box<[Requirement]>),
    Item(ItemIndex),
    Equipment {
        equipment: Equipment,
        minimum_level: u8,
    },
    /// Expanding dependencies must not remove the named tech's enabled check.
    Tech(TechIndex),
    Proficiency {
        tech_idx: TechIndex,
        minimum: u32,
    },
    Flag(FlagIndex),
    PrizeCount {
        kind: PrizeKind,
        minimum: u8,
    },
    /// Includes the weapon/capability needed to use this ammunition.
    UseAmmo {
        kind: Ammo,
        count: u32,
    },
    /// Apply each use separately, allowing potions between uses.
    /// Item and equipment requirements are compiled separately.
    UseMagic {
        /// Base normalized magic points before magic-upgrade scaling.
        cost_per_use: u32,
        num_uses: u32,
    },
    /// Apply each hit separately, including any intervening Fairy revival.
    Damage {
        /// Damage per hit in eighths of a heart: green, blue, and red mail.
        per_mail: [u32; 3],
        hits: u32,
    },
    Pay(u32),
    Refill {
        resource: Resource,
        /// Health uses eighths of a heart; magic uses base normalized points
        /// before magic-upgrade scaling. Other resources use individual units.
        limit: u32,
    },
    ResourceMissingAtMost {
        resource: Resource,
        /// Same units as `Refill`.
        count: u32,
    },
    Follower(Follower),
    /// Remove the current follower only if it appears in this list.
    LoseFollowers(Box<[Follower]>),
    ObstacleCleared(ObstacleIndex),
    ObstacleNotCleared(ObstacleIndex),
    /// Requires the door to either be already unlocked (by an OpenDoor effect)
    /// or to be unlockable with a key.
    Door(DoorIndex),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Equipment {
    Sword,
    Shield,
    Glove,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrizeKind {
    Pendant,
    Crystal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ammo {
    Arrow,
    SilverArrow,
    Bomb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resource {
    Health,
    Magic,
    Arrows,
    Bombs,
    Rupees,
    BottleContent(BottleContent),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottleContent {
    Fairy,
    Bee,
    GoldBee,
    RedPotion,
    GreenPotion,
    BluePotion,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Follower {
    Zelda,
    OldMan,
    Blind,
    Dwarf,
    PurpleChest,
    SuperBomb,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Effect {
    /// Generation supplies the item placed here; collect each location once.
    CollectItem(ItemLocationIndex),
    /// Fixed event such as flute activation; not a randomized placement slot.
    GrantItem(ItemIndex),
    SetFlag(FlagIndex),
    ClearObstacle(ObstacleIndex),
    ResetObstacle(ObstacleIndex),
    /// Persistent unlocking of a non-key door by a successful strat.
    OpenDoor(DoorIndex),
    SetFollower(Option<Follower>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemLocation {
    pub name: String,
    /// Source `itemLocation` node, resolved in the item's world for overworld items.
    /// Collection is performed by a strat's `CollectItem` effect.
    pub vertex_idx: VertexIndex,
    /// Unheadered ROM file offsets. Empty when the source address is unknown.
    pub rom_addresses: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemDefinition {
    pub name: String,
    pub receipt_id: u8,
    pub prize_patch_bytes: Option<[u8; 6]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TechDefinition {
    /// Stable authored ID used in player tech settings.
    pub tech_id: u32,
    pub name: String,
    /// Empty for techs controlled by a Boolean setting.
    pub tiers: Vec<ProficiencyTier>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProficiencyTier {
    pub value: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Door {
    pub room_idx: RoomIndex,
    pub door_id: u32,
    pub name: String,
    pub world: Option<World>,
    pub lock: LockKind,
}

/// Source lock classification. Key assignment and ROM door binding are deferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockKind {
    SmallKey,
    BigKey,
    Bomb,
    BombOrBoots,
    Boots,
    Glove,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Obstacle {
    /// Authored room-local obstacle identity.
    pub name: String,
}

/// Hash the complete reflected schema, including enum names, tags, and payloads.
/// Recursive requirements use named schema references rather than infinite trees.
/// The catalog envelope uses the first eight SHA-256 bytes as a little-endian u64.
pub fn compute_schema_hash() -> Result<u64> {
    // Reflection errors are not Send + Sync; convert them once for anyhow.
    let registry = (|| {
        let mut tracer = Tracer::new(TracerConfig::default());
        tracer.trace_simple_type::<LogicCatalog>()?;
        // Each enum must be traced explicitly to discover all of its variants,
        // including those not encountered while tracing the root's first variants.
        tracer.trace_simple_type::<RoomNamespace>()?;
        tracer.trace_simple_type::<RoomKind>()?;
        tracer.trace_simple_type::<World>()?;
        tracer.trace_simple_type::<VertexOrigin>()?;
        tracer.trace_simple_type::<NodeType>()?;
        tracer.trace_simple_type::<Direction>()?;
        tracer.trace_simple_type::<Connection>()?;
        tracer.trace_simple_type::<EntranceKind>()?;
        tracer.trace_simple_type::<EdgeOrigin>()?;
        tracer.trace_simple_type::<Requirement>()?;
        tracer.trace_simple_type::<Equipment>()?;
        tracer.trace_simple_type::<PrizeKind>()?;
        tracer.trace_simple_type::<Ammo>()?;
        tracer.trace_simple_type::<Resource>()?;
        tracer.trace_simple_type::<BottleContent>()?;
        tracer.trace_simple_type::<Follower>()?;
        tracer.trace_simple_type::<Effect>()?;
        tracer.trace_simple_type::<LockKind>()?;

        tracer.registry()
    })()
    .map_err(|error| anyhow::anyhow!("{error}"))?;
    // Registry maps are ordered; vectors retain field and variant declaration
    // order. Encode the schema with a fixed configuration before hashing it.
    let bytes = bincode_next::serde::encode_to_vec(&registry, bincode_next::config::standard())?;
    let digest = Sha256::digest(bytes);
    let mut hash = [0; 8];
    hash.copy_from_slice(&digest[..8]);
    Ok(u64::from_le_bytes(hash))
}
