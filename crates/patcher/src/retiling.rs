use crate::{
    asset_bundle::{CompiledCutscene, CompiledOverworldOverlay, DynamicTileEntry},
    graphics::TileGraphicResolver,
    import::{Importer, OverworldAreaAssets, OverworldBackgroundSettings, Tile16},
};
use anyhow::{Context, Result, ensure};
pub use retiling_catalog::BackgroundLayering;
use retiling_catalog::{PaletteId, RetilingCatalog};
use std::collections::{BTreeMap, BTreeSet};

mod allocation;
mod areas;
mod events;
mod graphics;
mod maps;
mod rain;

use allocation::{
    CHARACTER_CAPACITY, allocate_characters, allocate_palettes, build_scrollable_transition_pairs,
};
use areas::{SelectedAreas, select_areas};
use events::{
    DynamicTileGroup, DynamicTileVariant, VANILLA_OVERWORLD_OVERLAYS, add_dynamic_dependencies,
    build_cutscenes, build_dynamic_overworld_overlays, build_dynamic_tile_groups,
    build_overworld_overlays, expand_dynamic_grid,
};
use graphics::{
    build_animation_tracks, build_character_rows, build_palette_rows, resolve_palettes,
};
use maps::{MAP16_CAPACITY, Map16Builder, build_bg1_maps, build_map};
use rain::{DARK_WORLD_RAIN_PALETTE, LIGHT_WORLD_RAIN_PALETTE, add_rain_tiles, build_rain_maps};

type TileKey = (PaletteId, usize);

pub struct CompiledBg1Variant {
    pub name: String,
    pub maps: BTreeMap<usize, Vec<u8>>,
}

pub struct BackgroundSettings {
    pub layering: BackgroundLayering,
    pub camera_follow_x: f32,
    pub camera_drift_x: f32,
    pub camera_follow_y: f32,
    pub camera_drift_y: f32,
}

pub struct CompiledTheme {
    pub screen_maps: BTreeMap<usize, Vec<u8>>,
    pub bg1_variants: BTreeMap<usize, Vec<CompiledBg1Variant>>,
    pub background_settings: BTreeMap<usize, BackgroundSettings>,
    pub map16_definitions: [Vec<u8>; 4],
    pub map16_properties: [Vec<u8>; 4],
    pub background_colors: [u16; 0xa0],
    pub rain_contexts: [u8; 0xa0],
    pub rain_maps: [Vec<u8>; 2],
    pub area_assets: Vec<OverworldAreaAssets>,
    pub dynamic_tile_groups: Vec<Vec<DynamicTileEntry>>,
    pub cutscenes: Vec<CompiledCutscene>,
    pub overworld_overlays: Vec<CompiledOverworldOverlay>,
    pub screen_count: usize,
    pub palette_count: usize,
    pub character_count: usize,
    pub map16_count: usize,
    pub fullest_palette_screen: usize,
    pub fullest_palette_half_slots: usize,
}

pub fn compile(
    catalog: &RetilingCatalog,
    importer: &Importer,
    vanilla_tiles: &[Tile16],
    vanilla_tile_types: &[u8],
    mut area_assets: Vec<OverworldAreaAssets>,
    theme_name: &str,
) -> Result<CompiledTheme> {
    let mut graphics = TileGraphicResolver::create(importer);
    let mut palettes = resolve_palettes(catalog, &mut graphics)?;
    let mut dynamic_tiles = Vec::new();
    for group in &catalog.dynamic_tile_groups {
        let mut variants = Vec::new();
        for variant in &group.variants {
            let mut after_frames = Vec::new();
            for frame in &variant.after_frames {
                after_frames.push(expand_dynamic_grid(frame));
            }
            variants.push(DynamicTileVariant {
                before: expand_dynamic_grid(&variant.before),
                after_frames,
                used: false,
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
        mut bg1_variants,
        background_settings,
        mut cutscenes,
    } = select_areas(catalog, theme_name)?;
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

    for palette in palettes.values_mut() {
        for tile in &mut palette.tiles {
            tile.canonicalize_flips();
        }
    }
    for area in &mut areas {
        for placement in &mut area.placements {
            placement.canonicalize_flip(&palettes);
        }
        if let Some(layer) = &mut area.overworld_overlay {
            for placement in layer.placements.iter_mut().flatten() {
                placement.canonicalize_flip(&palettes);
            }
        }
    }
    for variant in &mut bg1_variants {
        for placement in variant.placements.iter_mut().flatten() {
            placement.canonicalize_flip(&palettes);
        }
    }
    for cutscene in &mut cutscenes {
        for layer in cutscene.layers.values_mut() {
            for placement in layer.placements.iter_mut().flatten() {
                placement.canonicalize_flip(&palettes);
            }
        }
    }
    for group in &mut dynamic_tiles {
        for variant in &mut group.variants {
            for row in &mut variant.before {
                for placement in row {
                    placement.canonicalize_flip(&palettes);
                }
            }
            for frame in &mut variant.after_frames {
                for row in frame {
                    for placement in row {
                        placement.canonicalize_flip(&palettes);
                    }
                }
            }
        }
    }
    add_dynamic_dependencies(&mut areas, &mut dynamic_tiles);
    ensure!(area_assets.len() == 0xa0);

    let scrollable_transitions = build_scrollable_transition_pairs();
    let palette_slots = allocate_palettes(&areas, &palettes, &scrollable_transitions)?;
    let mut fullest_palette = (0, 0);
    for area in &areas {
        let mut occupied = [false; 12];
        for palette in &area.palettes {
            let slot = palette_slots[palette];
            occupied[slot] = true;
            if palettes[palette].uses_upper_half {
                occupied[slot + 1] = true;
            }
        }
        let count = occupied.into_iter().filter(|occupied| *occupied).count();
        if count > fullest_palette.1 {
            fullest_palette = (area.id, count);
        }
    }
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
        "{theme_name} needs {character_count} stable character slots, but the existing row ABI exposes {CHARACTER_CAPACITY}"
    );

    let mut map16 = Map16Builder::create(
        vanilla_tiles,
        vanilla_tile_types,
        &palettes,
        &palette_slots,
        &character_slots,
    );

    let mut screen_maps = BTreeMap::new();
    for (area, tiles) in areas.iter().zip(&area_tiles) {
        let palette_rows = build_palette_rows(area, &palettes, &palette_slots);
        let character_rows =
            build_character_rows(tiles, &palettes, &palette_slots, &character_slots)?;
        let animation_tracks =
            build_animation_tracks(tiles, &palettes, &palette_slots, &character_slots);
        for map_y in 0..area.height / 64 {
            for map_x in 0..area.width / 64 {
                let map = build_map(area, map_x, map_y, &mut map16)?;
                screen_maps.insert(area.id + map_x + map_y * 8, map);
            }
        }

        let mut transition_palette_halves = [false; 12];
        for palette in &area.palettes {
            let slot = palette_slots[palette];
            transition_palette_halves[slot] = true;
            if palettes[palette].uses_upper_half {
                transition_palette_halves[slot + 1] = true;
            }
        }
        let mut transition_palette_ranges = Vec::new();
        for row in 0..6 {
            let lower = transition_palette_halves[row * 2];
            let upper = transition_palette_halves[row * 2 + 1];
            if lower || upper {
                transition_palette_ranges.push(crate::import::OverworldPaletteRange {
                    start_color: (2 + row) as u8 * 16 + if lower { 0 } else { 8 },
                    color_count: if lower && upper { 16 } else { 8 },
                });
            }
        }
        let mut transition_character_rows = vec![false; 60];
        for tile in tiles {
            transition_character_rows[character_slots[tile] / 16] = true;
        }
        let assets = &mut area_assets[area.id];
        assets.palette_rows = palette_rows.to_vec();
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
            camera_follow_x: encode_eighths(background_settings[&area.id].camera_follow_x)?,
            camera_drift_x: encode_eighths(background_settings[&area.id].camera_drift_x)?,
            camera_follow_y: encode_eighths(background_settings[&area.id].camera_follow_y)?,
            camera_drift_y: encode_eighths(background_settings[&area.id].camera_drift_y)?,
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
            eprintln!(
                "warning: {theme_name} has no generated {name} overlay for area ${area_id:02X}"
            );
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
        screen_count: screen_maps.len(),
        palette_count: palette_slots.len(),
        character_count,
        map16_count: map16.definitions.len(),
        fullest_palette_screen: fullest_palette.0,
        fullest_palette_half_slots: fullest_palette.1,
        screen_maps,
        bg1_variants: compiled_bg1_variants,
        background_settings,
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
