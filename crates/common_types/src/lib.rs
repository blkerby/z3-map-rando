//! Common domain types shared by catalogs, generation, and patching.

use serde::{Deserialize, Serialize};
use type_hash::TypeHash;

/// Shared project release version:
pub const VERSION: &str = include_str!("../../../VERSION").trim_ascii();

/// Authored room identity. Overworld IDs are separate from all interior room kinds.
/// Existing IDs are permanent; removed IDs must not be reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TypeHash)]
pub enum SourceRoomId {
    Overworld(u32),
    Underworld(u32),
}

/// Authored identity, independent of catalog ordering and patching addresses.
/// Existing IDs are permanent; removed IDs must not be reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TypeHash)]
pub struct ItemLocationId {
    pub room_id: SourceRoomId,
    /// ID of the room's item entry, not its `itemLocation` logic node.
    pub item_id: u32,
}
