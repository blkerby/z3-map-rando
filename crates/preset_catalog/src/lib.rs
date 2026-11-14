//! Built-in generation presets, prepared from the preset files and difficulty classifications in
//! `data/` (see `docs/data-formats/settings-data.md`).

use anyhow::{Result, ensure};
use seed_data::{Settings, SkillSettings};
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter, Read, Write};
use type_hash::TypeHash;

const CATALOG_MAGIC: [u8; 8] = *b"Z3PRESET";
const CATALOG_CONFIG: bincode_next::config::Configuration = bincode_next::config::standard();

#[derive(Clone, Debug, Serialize, Deserialize, TypeHash)]
pub struct PresetCatalog {
    /// Built-in root presets, the default first.
    pub root: Vec<RootPreset>,
    /// Built-in skill presets, derived from the difficulty classifications, easiest first.
    pub skill: Vec<SkillPreset>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TypeHash)]
pub struct RootPreset {
    pub id: String,
    pub label: String,
    /// The skill preset this preset selects.
    pub skill_preset: String,
    /// Complete values, with the selected skill preset applied.
    pub settings: Settings,
}

#[derive(Clone, Debug, Serialize, Deserialize, TypeHash)]
pub struct SkillPreset {
    pub id: String,
    pub label: String,
    pub settings: SkillSettings,
}

/// Write the uncompressed envelope followed by a Zstd-compressed bincode payload.
pub fn encode_catalog(
    catalog: &PresetCatalog,
    mut writer: impl Write,
    compression_level: i32,
) -> Result<()> {
    writer.write_all(&CATALOG_MAGIC)?;
    writer.write_all(&PresetCatalog::type_hash().to_le_bytes())?;
    let mut encoder = zstd::stream::write::Encoder::new(writer, compression_level)?;
    {
        let mut buffered = BufWriter::new(&mut encoder);
        bincode_next::serde::encode_into_std_write(catalog, &mut buffered, CATALOG_CONFIG)?;
        buffered.flush()?;
    }
    encoder.finish()?;
    Ok(())
}

/// Check the envelope, then stream the compressed payload into the catalog.
pub fn decode_catalog(mut reader: impl Read) -> Result<PresetCatalog> {
    let mut magic = [0; 8];
    reader.read_exact(&mut magic)?;
    ensure!(magic == CATALOG_MAGIC, "invalid preset catalog magic bytes");
    let mut hash = [0; 8];
    reader.read_exact(&mut hash)?;
    ensure!(
        u64::from_le_bytes(hash) == PresetCatalog::type_hash(),
        "preset catalog type hash mismatch; rebuild the catalog with this version"
    );
    let decoder = zstd::stream::read::Decoder::new(reader)?;
    let mut buffered = BufReader::new(decoder);
    let catalog = bincode_next::serde::decode_from_std_read(&mut buffered, CATALOG_CONFIG)?;
    Ok(catalog)
}
