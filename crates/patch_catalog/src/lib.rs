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
    pub symbols: PatchSymbols,
    pub item_locations: BTreeMap<ItemLocationId, ItemLocation>,
    /// Keys are stable item names from `z3-json-data/items.json`.
    pub items: BTreeMap<String, ItemEncoding>,
}

macro_rules! define_patch_symbols {
    ($visibility:vis struct $name:ident {
        $($field:ident: $field_type:ty => $symbol:ident),* $(,)?
    }) => {
        /// Exported ASM interface consumed by Rust. Addresses are SNES addresses,
        /// not ROM file offsets.
        #[derive(Clone, Debug, Encode, Decode, TypeHash)]
        $visibility struct $name {
            $(pub $field: $field_type,)*
        }

        impl $name {
            /// Build the typed manifest from unprefixed export names, consuming
            /// every export exactly once. This operation runs in the builder.
            pub fn import_symbols(mut symbols: BTreeMap<String, u32>) -> anyhow::Result<Self> {
                let imported = Self {
                    $($field: symbols.remove(stringify!($symbol)).ok_or_else(|| {
                        anyhow::anyhow!("Missing exported symbol: {}", stringify!($symbol))
                    })?,)*
                };
                if !symbols.is_empty() {
                    anyhow::bail!("Unconsumed exported symbols: {:?}", symbols.keys());
                }
                Ok(imported)
            }
        }
    };
}

define_patch_symbols! {
    pub struct PatchSymbols {
        map16_top_left: u32 => Map16TopLeft,
        map16_top_right: u32 => Map16TopRight,
        map16_bottom_left: u32 => Map16BottomLeft,
        map16_bottom_right: u32 => Map16BottomRight,
        map16_property_top_left: u32 => Map16PropertyTopLeft,
        map16_property_top_right: u32 => Map16PropertyTopRight,
        map16_property_bottom_left: u32 => Map16PropertyBottomLeft,
        map16_property_bottom_right: u32 => Map16PropertyBottomRight,
        dynamic_tile_group_pointers: u32 => DynamicTileGroupPointers,
        cutscene_pointers: u32 => CutscenePointers,
        overworld_overlay_pointers: u32 => OverworldOverlayPointers,
    }
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
