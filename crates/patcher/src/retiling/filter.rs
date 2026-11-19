use super::{
    areas::select_areas,
    events::{expand_dynamic_grid, find_dynamic_areas},
    world_map::WORLD_MAP_CLOUDS,
};
use crate::rom::resolve_world_layout;
use anyhow::Result;
use patch_catalog::PatchCatalog;
use retiling_catalog::{
    RetilingCatalog,
    palette::{DARK_WORLD_RAIN_PALETTE, LIGHT_WORLD_RAIN_PALETTE, select_palette_profiles},
    resolved::ResolvedRetiling,
};
use seed_data::SeedData;
use std::collections::{BTreeMap, BTreeSet};

/// Prepare the retiling data for a saved seed using only the current catalogs.
pub fn filter_retiling_catalog(
    seed: &SeedData,
    catalog: &RetilingCatalog,
    patches: &PatchCatalog,
) -> Result<RetilingCatalog> {
    let layout = resolve_world_layout(seed, patches)?;
    let profiles = select_palette_profiles(
        &catalog.palette_profiles,
        layout
            .areas
            .iter()
            .map(|area| (usize::from(area.vanilla_map_id), area.theme_name.as_str())),
    )?;
    let selected = select_areas(catalog, &layout)?;

    // Preserve the full catalog's palette allocations and authored tile indexes.
    let mut needed = BTreeSet::from([LIGHT_WORLD_RAIN_PALETTE, DARK_WORLD_RAIN_PALETTE]);
    let mut palette_profiles = BTreeMap::new();
    for area in &layout.areas {
        let id = usize::from(area.vanilla_map_id);
        if let Some(&profile) = profiles.get(&id) {
            needed.extend(profile.keys().copied());
            palette_profiles.insert(
                id,
                BTreeMap::from([(area.theme_name.clone(), Ok(profile.clone()))]),
            );
        }
    }
    for area in &selected.areas {
        for tile in area.map_tiles.iter().flatten() {
            needed.insert(tile.palette);
        }
    }

    let mut areas = BTreeMap::new();
    for name in WORLD_MAP_CLOUDS {
        let mut clouds = catalog.areas[name].clone();
        clouds.themes.retain(|theme, _| theme == "base");
        for layer in &clouds.themes["base"].layers {
            for tile in &layer.grid.tiles {
                needed.insert(tile.palette);
            }
        }
        areas.insert(name.to_owned(), clouds);
    }

    let mut patterns = Vec::new();
    for group in &catalog.dynamic_tile_groups {
        for variant in &group.variants {
            patterns.push(expand_dynamic_grid(&variant.before, &catalog.palettes));
        }
    }
    let mut matches = find_dynamic_areas(&selected.areas, &patterns).into_iter();
    let mut dynamic_tile_groups = catalog.dynamic_tile_groups.clone();
    for group in &mut dynamic_tile_groups {
        group.variants.retain_mut(|variant| {
            let areas = matches.next().unwrap();
            if areas.is_empty() {
                return false;
            }
            variant.areas = Some(areas);
            true
        });
    }

    let mut palettes = BTreeMap::new();
    for id in needed {
        palettes.insert(id, catalog.palettes[&id].clone());
    }
    Ok(RetilingCatalog {
        palettes,
        areas,
        dynamic_tile_groups,
        conditions: Default::default(),
        palette_profiles,
        resolved: Some(ResolvedRetiling {
            seed_name: seed.metadata.seed_name.clone(),
            selected,
        }),
    })
}
