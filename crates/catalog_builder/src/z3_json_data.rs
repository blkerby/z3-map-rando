use anyhow::{Context, Result};
use logic_catalog::{Direction, NodeType, ProficiencyTier, RoomKind, World};
use serde::{Deserialize, de::DeserializeOwned};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn read_source<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn collect_source_files(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
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

#[derive(Deserialize)]
pub struct Connections {
    pub connections: Vec<Connection>,
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum Connection {
    Door {
        world: SourceWorld,
        overworld: EntranceEndpoint,
        underworld: InteriorEndpoint,
    },
    Drop {
        world: SourceWorld,
        overworld: EntranceEndpoint,
        underworld: InteriorEndpoint,
    },
    Teleport {
        to_world: SourceWorld,
        underworld: InteriorEndpoint,
        overworld: TeleportEndpoint,
    },
    Whirlpool {
        world: SourceWorld,
        overworld: WhirlpoolEndpoint,
        overworld2: WhirlpoolEndpoint,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteriorEndpoint {
    pub room_id: u32,
    pub node_id: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntranceEndpoint {
    pub room_id: u32,
    pub entrance_id: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeleportEndpoint {
    pub room_id: u32,
    pub teleport_id: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhirlpoolEndpoint {
    pub room_id: u32,
    pub whirlpool_id: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    pub id: u32,
    pub name: String,
    pub room_type: RoomKind,
    pub position: Option<[u8; 2]>,
    pub size: Option<[u16; 2]>,
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub items: Vec<ItemLocation>,
    #[serde(default)]
    pub locked_doors: Vec<Door>,
    #[serde(default)]
    pub obstacles: Vec<Obstacle>,
    pub strats: Vec<Strat>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeWorld {
    Light,
    Dark,
    Both,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceWorld {
    Light,
    Dark,
}

impl SourceWorld {
    pub fn get_world(self) -> World {
        match self {
            Self::Light => World::Light,
            Self::Dark => World::Dark,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: u32,
    pub name: String,
    pub node_type: Option<SourceNodeType>,
    pub world: Option<NodeWorld>,
    pub spawn_point: Option<String>,
    #[serde(default)]
    pub entrances: Vec<Endpoint>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
    #[serde(default)]
    pub teleports: Vec<Endpoint>,
    #[serde(default)]
    pub whirlpools: Vec<Endpoint>,
    pub flute_location: Option<u8>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceNodeType {
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

impl SourceNodeType {
    pub fn get_node_type(self) -> NodeType {
        match self {
            Self::Door => NodeType::Door,
            Self::Drop => NodeType::Drop,
            Self::Junction => NodeType::Junction,
            Self::PreBoss => NodeType::PreBoss,
            Self::Boss => NodeType::Boss,
            Self::Event => NodeType::Event,
            Self::Fortune => NodeType::Fortune,
            Self::Minigame => NodeType::Minigame,
            Self::Shop => NodeType::Shop,
        }
    }
}

#[derive(Deserialize)]
pub struct Endpoint {
    pub id: u32,
    pub name: String,
    pub world: SourceWorld,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceDirection {
    North,
    South,
    East,
    West,
}

impl SourceDirection {
    pub fn get_direction(self) -> Direction {
        match self {
            Self::North => Direction::North,
            Self::South => Direction::South,
            Self::East => Direction::East,
            Self::West => Direction::West,
        }
    }
}

#[derive(Deserialize)]
pub struct Transition {
    pub edge: SourceDirection,
    pub span: [f32; 2],
    pub terrain: Option<String>,
    pub world: Option<SourceWorld>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemLocation {
    pub id: u32,
    pub location_name: String,
    pub item_location: u32,
    pub item: String,
    pub item_address: Addresses,
    pub world: Option<SourceWorld>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Addresses {
    One(String),
    Many(Vec<String>),
}

#[derive(Clone, Copy, Deserialize)]
pub enum KeyType {
    #[serde(rename = "small")]
    Small,
    #[serde(rename = "big")]
    Big,
    #[serde(rename = "bomb")]
    Bomb,
    #[serde(rename = "bomb, boots")]
    BombOrBoots,
    #[serde(rename = "boots")]
    Boots,
    #[serde(rename = "glove")]
    Glove,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Door {
    pub id: u32,
    pub location_name: String,
    pub key_type: KeyType,
    pub world: Option<SourceWorld>,
}

#[derive(Deserialize)]
pub struct Obstacle {
    pub id: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StratWorld {
    Light,
    Dark,
    Any,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Bunny {
    Yes,
    #[default]
    No,
    Any,
}

#[derive(Clone, Copy, Deserialize)]
pub enum Follower {
    None,
    Zelda,
    #[serde(rename = "Old Man")]
    OldMan,
    Blind,
    Dwarf,
    #[serde(rename = "Purple Chest")]
    PurpleChest,
    #[serde(rename = "Super Bomb")]
    SuperBomb,
}

impl Follower {
    pub fn get_follower(self) -> Option<logic_catalog::Follower> {
        use logic_catalog::Follower as F;
        match self {
            Self::None => None,
            Self::Zelda => Some(F::Zelda),
            Self::OldMan => Some(F::OldMan),
            Self::Blind => Some(F::Blind),
            Self::Dwarf => Some(F::Dwarf),
            Self::PurpleChest => Some(F::PurpleChest),
            Self::SuperBomb => Some(F::SuperBomb),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Strat {
    pub link: [u32; 2],
    pub name: String,
    pub requires: Vec<Requirement>,
    #[serde(default)]
    pub collects_items: Vec<u32>,
    #[serde(default)]
    pub sets_flags: Vec<String>,
    #[serde(default)]
    pub clears_obstacles: Vec<String>,
    #[serde(default)]
    pub resets_obstacles: Vec<String>,
    #[serde(default)]
    pub unlocks_door: Vec<u32>,
    pub world: Option<StratWorld>,
    pub from_world: Option<SourceWorld>,
    pub to_world: Option<SourceWorld>,
    #[serde(default)]
    pub is_bunny: Bunny,
    pub sets_follower: Option<Follower>,
    pub follower_complete: Option<Follower>,
    pub entrance_state: Option<Event>,
    pub exit_state: Option<Event>,
}

#[derive(Deserialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub name: String,
    #[serde(rename = "entranceID")]
    pub entrance_id: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Items {
    pub inventory: Vec<Item>,
    pub refills: Vec<Item>,
    pub bottle_contents: Vec<Item>,
    pub currency: Vec<Item>,
    pub dungeon_items: Vec<Item>,
    pub dungeon_prizes: Vec<Item>,
    pub goal_items: Vec<Item>,
    pub expansions: Vec<Item>,
    pub flags: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub name: String,
    pub item_receipt_id: String,
    pub prize_patch_bytes: Option<[String; 6]>,
}

#[derive(Deserialize)]
pub struct Helper {
    pub name: String,
    pub requires: Vec<Requirement>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Techs {
    pub tech_categories: Vec<TechCategory>,
}

#[derive(Deserialize)]
pub struct TechCategory {
    pub techs: Vec<Tech>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tech {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub tiers: Vec<ProficiencyTier>,
    #[serde(default)]
    pub tech_requires: Vec<Requirement>,
    #[serde(default)]
    pub other_requires: Vec<Requirement>,
    #[serde(default)]
    pub extension_techs: Vec<Tech>,
}

#[derive(Deserialize)]
pub struct Enemies {
    pub enemies: Vec<Enemy>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Enemy {
    pub names: Vec<String>,
    pub dmg_to_link: DamageByMail,
}

#[derive(Deserialize)]
pub struct DamageByMail {
    pub green: u32,
    pub blue: u32,
    pub red: u32,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Requirement {
    Named(String),
    Operation(Operation),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Operation {
    And(Vec<Requirement>),
    Or(Vec<Requirement>),
    Sword(u8),
    SwordExact(u8),
    Shield(u8),
    ShieldExact(u8),
    Glove(u8),
    Arrows(u32),
    SilverArrows(u32),
    Bombs(u32),
    MagicPowder(u32),
    FireRod(u32),
    IceRod(u32),
    Rod(u32),
    Bombos(u32),
    Ether(u32),
    Quake(u32),
    Lamp(u32),
    RedCane(u32),
    BlueCane(u32),
    Cape(u32),
    Damage(Damage),
    Refill(Refill),
    CombatProficiency(u32),
    BossProficiency(u32),
    DarkProficiency(u32),
    Pendants(u8),
    Crystals(u8),
    UnlockDoor(u32),
    Pay(u32),
    Flag(String),
    NotFlag(String),
    Follower(Follower),
    FollowerLost(Vec<Follower>),
    ObstaclesCleared(Vec<String>),
    ObstaclesNotCleared(Vec<String>),
    ResourceMissingAtMost(Vec<MissingResource>),
}

#[derive(Deserialize)]
pub struct Damage {
    pub enemy: String,
    pub attack: Option<String>,
    pub count: u32,
}

#[derive(Clone, Copy, Deserialize)]
pub enum Resource {
    Health,
    Magic,
    Arrows,
    Bombs,
    Rupee,
    Fairy,
    Bee,
    GoldBee,
    RedPotion,
    GreenPotion,
    BluePotion,
}

impl Resource {
    pub fn get_resource(self) -> logic_catalog::Resource {
        use logic_catalog::{BottleContent as B, Resource as R};
        match self {
            Self::Health => R::Health,
            Self::Magic => R::Magic,
            Self::Arrows => R::Arrows,
            Self::Bombs => R::Bombs,
            Self::Rupee => R::Rupees,
            Self::Fairy => R::BottleContent(B::Fairy),
            Self::Bee => R::BottleContent(B::Bee),
            Self::GoldBee => R::BottleContent(B::GoldBee),
            Self::RedPotion => R::BottleContent(B::RedPotion),
            Self::GreenPotion => R::BottleContent(B::GreenPotion),
            Self::BluePotion => R::BottleContent(B::BluePotion),
        }
    }
}

#[derive(Deserialize)]
pub struct Refill {
    #[serde(rename = "type")]
    pub resource: Resource,
    pub limit: u32,
}

#[derive(Deserialize)]
pub struct MissingResource {
    #[serde(rename = "type")]
    pub resource: Resource,
    pub count: u32,
}
