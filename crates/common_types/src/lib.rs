//! Common domain types shared by catalogs, generation, and patching.

mod item;

pub use item::{Item, ItemKind};

use serde::{Deserialize, Serialize};

/// Check Frog/Dwarf permission for an ordinary native ROM door entrance.
/// IDs below $3C are the first 59 forbidden door-table entries. Drops and
/// special overworld transitions bypass this check.
pub fn allows_frog_dwarf(rom_entrance_id: u8) -> bool {
    rom_entrance_id >= 0x3c
}

/// Authored room identity, unique across overworld and interior rooms.
/// Existing IDs are permanent; removed IDs must not be reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceRoomId(pub u32);

/// Retiling theme IDs that generation can save for rooms drawn as overworld areas.
/// Each must exist in the retiling catalog for every area with a vanilla map ID.
pub const SUPPORTED_THEMES: [&str; 1] = ["desert"];

/// Special overworld areas without a grid position, as (room ID, area, parent overworld
/// room ID). A special area takes the theme of the overworld room entered from.
/// Sacred Grove and Under the Bridge share $80, which uses Sacred Grove's parent theme until
/// the two parts can be themed separately (plans/splitting_area_80.md).
pub const SPECIAL_ROOM_AREAS: [(SourceRoomId, u8, SourceRoomId); 3] = [
    (SourceRoomId(142), 0x80, SourceRoomId(1)),
    (SourceRoomId(143), 0x80, SourceRoomId(28)),
    (SourceRoomId(144), 0x81, SourceRoomId(7)),
];

/// Authored identity, independent of catalog ordering and patching addresses.
/// Existing IDs are permanent; removed IDs must not be reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ItemLocationId {
    pub room_id: SourceRoomId,
    /// ID of the room's item entry, not its `itemLocation` logic node.
    pub item_id: u32,
}

/// The Waterfall of Wishing and Pyramid fairies' upgrade locations, in the order
/// of the `FairyItems` ROM table.
pub const FAIRY_UPGRADE_LOCATIONS: [ItemLocationId; 2] = [
    ItemLocationId {
        room_id: SourceRoomId(52),
        item_id: 1,
    },
    ItemLocationId {
        room_id: SourceRoomId(109),
        item_id: 1,
    },
];

/// Authored door identity, independent of catalog ordering and ROM addresses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DoorId {
    pub room_id: SourceRoomId,
    pub door_id: u32,
}
