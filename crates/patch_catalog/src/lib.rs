//! Patch catalog containing seed-independent patching data, including
//! - IPS patches pre-built from ASM
//! - item data extracted from z3-json-data.

use bincode_next::{Decode, Encode};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    io::{BufReader, BufWriter, Read, Write},
};
use type_hash::TypeHash;

const CATALOG_MAGIC: [u8; 8] = *b"Z3PATCH\0";
const CATALOG_CONFIG: bincode_next::config::Configuration = bincode_next::config::standard();

/// Write the uncompressed envelope followed by a Zstd-compressed bincode payload.
pub fn encode_catalog(
    catalog: &PatchCatalog,
    mut writer: impl Write,
    compression_level: i32,
) -> anyhow::Result<()> {
    writer.write_all(&CATALOG_MAGIC)?;
    writer.write_all(&PatchCatalog::type_hash().to_le_bytes())?;
    let mut encoder = zstd::stream::write::Encoder::new(writer, compression_level)?;
    {
        let mut buffered = BufWriter::new(&mut encoder);
        bincode_next::encode_into_std_write(catalog, &mut buffered, CATALOG_CONFIG)?;
        buffered.flush()?;
    }
    encoder.finish()?;
    Ok(())
}

/// Read the envelope, then stream the compressed payload into the catalog.
pub fn decode_catalog(mut reader: impl Read) -> anyhow::Result<PatchCatalog> {
    let mut envelope = [0; 16];
    reader.read_exact(&mut envelope)?;
    let decoder = zstd::stream::read::Decoder::new(reader)?;
    let mut buffered = BufReader::new(decoder);
    let catalog = bincode_next::decode_from_std_read(&mut buffered, CATALOG_CONFIG)?;
    Ok(catalog)
}

#[derive(Clone, Debug, Encode, Decode, TypeHash)]
pub struct PatchCatalog {
    pub patches: PatchIps,
    pub symbols: PatchSymbols,
    pub item_locations: BTreeMap<ItemLocationId, ItemLocation>,
    /// Keys are stable item names from `z3-json-data/items.json`.
    pub items: BTreeMap<String, ItemEncoding>,
}

/// Complete IPS payloads, named after their ASM filename stems.
/// Patches remain separate so the patcher can detect conflicting writes.
#[derive(Clone, Debug, Deserialize, Encode, Decode, TypeHash)]
#[serde(deny_unknown_fields)]
pub struct PatchIps {
    pub bg3_tilemap: Vec<u8>,
    pub fastrom_base: Vec<u8>,
    pub fastrom_extra: Vec<u8>,
    pub mirror_bg1_hdma: Vec<u8>,
    pub nmi_optimize: Vec<u8>,
    pub overworld_animations: Vec<u8>,
    pub overworld_assets: Vec<u8>,
    pub overworld_bg_color: Vec<u8>,
    pub overworld_bg_tilemaps: Vec<u8>,
    pub overworld_cutscenes: Vec<u8>,
    pub overworld_dynamic_tiles: Vec<u8>,
    pub overworld_entrances: Vec<u8>,
    pub overworld_lightning: Vec<u8>,
    pub overworld_map16_graphics: Vec<u8>,
    pub overworld_map16_properties: Vec<u8>,
    pub overworld_map_data: Vec<u8>,
    pub overworld_vram: Vec<u8>,
    pub rom_size: Vec<u8>,
}

/// Exported ASM interface consumed by Rust. Addresses are SNES addresses,
/// not ROM file offsets. Field names match the explicit ASM export names.
#[derive(Clone, Debug, Deserialize, Encode, Decode, TypeHash)]
#[serde(deny_unknown_fields)]
pub struct PatchSymbols {
    pub map16_top_left: u32,
    pub map16_top_right: u32,
    pub map16_bottom_left: u32,
    pub map16_bottom_right: u32,
    pub map16_property_top_left: u32,
    pub map16_property_top_right: u32,
    pub map16_property_bottom_left: u32,
    pub map16_property_bottom_right: u32,
    pub dynamic_tile_group_pointers: u32,
    pub cutscene_pointers: u32,
    pub overworld_overlay_pointers: u32,
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
