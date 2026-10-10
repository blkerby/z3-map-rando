//! Patch catalog containing seed-independent patching data, including
//! - IPS patches pre-built from ASM
//! - item data extracted from z3-json-data.

use bincode_next::{Decode, Encode};
use std::collections::BTreeMap;
use type_hash::TypeHash;

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct PatchCatalog {
    /// Keys are ASM filenames without their extension; values are complete IPS files.
    /// Patches remain separate so the patcher can detect conflicting writes.
    pub patches: BTreeMap<String, Vec<u8>>,
    /// Exported Asar symbol names without the leading `!`, with their numeric values.
    /// Address symbols use SNES addresses, not ROM file offsets. Values may also
    /// represent RAM/VRAM addresses or constants, according to the ASM interface.
    pub symbols: BTreeMap<String, u32>,
    pub item_locations: BTreeMap<ItemLocationId, ItemLocation>,
    /// Keys are stable item names from `z3-json-data/items.json`.
    pub items: BTreeMap<String, ItemEncoding>,
}

/// Authored identity, independent of catalog ordering and patching addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, TypeHash)]
pub struct ItemLocationId {
    pub room_id: SourceRoomId,
    /// ID of the room's item entry, not its `itemLocation` logic node.
    pub item_id: u32,
}

/// Overworld rooms have a separate ID namespace from all interior room kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Encode, Decode, TypeHash)]
pub enum SourceRoomId {
    Overworld(u32),
    Underworld(u32),
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct ItemLocation {
    /// Unheadered ROM file offsets extracted from the z3-json-data `itemAddress`.
    /// Ordinary pickups receive the item's receipt ID as a single address.
    /// Dungeon prizes have six offsets, in the same order as
    /// `ItemEncoding::prize_patch_bytes`. Empty when the source address is unknown.
    pub rom_addresses: Vec<u32>,
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct ItemEncoding {
    pub receipt_id: u8,
    /// Dungeon-prize bytes in source order, paired with the location's six offsets.
    pub prize_patch_bytes: Option<[u8; 6]>,
}
