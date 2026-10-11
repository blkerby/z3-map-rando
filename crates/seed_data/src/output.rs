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
    /// Theme selected for the whole game.
    pub theme_name: String,
    /// Final placements of all pickups, including maps, compasses, keys, and prizes.
    /// Maps and compasses are shuffled within their own dungeon.
    /// Keys and prizes keep their vanilla placements.
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
    /// Provenance only; patching does not require this catalog.
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
