//! A compact catalog of the ALTTPRetiling data, sanitized to reference
//! vanilla tile graphics rather than contain a copy of them.

use anyhow::{Result, ensure};
use bincode_next::{Decode, Encode};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io::{BufReader, BufWriter, Read, Write},
};
use type_hash::TypeHash;

const CATALOG_MAGIC: [u8; 8] = *b"Z3RETILE";
const CATALOG_CONFIG: bincode_next::config::Configuration = bincode_next::config::standard();

/// Write the uncompressed envelope followed by a Zstd-compressed bincode payload.
pub fn encode_catalog(
    catalog: &RetilingCatalog,
    mut writer: impl Write,
    compression_level: i32,
) -> Result<()> {
    writer.write_all(&CATALOG_MAGIC)?;
    writer.write_all(&RetilingCatalog::type_hash().to_le_bytes())?;
    let mut encoder = zstd::stream::write::Encoder::new(writer, compression_level)?;
    {
        let mut buffered = BufWriter::new(&mut encoder);
        bincode_next::encode_into_std_write(catalog, &mut buffered, CATALOG_CONFIG)?;
        buffered.flush()?;
    }
    encoder.finish()?;
    Ok(())
}

/// Check the envelope, then stream the compressed payload into the catalog.
pub fn decode_catalog(mut reader: impl Read) -> Result<RetilingCatalog> {
    let mut magic = [0; 8];
    reader.read_exact(&mut magic)?;
    ensure!(
        magic == CATALOG_MAGIC,
        "invalid retiling catalog magic bytes"
    );
    let mut hash = [0; 8];
    reader.read_exact(&mut hash)?;
    ensure!(
        u64::from_le_bytes(hash) == RetilingCatalog::type_hash(),
        "retiling catalog type hash mismatch; rebuild the catalog with this version"
    );
    let decoder = zstd::stream::read::Decoder::new(reader)?;
    let mut buffered = BufReader::new(decoder);
    let catalog = bincode_next::decode_from_std_read(&mut buffered, CATALOG_CONFIG)?;
    Ok(catalog)
}

/// External palette ID assigned by the editor (not a SNES palette slot).
pub type PaletteId = u16;
/// Index into a palette's tile list.
pub type TileId = u16;
/// Red, green, and blue components, each in the range 0..=31.
pub type Color = [u8; 3];

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct RetilingCatalog {
    pub palettes: BTreeMap<PaletteId, Palette>,
    /// Keys are source area names, including non-map templates such as Tree Edges.
    pub areas: BTreeMap<String, Area>,
    pub dynamic_tile_groups: Vec<DynamicTileGroup>,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct Palette {
    /// Source palette filename without its extension.
    pub name: String,
    /// Authored color indexes are preserved, including duplicate RGB colors.
    pub colors: [Color; 16],
    /// Array position is the tile ID used by placements and animation groups.
    pub tiles: Vec<Tile>,
    pub animated_tile_groups: Vec<AnimatedTileGroup>,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct Tile {
    pub graphic: TileGraphic,
    pub priority: bool,
    /// Authored 8x8 collision/property byte, before applying placement flips.
    pub collision: u8,
    pub h_flippable: bool,
    pub v_flippable: bool,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
#[repr(u8)]
pub enum TileGraphic {
    /// Row-major authored palette indexes 0..=15; index zero is transparent.
    Custom {
        pixels: [[u8; 8]; 8],
    } = 0,
    Vanilla(VanillaTileReference) = 1,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct VanillaTileReference {
    /// Numbered vanilla background graphics sheet, in $00..=$60.
    pub sheet: u8,
    /// Tile offset within the sheet, in 0..64.
    pub tile: u8,
    /// Map from canonical color indexes to exact authored palette indexes.
    /// Canonical index zero is the first encountered color, not transparency.
    pub color_indexes: Vec<u8>,
    /// Applied after canonicalizing the ROM tile and mapping its color indexes.
    /// This reconstructs the authored graphic; placement flips apply separately.
    pub flip: Flip,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct AnimatedTileGroup {
    /// First of the 16 consecutive palette-local tiles replaced by each frame.
    /// Priority and collision come from those tile definitions.
    pub base_tile: TileId,
    /// Additional frames after the initial frame stored in the palette's tiles.
    pub frames: Vec<[TileGraphic; 16]>,
    /// Number of game frames to hold each animation frame.
    pub frame_hold: u16,
    /// Initial offset measured in game frames.
    pub phase_offset: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, TypeHash)]
pub struct TilePlacement {
    /// Horizontal position in 8x8 tiles from the grid's top-left corner.
    pub x: u8,
    /// Vertical position in 8x8 tiles from the grid's top-left corner.
    pub y: u8,
    pub palette: PaletteId,
    pub tile: TileId,
    pub flip: Flip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, TypeHash)]
#[repr(u8)]
pub enum Flip {
    None = 0,
    Horizontal = 1,
    Vertical = 2,
    Both = 3,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct Area {
    /// Top-left vanilla map ID; absent for editor templates without a ROM map.
    pub vanilla_map_id: Option<u8>,
    /// Keys are source theme names, such as Base and Desert.
    pub themes: BTreeMap<String, AreaTheme>,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct AreaTheme {
    pub background: BackgroundSettings,
    /// Source layer order is retained for compositing by the asset compiler.
    /// Every layer grid retains the full area dimensions with sparse placements.
    pub layers: Vec<Layer>,
    pub cutscenes: Vec<Cutscene>,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct BackgroundSettings {
    pub color: Color,
    pub layering: BackgroundLayering,
    /// X and Y camera-follow factors.
    pub camera_follow: [f32; 2],
    /// X and Y background drift, in pixels per game frame.
    pub camera_drift: [f32; 2],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Encode, Decode, TypeHash)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum BackgroundLayering {
    #[default]
    None = 0,
    HalfAdd = 1,
    Backdrop = 2,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct Layer {
    /// Used by cutscene draw actions and by the compiler's event interpretation.
    pub name: String,
    pub background: Background,
    pub grid: TileGrid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Encode, Decode, TypeHash)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Background {
    Bg1 = 0,
    Bg2 = 1,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct TileGrid {
    /// Width and height in 8x8 tiles, rather than editor screens or pixels.
    pub width: u8,
    pub height: u8,
    /// Sparse placements. Omitted positions are empty; layer compositing skips them.
    pub tiles: Vec<TilePlacement>,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct DynamicTileGroup {
    pub kind: DynamicTileType,
    pub variants: Vec<DynamicTileVariant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Encode, Decode, TypeHash)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum DynamicTileType {
    CutGrass = 0,
    DigTerrain = 1,
    GreenBush = 2,
    HeavyBush = 3,
    HammerPeg = 4,
    LiftSign = 5,
    SmallGrayRock = 6,
    SmallBlackRock = 7,
    LargeGrayRock = 8,
    LargeBlackRock = 9,
    RockPile = 10,
    SecretHole = 11,
    SecretPortal = 12,
    SecretBombableEntrance = 13,
    SecretStairs = 14,
    WoodenDoor = 15,
    SanctuaryDoor = 16,
    HyruleCastleDoor = 17,
    GraveCorpse = 18,
    GraveStairs = 19,
    GravePit = 20,
    HyruleCastleGate = 21,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct DynamicTileVariant {
    pub before: TileGrid,
    pub after_frames: Vec<TileGrid>,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct Cutscene {
    pub event: CutsceneEvent,
    pub actions: Vec<CutsceneAction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Encode, Decode, TypeHash)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum CutsceneEvent {
    PalaceOfDarknessEntranceOpened = 1,
    SkullWoodsEntranceOpened = 2,
    MiseryMireEntranceOpened = 3,
    TurtleRockEntranceOpened = 4,
    GanonsTowerEntranceOpened = 5,
}

#[derive(Clone, Debug, Deserialize, Encode, Decode, TypeHash)]
#[serde(tag = "action", rename_all = "snake_case")]
#[repr(u8)]
pub enum CutsceneAction {
    Wait {
        frames: u8,
    } = 0,
    PlaySound {
        channel: u8,
        sound: u8,
    } = 1,
    PlayMusic {
        song: u8,
    } = 2,
    /// Name of a layer in the enclosing area theme.
    Draw {
        layer: String,
    } = 3,
    SetComplete = 4,
    StartShake = 5,
    StopShake = 6,
    End = 7,
}
