//! Patch catalog containing seed-independent patching data, including
//! - IPS patches pre-built from ASM
//! - item data extracted from z3-json-data.

pub use common_types::{ItemLocationId, SourceRoomId};

use serde::{Deserialize, Serialize};
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
        bincode_next::serde::encode_into_std_write(catalog, &mut buffered, CATALOG_CONFIG)?;
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
    let catalog = bincode_next::serde::decode_from_std_read(&mut buffered, CATALOG_CONFIG)?;
    Ok(catalog)
}

#[derive(Clone, Debug, Serialize, Deserialize, TypeHash)]
pub struct PatchCatalog {
    pub patches: PatchIps,
    pub symbols: PatchSymbols,
    pub item_locations: BTreeMap<ItemLocationId, ItemLocation>,
    /// Keys are item names from `z3-json-data/items.json` plus project-local receipts.
    pub items: BTreeMap<String, ItemEncoding>,
}

/// A complete IPS payload with its diagnostic name.
#[derive(Clone, Debug, Serialize, Deserialize, TypeHash)]
pub struct Ips {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Complete IPS payloads, with fields named after their ASM filename stems.
/// Patches remain separate so the patcher can detect conflicting writes.
#[derive(Clone, Debug, Serialize, Deserialize, TypeHash)]
#[serde(deny_unknown_fields)]
pub struct PatchIps {
    pub bg3_tilemap: Ips,
    pub fastrom_base: Ips,
    pub fastrom_extra: Ips,
    pub item_placements: Ips,
    pub mirror_bg1_hdma: Ips,
    pub nmi_optimize: Ips,
    pub new_game: Ips,
    pub overworld_animations: Ips,
    pub overworld_assets: Ips,
    pub overworld_bg_color: Ips,
    pub overworld_bg_tilemaps: Ips,
    pub overworld_cutscenes: Ips,
    pub overworld_dynamic_tiles: Ips,
    pub overworld_entrances: Ips,
    pub overworld_lightning: Ips,
    pub overworld_map16_graphics: Ips,
    pub overworld_map16_properties: Ips,
    pub overworld_map_data: Ips,
    pub overworld_vram: Ips,
    pub rom_size: Ips,
}

/// Exported ASM interface consumed by Rust. Addresses are SNES addresses,
/// not ROM file offsets. Field names match the explicit ASM export names.
#[derive(Clone, Debug, Serialize, Deserialize, TypeHash)]
#[serde(deny_unknown_fields)]
pub struct PatchSymbols {
    pub catfish_item: u32,
    pub king_zora_item: u32,
    pub theme_flat_map16_data: u32,
    pub flat_map16_screen_pointers: u32,
    pub bg1_flat_map16_screen_pointers: u32,
    pub area_80_bg1_flat_map16_pointers: u32,
    pub generated_bg1_enabled: u32,
    pub lost_woods_clear_bg1_flat_map16_pointers: u32,
    pub rain_contexts: u32,
    pub rain_flat_map16_pointers: u32,
    pub theme_background_colors: u32,
    pub overworld_area_record_variant_pointers: u32,
    pub overworld_asset_data: u32,
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

#[derive(Clone, Debug, Serialize, Deserialize, TypeHash)]
pub struct ItemLocation {
    /// Unheadered ROM file offsets read from `itemAddress` or derived for prizes.
    /// Ordinary pickups receive the item's receipt ID as a single address.
    /// Dungeon prizes have six offsets, in the same order as
    /// `ItemEncoding::prize_patch_bytes`. Empty when no address is supplied,
    /// or for fixed keys and fairy exchanges.
    /// Non-chest addresses may refer to data consumed by the item-placement ASM.
    pub rom_addresses: Vec<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TypeHash)]
pub struct ItemEncoding {
    pub receipt_id: u8,
    /// Locally derived dungeon-prize bytes, paired with the location's six offsets.
    pub prize_patch_bytes: Option<[u8; 6]>,
}
