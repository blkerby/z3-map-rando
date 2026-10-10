use crate::settings::Settings;
use logic_catalog::ItemLocationIndex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedData {
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
    pub generator_version: String,
    /// Full source commit hash, if available with a clean working tree.
    pub generator_commit: Option<String>,
    /// SHA-256 of the complete encoded logic catalog, in hexadecimal.
    pub logic_catalog_hash: String,
    /// RNG seed stored as a string to preserve precision in JSON.
    pub rng_seed: String,
    /// Generation timestamp in RFC 3339 UTC format.
    pub generated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemPlacement {
    /// Index into the exact logic catalog identified by metadata.
    pub item_location_idx: ItemLocationIndex,
    /// Item identity from items.json.
    pub item_name: String,
}
