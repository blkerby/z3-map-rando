//! ROM assembly shared by native and future browser callers.

use crate::{
    Patcher, PcAddr, SeedData, SnesAddr, TransitionAssetPhase, asset_bundle,
    import::{FlatMap16, Importer},
    retiling as theme, verify_vanilla_rom,
};
use anyhow::{Result, ensure};
use patch_catalog::PatchCatalog;
use retiling_catalog::RetilingCatalog;
use seed_data::WorldLayout;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Temporary address space used only to split the importer's vanilla flat maps.
const VANILLA_FLAT_MAPS_START: SnesAddr = SnesAddr(0xb80000);
const BANK_SIZE: usize = 0x8000;

/// Customization applied while patching, independently of saved generation settings.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct PatchSettings {
    pub transition_asset_phase: TransitionAssetPhase,
}

/// Development diagnostics; the patching core does not print them.
#[derive(Debug)]
pub struct RetilingStats {
    pub screen_count: usize,
    pub palette_count: usize,
    pub character_count: usize,
    pub map16_count: usize,
    pub bg1_area_count: usize,
    pub bg1_variant_names: Vec<String>,
    pub bg1_bytes: usize,
    pub background_record_count: usize,
    pub background_modes: [usize; 3],
    pub parallax_records: usize,
    pub drifting_records: usize,
    pub fullest_palette_screen: usize,
    pub fullest_palette_half_slots: usize,
    pub asset_pointer_table_bytes: usize,
    pub asset_data_bytes: usize,
    pub unique_asset_blocks: usize,
    pub warnings: Vec<String>,
}

struct FlatMapTables {
    flat: FlatMap16,
    bg1_pointers: Vec<u8>,
    area_80_bg1_pointers: Vec<u8>,
    lost_woods_clear_bg1_pointers: Vec<u8>,
    rain_pointers: Vec<u8>,
}

/// Create a ROM from saved choices using the current catalogs.
/// Item-placement writes are added separately; this currently applies retiling.
pub fn patch_rom(
    rom: &[u8],
    seed: &SeedData,
    retiling: &RetilingCatalog,
    patches: &PatchCatalog,
    settings: &PatchSettings,
) -> Result<Vec<u8>> {
    verify_vanilla_rom(rom)?;
    let mut output = rom.to_vec();
    apply_retiling(&mut output, &seed.world_layout, retiling, patches, settings)?;
    finalize_rom(&mut output);
    Ok(output)
}

/// Apply engine patches and selected artwork to a verified vanilla ROM buffer.
/// Call `finalize_rom` after all seed-specific writes are complete.
pub fn apply_retiling(
    rom: &mut Vec<u8>,
    layout: &WorldLayout,
    catalog: &RetilingCatalog,
    patches: &PatchCatalog,
    settings: &PatchSettings,
) -> Result<RetilingStats> {
    let mut fastrom_base = Patcher::default();
    fastrom_base.use_ips("fastrom_base.ips", &patches.patches.fastrom_base)?;
    fastrom_base.apply(rom)?;

    let mut importer = Importer::new(rom.clone())?;
    let vanilla_flat = importer.flat_map16((VANILLA_FLAT_MAPS_START.0 >> 16) as u8)?;
    let vanilla_maps = split_flat_maps(&vanilla_flat)?;
    let tile_types = importer.tile_types()?.to_owned();
    let vanilla_tiles = importer.tiles16()?.to_owned();
    let area_assets = importer.overworld_area_assets()?;
    let credits_cool_background = importer.credits_cool_background_assets()?;
    let sprite_seed = importer.overworld_sprite_seed()?;

    let compiled = theme::compile(
        catalog,
        &importer,
        &vanilla_tiles,
        &tile_types,
        area_assets,
        layout,
    )?;
    let credits_overworld = importer.credits_overworld_assets(&compiled.area_assets)?;
    let flat = assemble_flat_maps(
        vanilla_maps,
        &compiled.screen_maps,
        &compiled.bg1_variants,
        &compiled.rain_maps,
        patches.symbols.theme_flat_map16_data,
    )?;
    let bundle = asset_bundle::build(
        &compiled.area_assets,
        &credits_overworld,
        &credits_cool_background,
        &sprite_seed,
        &compiled.dynamic_tile_groups,
        &compiled.cutscenes,
        &compiled.overworld_overlays,
        settings.transition_asset_phase,
        &patches.symbols,
    )?;

    let mut stats = RetilingStats {
        screen_count: compiled.screen_count,
        palette_count: compiled.palette_count,
        character_count: compiled.character_count,
        map16_count: compiled.map16_count,
        bg1_area_count: compiled.bg1_variants.len(),
        bg1_variant_names: Vec::new(),
        bg1_bytes: 0,
        background_record_count: compiled.background_settings.len(),
        background_modes: [0; 3],
        parallax_records: 0,
        drifting_records: 0,
        fullest_palette_screen: compiled.fullest_palette_screen,
        fullest_palette_half_slots: compiled.fullest_palette_half_slots,
        asset_pointer_table_bytes: bundle.pointer_table.len(),
        asset_data_bytes: bundle.data.len(),
        unique_asset_blocks: bundle.unique_blocks,
        warnings: compiled.warnings,
    };
    for variants in compiled.bg1_variants.values() {
        for variant in variants {
            stats.bg1_variant_names.push(variant.name.clone());
            for map in variant.maps.values() {
                stats.bg1_bytes += map.len();
            }
        }
    }
    for background in compiled.background_settings.values() {
        match background.layering {
            theme::BackgroundLayering::None => stats.background_modes[0] += 1,
            theme::BackgroundLayering::HalfAdd => stats.background_modes[1] += 1,
            theme::BackgroundLayering::Backdrop => stats.background_modes[2] += 1,
        }
        if background.camera_follow_x != 1.0 || background.camera_follow_y != 1.0 {
            stats.parallax_records += 1;
        }
        if background.camera_drift_x != 0.0 || background.camera_drift_y != 0.0 {
            stats.drifting_records += 1;
        }
    }

    rom.resize(4 * 1024 * 1024, 0);
    let mut patcher = Patcher::default();
    patcher.context("flat Map16 data").write(
        SnesAddr(patches.symbols.theme_flat_map16_data).into(),
        flat.flat.maps,
    )?;
    patcher.context("flat Map16 pointers").write(
        SnesAddr(patches.symbols.flat_map16_screen_pointers).into(),
        flat.flat.screen_pointers,
    )?;
    patcher.context("BG1 flat Map16 pointers").write(
        SnesAddr(patches.symbols.bg1_flat_map16_screen_pointers).into(),
        flat.bg1_pointers,
    )?;
    patcher.context("area $80 BG1 variant pointers").write(
        SnesAddr(patches.symbols.area_80_bg1_flat_map16_pointers).into(),
        flat.area_80_bg1_pointers,
    )?;
    patcher.context("generated BG1 enable marker").write(
        SnesAddr(patches.symbols.generated_bg1_enabled).into(),
        vec![1],
    )?;
    patcher
        .context("Lost Woods clear BG1 variant pointers")
        .write(
            SnesAddr(patches.symbols.lost_woods_clear_bg1_flat_map16_pointers).into(),
            flat.lost_woods_clear_bg1_pointers,
        )?;
    patcher.context("rain contexts").write(
        SnesAddr(patches.symbols.rain_contexts).into(),
        compiled.rain_contexts.to_vec(),
    )?;
    patcher.context("rain map pointers").write(
        SnesAddr(patches.symbols.rain_flat_map16_pointers).into(),
        flat.rain_pointers,
    )?;

    let mut context = patcher.context("expanded Map16 definitions");
    for (start, definitions) in [
        patches.symbols.map16_top_left,
        patches.symbols.map16_top_right,
        patches.symbols.map16_bottom_left,
        patches.symbols.map16_bottom_right,
    ]
    .into_iter()
    .zip(compiled.map16_definitions)
    {
        context.write(SnesAddr(start).into(), definitions)?;
    }
    let mut context = patcher.context("Map16 quadrant properties");
    for (start, properties) in [
        patches.symbols.map16_property_top_left,
        patches.symbols.map16_property_top_right,
        patches.symbols.map16_property_bottom_left,
        patches.symbols.map16_property_bottom_right,
    ]
    .into_iter()
    .zip(compiled.map16_properties)
    {
        context.write(SnesAddr(start).into(), properties)?;
    }
    let mut background_colors = Vec::with_capacity(compiled.background_colors.len() * 2);
    for color in compiled.background_colors {
        background_colors.extend_from_slice(&color.to_le_bytes());
    }
    patcher.context("theme background colors").write(
        SnesAddr(patches.symbols.theme_background_colors).into(),
        background_colors,
    )?;
    patcher.context("overworld asset pointer table").write(
        SnesAddr(bundle.pointer_table_start).into(),
        bundle.pointer_table,
    )?;
    patcher
        .context("overworld asset data")
        .write(SnesAddr(bundle.data_start).into(), bundle.data)?;

    for (name, bytes) in [
        ("fastrom_extra.ips", &patches.patches.fastrom_extra),
        ("rom_size.ips", &patches.patches.rom_size),
        (
            "overworld_map_data.ips",
            &patches.patches.overworld_map_data,
        ),
        (
            "overworld_map16_graphics.ips",
            &patches.patches.overworld_map16_graphics,
        ),
        (
            "overworld_map16_properties.ips",
            &patches.patches.overworld_map16_properties,
        ),
        ("bg3_tilemap.ips", &patches.patches.bg3_tilemap),
        (
            "overworld_bg_tilemaps.ips",
            &patches.patches.overworld_bg_tilemaps,
        ),
        ("nmi_optimize.ips", &patches.patches.nmi_optimize),
        ("mirror_bg1_hdma.ips", &patches.patches.mirror_bg1_hdma),
        ("overworld_vram.ips", &patches.patches.overworld_vram),
        ("overworld_assets.ips", &patches.patches.overworld_assets),
        (
            "overworld_animations.ips",
            &patches.patches.overworld_animations,
        ),
        (
            "overworld_entrances.ips",
            &patches.patches.overworld_entrances,
        ),
        (
            "overworld_bg_color.ips",
            &patches.patches.overworld_bg_color,
        ),
        (
            "overworld_dynamic_tiles.ips",
            &patches.patches.overworld_dynamic_tiles,
        ),
        (
            "overworld_cutscenes.ips",
            &patches.patches.overworld_cutscenes,
        ),
        (
            "overworld_lightning.ips",
            &patches.patches.overworld_lightning,
        ),
    ] {
        patcher.use_ips(name, bytes)?;
    }
    patcher.apply(rom)?;
    rom[0x7fd7] = 0x0c;

    Ok(stats)
}

/// Update the checksum and complement after assembling the complete 4 MiB ROM.
pub fn finalize_rom(rom: &mut [u8]) {
    rom[0x7fdc..0x7fe0].copy_from_slice(&[0xff, 0xff, 0, 0]);
    let mut checksum = 0u16;
    for &byte in rom.iter() {
        checksum = checksum.wrapping_add(u16::from(byte));
    }
    rom[0x7fdc..0x7fde].copy_from_slice(&(!checksum).to_le_bytes());
    rom[0x7fde..0x7fe0].copy_from_slice(&checksum.to_le_bytes());
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

fn assemble_flat_maps(
    mut maps: Vec<Vec<u8>>,
    replacements: &BTreeMap<usize, Vec<u8>>,
    bg1_variants: &BTreeMap<usize, Vec<theme::CompiledBg1Variant>>,
    rain_maps: &[Vec<u8>; 2],
    map_data_start: u32,
) -> Result<FlatMapTables> {
    for (screen, map) in replacements {
        ensure!(
            *screen < maps.len(),
            "theme screen {screen:02X} is out of range"
        );
        maps[*screen] = map.clone();
    }

    let mut indices = BTreeMap::new();
    let mut data = Vec::new();
    let mut pointers = Vec::with_capacity(0xa0 * 3);
    for map in &maps {
        let address = intern_flat_map(map, &mut indices, &mut data, map_data_start)?;
        pointers.extend_from_slice(&address.to_le_bytes()[..3]);
    }

    let mut bg1_pointers = vec![0; 0xa0 * 3];
    let mut area_80_pointers = [0; 2];
    let mut lost_woods_clear_pointers = [0; 10];
    for (&area, variants) in bg1_variants {
        for variant in variants {
            let lost_woods_clear = area == 0x00 && variant.name == "Woods Clear";
            let area_80_index = if area == 0x80 {
                Some(match variant.name.as_str() {
                    "Grove Fog" => 0,
                    "Bridge Shadow" => 1,
                    name => anyhow::bail!("unknown area $80 BG1 variant {name}"),
                })
            } else {
                None
            };
            for (&screen, map) in &variant.maps {
                ensure!(screen < 0xa0, "BG1 screen {screen:02X} is out of range");
                ensure!(
                    replacements.contains_key(&screen),
                    "BG1 screen {screen:02X} has no generated BG2 map"
                );
                let address = intern_flat_map(map, &mut indices, &mut data, map_data_start)?;
                if (area != 0x80 || area_80_index == Some(0)) && !lost_woods_clear {
                    let offset = screen * 3;
                    ensure!(
                        bg1_pointers[offset..offset + 3] == [0; 3],
                        "multiple BG1 maps target screen {screen:02X}"
                    );
                    bg1_pointers[offset..offset + 3].copy_from_slice(&address.to_le_bytes()[..3]);
                }
                if lost_woods_clear {
                    lost_woods_clear_pointers[screen] = address;
                }
                if let Some(index) = area_80_index {
                    ensure!(
                        screen == 0x80,
                        "area $80 BG1 has unexpected map {screen:02X}"
                    );
                    area_80_pointers[index] = address;
                }
            }
        }
    }
    let mut rain_pointers = Vec::with_capacity(6);
    for map in rain_maps {
        let address = intern_flat_map(map, &mut indices, &mut data, map_data_start)?;
        rain_pointers.extend_from_slice(&address.to_le_bytes()[..3]);
    }
    ensure!(
        data.len() <= 10 * BANK_SIZE,
        "flat maps exceed reserved banks C0-C9"
    );
    let mut area_80_bg1_pointers = Vec::with_capacity(6);
    for pointer in area_80_pointers {
        area_80_bg1_pointers.extend_from_slice(&pointer.to_le_bytes()[..3]);
    }
    let mut lost_woods_clear_bg1_pointers = Vec::with_capacity(30);
    for pointer in lost_woods_clear_pointers {
        lost_woods_clear_bg1_pointers.extend_from_slice(&pointer.to_le_bytes()[..3]);
    }
    Ok(FlatMapTables {
        flat: FlatMap16 {
            maps: data,
            screen_pointers: pointers,
        },
        bg1_pointers,
        area_80_bg1_pointers,
        lost_woods_clear_bg1_pointers,
        rain_pointers,
    })
}

fn intern_flat_map(
    map: &[u8],
    indices: &mut BTreeMap<Vec<u8>, usize>,
    data: &mut Vec<u8>,
    map_data_start: u32,
) -> Result<u32> {
    ensure!(map.len() == 0x800);
    let index = if let Some(&index) = indices.get(map) {
        index
    } else {
        let index = indices.len();
        indices.insert(map.to_vec(), index);
        data.extend_from_slice(map);
        index
    };
    Ok(map_data_start + u32::try_from(index / 16)? * 0x10000 + u32::try_from(index % 16)? * 0x0800)
}
