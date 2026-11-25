use crate::{
    asset_bundle::{CompiledCutscene, CompiledOverworldOverlay, DynamicTileEntry},
    graphics::TileGraphicResolver,
    import::{Importer, OverworldAreaAssets, OverworldBackgroundSettings, Tile16},
};
use anyhow::{Context, Result, ensure};
pub use retiling_catalog::BackgroundLayering;
use retiling_catalog::{
    PaletteId, RetilingCatalog,
    palette::{
        DARK_WORLD_RAIN_PALETTE, LIGHT_WORLD_RAIN_PALETTE, PALETTE_ATTEMPTS, allocate_palettes,
        select_palette_profiles,
    },
    resolved::SelectedAreas,
};
use seed_data::WorldLayout;
use std::collections::{BTreeMap, BTreeSet};

mod allocation;
mod areas;
mod condition;
mod events;
mod filter;
mod graphics;
mod maps;
mod rain;
pub use filter::filter_retiling_catalog;
mod world_map;
pub use world_map::{CompiledWorldMap, compile_world_map};

use allocation::{CHARACTER_CAPACITY, allocate_characters};
use areas::select_areas;
use events::{
    DynamicTileGroup, DynamicTileVariant, VANILLA_OVERWORLD_OVERLAYS, add_dynamic_dependencies,
    build_cutscenes, build_dynamic_overworld_overlays, build_dynamic_tile_groups,
    build_overworld_overlays, expand_dynamic_grid,
};
use graphics::{build_animation_tracks, build_character_rows, resolve_palettes};
use maps::{MAP16_CAPACITY, Map16Builder, build_bg1_maps, build_map};
use rain::{add_rain_tiles, build_rain_maps};

type TileKey = (PaletteId, usize);

pub struct CompiledBg1Variant {
    pub name: String,
    pub maps: BTreeMap<usize, Vec<u8>>,
}

pub struct CompiledTheme {
    pub world_map: CompiledWorldMap,
    pub screen_maps: BTreeMap<usize, Vec<u8>>,
    pub bg1_variants: BTreeMap<usize, Vec<CompiledBg1Variant>>,
    pub map16_definitions: [Vec<u8>; 4],
    pub map16_properties: [Vec<u8>; 4],
    pub background_colors: [u16; 0xa0],
    pub rain_contexts: [u8; 0xa0],
    pub rain_maps: [Vec<u8>; 2],
    pub area_assets: Vec<OverworldAreaAssets>,
    pub dynamic_tile_groups: Vec<Vec<DynamicTileEntry>>,
    pub cutscenes: Vec<CompiledCutscene>,
    pub overworld_overlays: Vec<CompiledOverworldOverlay>,
}

pub fn compile(
    catalog: &RetilingCatalog,
    importer: &Importer,
    vanilla_tiles: &[Tile16],
    vanilla_tile_types: &[u8],
    mut area_assets: Vec<OverworldAreaAssets>,
    layout: &WorldLayout,
    palette_seed: u64,
) -> Result<CompiledTheme> {
    let mut graphics = TileGraphicResolver::create(importer);
    let mut palettes = resolve_palettes(catalog, &mut graphics)?;
    let scrollable_transitions = layout.build_scrollable_transition_pairs();
    let mut area_themes = Vec::new();
    for (area, theme) in layout.build_area_themes() {
        area_themes.push((usize::from(area), theme));
    }
    let profiles = select_palette_profiles(&catalog.palette_profiles, area_themes)?;
    let allocation = allocate_palettes(&profiles, &scrollable_transitions, palette_seed)
        .with_context(|| {
            format!("no overworld palette assignment found in {PALETTE_ATTEMPTS} passes")
        })?;
    let mut color_indices = BTreeMap::new();
    for (&id, table) in &allocation {
        let mut indices = [0; 16];
        for (i, &rgb) in palettes[&id].colors.iter().enumerate().skip(1) {
            let color = crate::graphics::encode_bgr555(rgb);
            if let Some(j) = table.colors.iter().position(|&entry| entry == color) {
                indices[i] = (j + 1) as u8;
            }
        }
        color_indices.insert(id, indices);
    }
    let mut dynamic_tiles = Vec::new();
    for group in &catalog.dynamic_tile_groups {
        let mut variants = Vec::new();
        for variant in &group.variants {
            let mut after_frames = Vec::new();
            for frame in &variant.after_frames {
                after_frames.push(expand_dynamic_grid(frame, &catalog.palettes));
            }
            variants.push(DynamicTileVariant {
                before: expand_dynamic_grid(&variant.before, &catalog.palettes),
                after_frames,
                areas: variant.areas.clone(),
            });
        }
        dynamic_tiles.push(DynamicTileGroup {
            kind: group.kind,
            variants,
        });
    }
    let SelectedAreas {
        mut areas,
        background_colors,
        bg1_variants,
        background_settings,
        cutscenes,
    } = match &catalog.resolved {
        Some(resolved) => resolved.selected.clone(),
        None => select_areas(catalog, layout)?,
    };
    let rain_tiles = [
        add_rain_tiles(&mut palettes, &area_assets[0x2c], LIGHT_WORLD_RAIN_PALETTE),
        add_rain_tiles(&mut palettes, &area_assets[0x70], DARK_WORLD_RAIN_PALETTE),
    ];
    let mut bg1_areas = BTreeSet::new();
    for variant in &bg1_variants {
        bg1_areas.insert(variant.area);
    }
    let mut rain_contexts = [0; 0xa0];
    for area in &mut areas {
        if bg1_areas.contains(&area.id) {
            continue;
        }
        let context = if area.palettes.contains(&LIGHT_WORLD_RAIN_PALETTE)
            && !area.palettes.contains(&DARK_WORLD_RAIN_PALETTE)
        {
            1
        } else if area.palettes.contains(&DARK_WORLD_RAIN_PALETTE) || area.id >= 0x40 {
            2
        } else {
            1
        };
        let palette = if context == 1 {
            LIGHT_WORLD_RAIN_PALETTE
        } else {
            DARK_WORLD_RAIN_PALETTE
        };
        area.palettes.insert(palette);
        for &tile in &rain_tiles[context - 1] {
            area.extra_tiles.insert((palette, tile));
        }
        for map_y in 0..area.height / 64 {
            for map_x in 0..area.width / 64 {
                rain_contexts[area.id + map_x + map_y * 8] = context as u8;
            }
        }
    }

    add_dynamic_dependencies(&mut areas, &mut dynamic_tiles);
    ensure!(area_assets.len() == 0xa0);

    let (character_slots, area_tiles) =
        allocate_characters(&areas, &palettes, &scrollable_transitions)?;
    for variant in &bg1_variants {
        let area_index = areas
            .iter()
            .position(|area| area.id == variant.area)
            .with_context(|| format!("BG1 area {:02X} has no BG2 asset record", variant.area))?;
        for placement in variant.placements.iter().flatten() {
            ensure!(
                areas[area_index].palettes.contains(&placement.palette)
                    && area_tiles[area_index].contains(&(placement.palette, placement.tile)),
                "BG1 variant {} references an asset absent from area {:02X}",
                variant.name,
                variant.area
            );
        }
    }
    let character_count = character_slots
        .values()
        .copied()
        .max()
        .map_or(0, |slot| slot + 1);
    ensure!(
        character_count <= CHARACTER_CAPACITY,
        "world layout needs {character_count} stable character slots, but the existing row ABI exposes {CHARACTER_CAPACITY}"
    );

    let mut map16 = Map16Builder::create(
        vanilla_tiles,
        vanilla_tile_types,
        &palettes,
        &allocation,
        &character_slots,
    );

    let mut screen_maps = BTreeMap::new();
    for (area, tiles) in areas.iter().zip(&area_tiles) {
        let character_rows =
            build_character_rows(tiles, &palettes, &color_indices, &character_slots)?;
        let animation_tracks =
            build_animation_tracks(tiles, &palettes, &color_indices, &character_slots);
        for map_y in 0..area.height / 64 {
            for map_x in 0..area.width / 64 {
                let map = build_map(area, map_x, map_y, &mut map16)?;
                screen_maps.insert(area.id + map_x + map_y * 8, map);
            }
        }

        let mut palette_rows = vec![[0; 32]; 6];
        let mut active_rows = [false; 6];
        for id in &area.palettes {
            let table = &allocation[id];
            if table.colors.is_empty() || active_rows[table.row] {
                continue;
            }
            active_rows[table.row] = true;
            for (&color, entry) in table
                .colors
                .iter()
                .zip(palette_rows[table.row][2..].chunks_exact_mut(2))
            {
                entry.copy_from_slice(&color.to_le_bytes());
            }
        }
        let mut transition_palette_ranges = Vec::new();
        for (row, active) in active_rows.into_iter().enumerate() {
            if active {
                transition_palette_ranges.push(crate::import::OverworldPaletteRange {
                    start_color: (row + 2) as u8 * 16,
                    color_count: 16,
                });
            }
        }
        let mut transition_character_rows = vec![false; 60];
        for tile in tiles {
            transition_character_rows[character_slots[tile] / 16] = true;
        }
        let assets = &mut area_assets[area.id];
        assets.palette_rows = palette_rows;
        assets.character_rows = character_rows.to_vec();
        assets.transition_palette_ranges = transition_palette_ranges;
        assets.transition_character_rows = transition_character_rows;
        assets.animation_tracks = animation_tracks;
        assets.background = OverworldBackgroundSettings {
            layering: match background_settings[&area.id].layering {
                BackgroundLayering::None => 0,
                BackgroundLayering::HalfAdd => 1,
                BackgroundLayering::Backdrop => 2,
            },
            camera_follow_x: encode_eighths(background_settings[&area.id].camera_follow[0])?,
            camera_drift_x: encode_eighths(background_settings[&area.id].camera_drift[0])?,
            camera_follow_y: encode_eighths(background_settings[&area.id].camera_follow[1])?,
            camera_drift_y: encode_eighths(background_settings[&area.id].camera_drift[1])?,
        };
        for map_y in 0..area.height / 64 {
            for map_x in 0..area.width / 64 {
                let id = area.id + map_x + map_y * 8;
                let sprites = area_assets[id].sprite_variants.clone();
                area_assets[id] = area_assets[area.id].clone();
                area_assets[id].sprite_variants = sprites;
            }
        }
    }

    let dynamic_tile_groups = build_dynamic_tile_groups(&dynamic_tiles, &mut map16)?;

    let (cutscenes, mut overworld_overlays) = build_cutscenes(&areas, &cutscenes, &mut map16)?;
    overworld_overlays.extend(build_dynamic_overworld_overlays(
        &areas,
        &screen_maps,
        &dynamic_tile_groups,
    )?);
    overworld_overlays.extend(build_overworld_overlays(&areas, &mut map16)?);
    for &(area_id, name) in &VANILLA_OVERWORLD_OVERLAYS {
        let mut found = false;
        for overlay in &overworld_overlays {
            if !overlay.writes.is_empty() && overlay.areas.contains(&area_id) {
                found = true;
                break;
            }
        }
        if !found {
            eprintln!("Warning: no generated {name} overlay for area ${area_id:02X}");
        }
    }

    for (id, &words) in map16.definitions.iter().enumerate() {
        map16
            .background_ids
            .entry(words)
            .or_insert(u16::try_from(id)?);
    }
    let mut compiled_bg1_variants = BTreeMap::<usize, Vec<CompiledBg1Variant>>::new();
    for variant in bg1_variants {
        let maps = build_bg1_maps(&variant, &mut map16)?;
        compiled_bg1_variants
            .entry(variant.area)
            .or_default()
            .push(CompiledBg1Variant {
                name: variant.name,
                maps,
            });
    }

    let rain_maps = build_rain_maps(vanilla_tiles, &rain_tiles, &mut map16)?;

    ensure!(map16.definitions.len() <= MAP16_CAPACITY);

    let mut map16_definitions = std::array::from_fn(|_| Vec::new());
    for tile in &map16.definitions {
        for quadrant in 0..4 {
            map16_definitions[quadrant].extend_from_slice(&tile[quadrant].to_le_bytes());
        }
    }
    Ok(CompiledTheme {
        world_map: world_map::build_world_map(&areas, &palettes, layout, catalog, true)?,
        screen_maps,
        bg1_variants: compiled_bg1_variants,
        map16_definitions,
        map16_properties: map16.properties,
        background_colors,
        rain_contexts,
        rain_maps,
        area_assets,
        dynamic_tile_groups,
        cutscenes,
        overworld_overlays,
    })
}

fn encode_eighths(value: f32) -> Result<i8> {
    let scaled = value * 8.0;
    ensure!(
        scaled.fract() == 0.0 && scaled >= f32::from(i8::MIN) && scaled <= f32::from(i8::MAX),
        "background camera value {value} is not representable in signed eighths"
    );
    Ok(scaled as i8)
}
