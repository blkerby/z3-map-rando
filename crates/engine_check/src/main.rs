use anyhow::{Context, Result};
use clap::Parser;
use patcher::{
    Patcher, PcAddr, SnesAddr,
    import::{FlatMap16, Importer, Tile16},
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

mod asset_bundle;
mod rain_tilemap;

use rain_tilemap::RAIN_TILEMAP;

const VANILLA_ROM_SHA256: &str = "794e040b02c7591b59ad8843b51e7c619b88f87cddc6083a8e7a4027b96a2271";
const FLAT_MAPS_START: SnesAddr = SnesAddr(0xb80000);
const FLAT_MAP_POINTERS_START: SnesAddr = SnesAddr(0xbfe000);
const MAP16_DEFINITION_STARTS: [SnesAddr; 4] = [
    SnesAddr(0xa18000),
    SnesAddr(0xa28000),
    SnesAddr(0xa38000),
    SnesAddr(0xa48000),
];
const MAP16_PROPERTY_STARTS: [SnesAddr; 4] = [
    SnesAddr(0xa58000),
    SnesAddr(0xa5c000),
    SnesAddr(0xa68000),
    SnesAddr(0xa6c000),
];
const RAIN_OVERLAY: usize = 0x9f;

// TEMPORARY: These sources are blanked after importing the generated asset
// bundle so any remaining use of the vanilla overworld loaders is visible.
const VANILLA_BG_PALETTE_SOURCES: [(SnesAddr, usize); 3] = [
    (SnesAddr(0x1be6c8), 6 * 5 * 7 * 2),
    (SnesAddr(0x1be86c), 20 * 3 * 7 * 2),
    (SnesAddr(0x1be604), 14 * 7 * 2),
];
const GFX_BANK_TABLE_POINTER: SnesAddr = SnesAddr(0x00e7d0);
const GFX_HIGH_TABLE_POINTER: SnesAddr = SnesAddr(0x00e7d5);
const GFX_LOW_TABLE_POINTER: SnesAddr = SnesAddr(0x00e7da);
const BLANK_3BPP_SHEET: SnesAddr = SnesAddr(0xbfe1e0);
const BLANK_4BPP_SHEET: SnesAddr = SnesAddr(0xbfe1e7);
const COMPRESSED_BLANK_3BPP: [u8; 7] = [0xe7, 0xff, 0, 0xe5, 0xff, 0, 0xff];
const COMPRESSED_BLANK_4BPP: [u8; 7] = [0xe7, 0xff, 0, 0xe7, 0xff, 0, 0xff];

#[derive(Parser)]
struct Args {
    input_rom: PathBuf,
    output_rom: PathBuf,
}

fn split_map16_definitions(tiles16: &[Tile16]) -> [Vec<u8>; 4] {
    std::array::from_fn(|quadrant| {
        tiles16
            .iter()
            .flat_map(|tile| tile[quadrant].to_vram_tilemap_word().to_le_bytes())
            .collect()
    })
}

fn split_map16_properties(tiles16: &[Tile16], tile_types: &[u8]) -> [Vec<u8>; 4] {
    std::array::from_fn(|quadrant| {
        let mut properties = vec![0; 0x4000];
        for (property, tile) in properties.iter_mut().zip(tiles16) {
            let word = tile[quadrant].to_vram_tilemap_word();
            let tile_type = tile_types[(word & 0x01ff) as usize];
            // Vanilla folds horizontal flip into the directional slope types.
            *property = if (0x10..0x1c).contains(&tile_type) {
                tile_type | ((word >> 14) as u8 & 1)
            } else {
                tile_type
            };
        }
        properties
    })
}

fn flat_map_mut(flat: &mut FlatMap16, screen: usize) -> Result<&mut [u8]> {
    let pointer = &flat.screen_pointers[screen * 3..screen * 3 + 3];
    let pointer = u32::from_le_bytes([pointer[0], pointer[1], pointer[2], 0]);
    let map_start: PcAddr = SnesAddr(pointer).into();
    let flat_start: PcAddr = FLAT_MAPS_START.into();
    let offset = usize::try_from(map_start.0 - flat_start.0)?;
    Ok(&mut flat.maps[offset..offset + 32 * 32 * 2])
}

fn replace_rain_tilemap(flat: &mut FlatMap16) -> Result<()> {
    let rain = &mut flat_map_mut(flat, RAIN_OVERLAY)?[..32 * 16 * 2];
    for (bytes, &tile) in rain.chunks_exact_mut(2).zip(RAIN_TILEMAP.iter().flatten()) {
        bytes.copy_from_slice(&tile.to_le_bytes());
    }
    Ok(())
}

fn read_u16(rom: &[u8], address: SnesAddr) -> Result<u16> {
    let start = usize::try_from(PcAddr::from(address).0)?;
    let bytes = rom
        .get(start..start + 2)
        .with_context(|| format!("read past ROM at ${:06X}", address.0))?;
    Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
}

fn blank_vanilla_overworld_assets(
    patcher: &mut Patcher,
    rom: &[u8],
    graphics_sheets: &[u8],
) -> Result<()> {
    let mut context = patcher.context("temporary blank vanilla BG palettes");
    for (start, length) in VANILLA_BG_PALETTE_SOURCES {
        context.write(start.into(), vec![0; length])?;
    }

    let mut context = patcher.context("temporary blank compressed BG graphics");
    context.write(BLANK_3BPP_SHEET.into(), COMPRESSED_BLANK_3BPP.to_vec())?;
    context.write(BLANK_4BPP_SHEET.into(), COMPRESSED_BLANK_4BPP.to_vec())?;

    let tables = [
        read_u16(rom, GFX_BANK_TABLE_POINTER)?,
        read_u16(rom, GFX_HIGH_TABLE_POINTER)?,
        read_u16(rom, GFX_LOW_TABLE_POINTER)?,
    ];
    let mut context = patcher.context("temporary redirect vanilla BG graphics");
    for &sheet in graphics_sheets {
        if (0x58..=0x5b).contains(&sheet) {
            // Skip animated tile sheets for now.
            continue;
        }
        anyhow::ensure!(
            sheet <= 0x72,
            "invalid overworld graphics sheet {sheet:02X}"
        );
        let source = if sheet <= 0x70 {
            BLANK_3BPP_SHEET.0
        } else {
            BLANK_4BPP_SHEET.0
        };
        let pointer = [(source >> 16) as u8, (source >> 8) as u8, source as u8];
        for (&table, &byte) in tables.iter().zip(&pointer) {
            context.write(
                SnesAddr(u32::from(table) + u32::from(sheet)).into(),
                vec![byte],
            )?;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut rom = fs::read(&args.input_rom)
        .with_context(|| format!("failed to read {}", args.input_rom.display()))?;
    anyhow::ensure!(
        rom.len() == 1024 * 1024,
        "expected a 1 MiB vanilla ROM, got {} bytes",
        rom.len()
    );
    let digest = Sha256::digest(&rom)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    anyhow::ensure!(
        digest == VANILLA_ROM_SHA256,
        "input ROM SHA-256 mismatch: expected {VANILLA_ROM_SHA256}, got {digest}"
    );

    let patch_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../patches/ips");

    // We don't want the "fastrom_base" to participate in conflict detection like the
    // other patches, because it's not actually a problem if other patches overwrite
    // the same code that it touches. So we use a temporary Patcher to apply it
    // directly to the base ROM.
    let mut fastrom_base = Patcher::default();
    fastrom_base.use_ips(&patch_dir.join("fastrom_base.ips"))?;
    fastrom_base.apply(&mut rom)?;

    let mut importer = Importer::new(rom.clone())?;
    let mut flat_map16 = importer.flat_map16((FLAT_MAPS_START.0 >> 16) as u8)?;
    replace_rain_tilemap(&mut flat_map16)?;
    let tile_types = importer.tile_types()?.to_owned();
    let map16_definitions = split_map16_definitions(importer.tiles16()?);
    let map16_properties = split_map16_properties(importer.tiles16()?, &tile_types);
    let area_assets = importer.overworld_area_assets()?;
    let graphics_sheets = importer.overworld_graphics_sheets()?;
    let asset_bundle = asset_bundle::build(&area_assets)?;
    eprintln!(
        "overworld assets: {} bytes total ({} metadata, {} payload), 16576 DMA bytes/area, {} unique payloads",
        asset_bundle.metadata.len() + asset_bundle.payload.len(),
        asset_bundle.metadata.len(),
        asset_bundle.payload.len(),
        asset_bundle.unique_payloads
    );
    rom.resize(2 * 1024 * 1024, 0);

    let mut patcher = Patcher::default();
    blank_vanilla_overworld_assets(&mut patcher, &rom, &graphics_sheets)?;

    let mut context = patcher.context("flat Map16 data");
    context.write(FLAT_MAPS_START.into(), flat_map16.maps)?;
    context.write(FLAT_MAP_POINTERS_START.into(), flat_map16.screen_pointers)?;

    let mut context = patcher.context("expanded Map16 definitions");
    for (start, definitions) in MAP16_DEFINITION_STARTS.into_iter().zip(map16_definitions) {
        context.write(start.into(), definitions)?;
    }

    let mut context = patcher.context("Map16 quadrant properties");
    for (start, properties) in MAP16_PROPERTY_STARTS.into_iter().zip(map16_properties) {
        context.write(start.into(), properties)?;
    }

    let mut context = patcher.context("overworld asset metadata");
    context.write(
        SnesAddr(asset_bundle::METADATA_START).into(),
        asset_bundle.metadata,
    )?;

    let mut context = patcher.context("overworld asset payloads");
    context.write(
        SnesAddr(asset_bundle::PAYLOAD_START).into(),
        asset_bundle.payload,
    )?;

    let patches = [
        "fastrom_extra.ips",
        "expand.ips",
        "flat_map16.ips",
        "expand_map16.ips",
        "independent_tile_type.ips",
        "reduce_bg3.ips",
        "bg_streamer.ips",
        "nmi_optimize.ips",
        "mirror_bg1.ips",
        "overworld_vram.ips",
    ];
    for patch in patches {
        patcher.use_ips(&patch_dir.join(patch))?;
    }
    patcher.apply(&mut rom)?;

    fs::write(&args.output_rom, rom)
        .with_context(|| format!("failed to write {}", args.output_rom.display()))
}
