use crate::settings::Settings;
use common_types::ItemLocationId;
use serde::{Deserialize, Serialize};

/// Current saved-seed format; independent of generator and catalog versions.
pub const SEED_FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedData {
    pub format_version: u32,
    pub metadata: SeedMetadata,
    pub settings: Settings,
    pub world_layout: WorldLayout,
    /// Final placements of all pickups, including maps, compasses, keys, and prizes.
    /// Maps and compasses are shuffled within their own dungeon.
    /// Keys keep their vanilla placements; pendants and crystals shuffle among prize locations.
    pub item_placements: Vec<ItemPlacement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedMetadata {
    /// Compact, human-readable, alphanumeric random name
    pub seed_name: String,
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
    /// Item identity from items.json.
    pub item_name: String,
}

/// Persisted area choices, independent of catalog ordering and graphical content.
/// Area positions and connections remain vanilla for the prototype.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldLayout {
    pub areas: Vec<AreaLayout>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AreaLayout {
    /// Top-left vanilla map ID, including the world, not a source room ID or
    /// the area's eventual position. Existing IDs are permanent and not reused.
    pub vanilla_map_id: u8,
    /// Authored theme name used as a stable identity, not merely a display name.
    pub theme_name: String,
}
