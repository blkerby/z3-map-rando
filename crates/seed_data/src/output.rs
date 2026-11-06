use crate::settings::Settings;
use anyhow::Result;
use common_types::{Item, ItemLocationId};
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter, Read, Write};

/// Current saved-seed format; independent of generator and catalog versions.
pub const SEED_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedData {
    pub format_version: u32,
    pub metadata: SeedMetadata,
    pub settings: Settings,
    pub world_layout: WorldLayout,
    /// Final placements of all pickups, including maps, compasses, keys, prizes, and fairy-exchange items.
    pub item_placements: Vec<ItemPlacement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedMetadata {
    /// Compact, human-readable, alphanumeric random name
    pub seed_name: String,
    /// Shared project release version, independent of Cargo package versions.
    pub generator_version: String,
    /// Full source commit hash, if available with a clean working tree.
    pub generator_commit: Option<String>,
    /// SHA-256 of the complete encoded logic catalog, in hexadecimal.
    /// Provenance only; patching does not require this catalog.
    pub logic_catalog_hash: String,
    /// SHA-256 of the complete encoded retiling catalog, in hexadecimal.
    /// Provenance only; patch preparation uses the current catalog, not this version.
    pub retiling_catalog_hash: String,
    /// RNG seed stored as a string to preserve precision in JSON.
    pub rng_seed: String,
    /// Generation timestamp in RFC 3339 UTC format.
    pub generated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemPlacement {
    /// Permanent authored identity, independent of catalog ordering.
    pub item_location_id: ItemLocationId,
    /// Logical item identity, including the assigned door for a Small Key.
    pub item: Item,
}

/// Persisted area choices, independent of catalog ordering and graphical content.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldLayout {
    pub areas: Vec<AreaLayout>,
    pub rooms: Vec<OverworldRoomPlacement>,
    pub borders: Vec<MapBorder>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverworldRoomPlacement {
    pub room_id: u32,
    pub vanilla_map_id: u8,
    pub position: u8,
    pub size: [u8; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapBorder {
    /// Adjacent physical cells, in ascending order; exterior borders are closed.
    pub cells: [u8; 2],
    pub variant: String,
}

impl WorldLayout {
    pub fn get_area_edges(&self, vanilla_map_id: u8) -> Vec<(String, String)> {
        if vanilla_map_id >= 0x80 {
            return Vec::new();
        }
        let Some(room) = self
            .rooms
            .iter()
            .find(|room| room.vanilla_map_id == vanilla_map_id & 0x3f)
        else {
            return Vec::new();
        };
        let mut edges = Vec::new();
        let x = i16::from(room.position % 8);
        let y = i16::from(room.position / 8);
        let [width, height] = room.size;
        for (side, count) in [
            ("north", width),
            ("south", width),
            ("east", height),
            ("west", height),
        ] {
            for segment in 0..count {
                let segment_offset = i16::from(segment);
                let (cx, cy, nx, ny) = match side {
                    "north" => (x + segment_offset, y, x + segment_offset, y - 1),
                    "south" => (
                        x + segment_offset,
                        y + i16::from(height) - 1,
                        x + segment_offset,
                        y + i16::from(height),
                    ),
                    "east" => (
                        x + i16::from(width) - 1,
                        y + segment_offset,
                        x + i16::from(width),
                        y + segment_offset,
                    ),
                    _ => (x, y + segment_offset, x - 1, y + segment_offset),
                };
                let mut variant = "closed";
                if (0..8).contains(&nx) && (0..8).contains(&ny) {
                    let first = (cy * 8 + cx) as u8;
                    let second = (ny * 8 + nx) as u8;
                    let cells = [first.min(second), first.max(second)];
                    if let Some(border) = self.borders.iter().find(|border| border.cells == cells) {
                        variant = &border.variant;
                    }
                }
                let slot = if count == 1 {
                    side.to_owned()
                } else {
                    format!("{side}-{}", segment + 1)
                };
                edges.push((slot, variant.to_owned()));
            }
        }
        edges
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AreaLayout {
    /// Top-left vanilla map ID, including the world, not a source room ID or
    /// the area's eventual position. Existing IDs are permanent and not reused.
    pub vanilla_map_id: u8,
    /// Authored theme name used as a stable identity, not merely a display name.
    pub theme_name: String,
}

impl SeedData {
    /// Stream pretty JSON through Zstd without buffering the full JSON payload.
    pub fn write(&self, writer: impl Write, compression_level: i32) -> Result<()> {
        let mut encoder = zstd::stream::write::Encoder::new(writer, compression_level)?;
        {
            let mut buffered = BufWriter::new(&mut encoder);
            serde_json::to_writer_pretty(&mut buffered, self)?;
            buffered.flush()?;
        }
        encoder.finish()?;
        Ok(())
    }

    /// Stream Zstd-compressed JSON directly into the saved seed types.
    pub fn read(reader: impl Read) -> Result<Self> {
        let decoder = zstd::stream::read::Decoder::new(reader)?;
        Ok(serde_json::from_reader(BufReader::new(decoder))?)
    }
}
