use crate::{Settings, SettingsDocument};
use anyhow::Result;
use common_types::{Item, SourceRoomId};
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter, Read, Write};

/// Saved room choices and typed placements allow patching with the current catalogs
/// without the map pool or generation RNG.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedData {
    pub metadata: SeedMetadata,
    /// Authoritative resolved settings.
    pub settings: Settings,
    /// Public sparse document that the resolved settings came from.
    pub settings_document: SettingsDocument,
    /// Rooms with any saved choice, in ascending room ID order.
    pub rooms: Vec<RoomData>,
    /// Open borders between adjacent map cells of different rooms; any other border is closed.
    /// Empty for the vanilla layout.
    pub borders: Vec<MapBorder>,
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
pub struct RoomData {
    pub room_id: SourceRoomId,
    /// Top-left map cell as [x, y], for overworld rooms in a randomized layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[u8; 2]>,
    /// Retiling theme, for rooms drawn as overworld areas. Overworld rooms use it in both worlds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// Final placements of all the room's locations, including empty slots, dungeon items,
    /// prizes, and Great Fairy upgrades.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<ItemPlacement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemPlacement {
    /// The room's item entry ID, a permanent authored identity.
    pub item_id: u32,
    /// Logical item identity, Nothing for an empty slot, or the assigned door for a Small Key.
    pub item: Item,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapBorder {
    /// Map cell as [x, y] on the west or north side of the border.
    pub position: [u8; 2],
    pub side: BorderSide,
    pub variant: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BorderSide {
    East,
    South,
}

impl SeedData {
    /// Write compact JSON; storage transports own compression.
    pub fn write(&self, writer: impl Write) -> Result<()> {
        let mut buffered = BufWriter::new(writer);
        serde_json::to_writer(&mut buffered, self)?;
        buffered.flush()?;
        Ok(())
    }

    /// Read the saved-seed JSON format.
    pub fn read(reader: impl Read) -> Result<Self> {
        Ok(serde_json::from_reader(BufReader::new(reader))?)
    }
}
