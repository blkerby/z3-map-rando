use anyhow::{Context, Result, ensure};
use clap::Parser;
use engine_check::{
    asset_bundle::{self, AssetLayout},
    rain_tilemap::RAIN_TILEMAP,
};
use patcher::{
    Patcher, PcAddr, SnesAddr,
    import::{FlatMap16, Importer},
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt::Write, fs, path::PathBuf};

mod theme;

const VANILLA_ROM_SHA256: &str = "794e040b02c7591b59ad8843b51e7c619b88f87cddc6083a8e7a4027b96a2271";
const VANILLA_FLAT_MAPS_START: SnesAddr = SnesAddr(0xb80000);
const THEME_FLAT_MAPS_START: SnesAddr = SnesAddr(0xc08000);
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
const THEME_BACKGROUND_COLORS_START: SnesAddr = SnesAddr(0xa09e00);
const THEME_ASSET_DATA_START: u32 = 0xd08000;
const BANK_SIZE: usize = 0x8000;
const RAIN_OVERLAY: usize = 0x9f;

#[derive(Parser)]
struct Args {
    input_rom: PathBuf,
    output_rom: PathBuf,
    retiling_project: PathBuf,
    #[arg(long, value_enum, default_value = "pre-scroll")]
    transition_asset_phase: asset_bundle::TransitionAssetPhase,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut rom = read_vanilla_rom(&args.input_rom)?;
    let patch_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../patches/ips");

    let mut fastrom_base = Patcher::default();
    fastrom_base.use_ips(&patch_dir.join("fastrom_base.ips"))?;
    fastrom_base.apply(&mut rom)?;

    let mut importer = Importer::new(rom.clone())?;
    let mut vanilla_flat = importer.flat_map16((VANILLA_FLAT_MAPS_START.0 >> 16) as u8)?;
    replace_rain_tilemap(&mut vanilla_flat)?;
    let vanilla_maps = split_flat_maps(&vanilla_flat)?;
    let tile_types = importer.tile_types()?.to_owned();
    let vanilla_tiles = importer.tiles16()?.to_owned();
    let area_assets = importer.overworld_area_assets()?;
    let credits_overworld = importer.credits_overworld_assets()?;
    let credits_cool_background = importer.credits_cool_background_assets()?;
    let sprite_seed = importer.overworld_sprite_seed()?;

    let compiled = theme::compile(
        &args.retiling_project,
        &vanilla_tiles,
        &tile_types,
        area_assets,
    )?;
    let flat = assemble_flat_maps(vanilla_maps, compiled.screen_maps)?;
    let bundle = asset_bundle::build(
        &compiled.area_assets,
        &credits_overworld,
        &credits_cool_background,
        &sprite_seed,
        &compiled.dynamic_tile_groups,
        args.transition_asset_phase,
        AssetLayout {
            data_start: THEME_ASSET_DATA_START,
            data_size: 48 * BANK_SIZE,
        },
    )?;

    eprintln!(
        "Desert: {} screens, {} palettes, {} character slots, {} Map16 definitions",
        compiled.screen_count,
        compiled.palette_count,
        compiled.character_count,
        compiled.map16_count,
    );
    let bg1_variant_count: usize = compiled.bg1_variants.values().map(Vec::len).sum();
    let mut bg1_bytes = 0;
    let mut bg1_names = Vec::new();
    for variants in compiled.bg1_variants.values() {
        for variant in variants {
            bg1_names.push(variant.name.as_str());
            for map in &variant.maps {
                bg1_bytes += map.len();
            }
        }
    }
    let mut background_modes = [0; 3];
    let mut parallax_records = 0;
    let mut drifting_records = 0;
    for settings in compiled.background_settings.values() {
        match settings.layering {
            theme::BackgroundLayering::None => background_modes[0] += 1,
            theme::BackgroundLayering::HalfAdd => background_modes[1] += 1,
            theme::BackgroundLayering::Backdrop => background_modes[2] += 1,
        }
        if settings.camera_follow_x != 1.0 || settings.camera_follow_y != 1.0 {
            parallax_records += 1;
        }
        if settings.camera_drift_x != 0.0 || settings.camera_drift_y != 0.0 {
            drifting_records += 1;
        }
    }
    eprintln!(
        "BG1: {} areas, {bg1_variant_count} variants ({bg1_bytes} bytes: {}), {} background records; modes {:?}, {parallax_records} parallax, {drifting_records} drifting; fullest palette ${:02X} uses {}/12 half-slots",
        compiled.bg1_variants.len(),
        bg1_names.join(", "),
        compiled.background_settings.len(),
        background_modes,
        compiled.fullest_palette_screen,
        compiled.fullest_palette_half_slots,
    );
    eprintln!(
        "overworld assets: {} bytes total ({} pointer table, {} data), {} unique blocks",
        bundle.pointer_table.len() + bundle.data.len(),
        bundle.pointer_table.len(),
        bundle.data.len(),
        bundle.unique_blocks,
    );

    rom.resize(4 * 1024 * 1024, 0);
    let mut patcher = Patcher::default();
    patcher
        .context("flat Map16 data")
        .write(THEME_FLAT_MAPS_START.into(), flat.maps)?;
    patcher
        .context("flat Map16 pointers")
        .write(FLAT_MAP_POINTERS_START.into(), flat.screen_pointers)?;

    let mut context = patcher.context("expanded Map16 definitions");
    for (start, definitions) in MAP16_DEFINITION_STARTS
        .into_iter()
        .zip(compiled.map16_definitions)
    {
        context.write(start.into(), definitions)?;
    }
    let mut context = patcher.context("Map16 quadrant properties");
    for (start, properties) in MAP16_PROPERTY_STARTS
        .into_iter()
        .zip(compiled.map16_properties)
    {
        context.write(start.into(), properties)?;
    }
    let mut background_colors = Vec::with_capacity(compiled.background_colors.len() * 2);
    for color in compiled.background_colors {
        background_colors.extend_from_slice(&color.to_le_bytes());
    }
    patcher
        .context("theme background colors")
        .write(THEME_BACKGROUND_COLORS_START.into(), background_colors)?;
    patcher.context("overworld asset pointer table").write(
        SnesAddr(bundle.pointer_table_start).into(),
        bundle.pointer_table,
    )?;
    patcher
        .context("overworld asset data")
        .write(SnesAddr(bundle.data_start).into(), bundle.data)?;

    for patch in [
        "fastrom_extra.ips",
        "rom_size.ips",
        "overworld_map_data.ips",
        "overworld_map16_graphics.ips",
        "overworld_map16_properties.ips",
        "bg3_tilemap.ips",
        "overworld_bg_tilemaps.ips",
        "nmi_optimize.ips",
        "mirror_bg1_hdma.ips",
        "overworld_vram.ips",
        "overworld_assets.ips",
        "overworld_animations.ips",
        "overworld_entrances.ips",
        "overworld_bg_color.ips",
        "overworld_dynamic_tiles.ips",
    ] {
        patcher.use_ips(&patch_dir.join(patch))?;
    }
    patcher.apply(&mut rom)?;
    rom[0x7fd7] = 0x0c;

    fs::write(&args.output_rom, rom)
        .with_context(|| format!("failed to write {}", args.output_rom.display()))
}

fn read_vanilla_rom(path: &PathBuf) -> Result<Vec<u8>> {
    let rom = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    ensure!(
        rom.len() == 1024 * 1024,
        "expected a 1 MiB vanilla ROM, got {} bytes",
        rom.len()
    );
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(&rom) {
        write!(digest, "{byte:02x}")?;
    }
    ensure!(
        digest == VANILLA_ROM_SHA256,
        "input ROM SHA-256 mismatch: expected {VANILLA_ROM_SHA256}, got {digest}"
    );
    Ok(rom)
}

fn get_flat_map(flat: &FlatMap16, screen: usize) -> Result<Vec<u8>> {
    let pointer = &flat.screen_pointers[screen * 3..screen * 3 + 3];
    let pointer = u32::from_le_bytes([pointer[0], pointer[1], pointer[2], 0]);
    let map_start: PcAddr = SnesAddr(pointer).into();
    let flat_start: PcAddr = VANILLA_FLAT_MAPS_START.into();
    let offset = usize::try_from(map_start.0 - flat_start.0)?;
    Ok(flat.maps[offset..offset + 0x800].to_vec())
}

fn split_flat_maps(flat: &FlatMap16) -> Result<Vec<Vec<u8>>> {
    let mut maps = Vec::with_capacity(0xa0);
    for screen in 0..0xa0 {
        maps.push(get_flat_map(flat, screen)?);
    }
    Ok(maps)
}

fn replace_rain_tilemap(flat: &mut FlatMap16) -> Result<()> {
    let pointer = &flat.screen_pointers[RAIN_OVERLAY * 3..RAIN_OVERLAY * 3 + 3];
    let pointer = u32::from_le_bytes([pointer[0], pointer[1], pointer[2], 0]);
    let map_start: PcAddr = SnesAddr(pointer).into();
    let flat_start: PcAddr = VANILLA_FLAT_MAPS_START.into();
    let offset = usize::try_from(map_start.0 - flat_start.0)?;
    let rain = &mut flat.maps[offset..offset + 32 * 16 * 2];
    for (bytes, &tile) in rain.chunks_exact_mut(2).zip(RAIN_TILEMAP.iter().flatten()) {
        bytes.copy_from_slice(&tile.to_le_bytes());
    }
    Ok(())
}

fn assemble_flat_maps(
    mut maps: Vec<Vec<u8>>,
    replacements: BTreeMap<usize, Vec<u8>>,
) -> Result<FlatMap16> {
    for (screen, map) in replacements {
        ensure!(
            screen < maps.len(),
            "theme screen {screen:02X} is out of range"
        );
        maps[screen] = map;
    }

    let mut indices = BTreeMap::new();
    let mut data = Vec::new();
    let mut pointers = Vec::with_capacity(0xa0 * 3);
    for map in maps {
        ensure!(map.len() == 0x800);
        let index = if let Some(&index) = indices.get(&map) {
            index
        } else {
            let index = indices.len();
            indices.insert(map.clone(), index);
            data.extend_from_slice(&map);
            index
        };
        let address = THEME_FLAT_MAPS_START.0
            + u32::try_from(index / 16)? * 0x10000
            + u32::try_from(index % 16)? * 0x0800;
        pointers.extend_from_slice(&address.to_le_bytes()[..3]);
    }
    ensure!(
        data.len() <= 10 * BANK_SIZE,
        "flat maps exceed reserved banks C0-C9"
    );
    Ok(FlatMap16 {
        maps: data,
        screen_pointers: pointers,
    })
}
