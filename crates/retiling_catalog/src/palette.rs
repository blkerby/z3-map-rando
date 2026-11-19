//! Palette requirements for the six overworld BG rows, shared by generation and ROM patching.

use crate::{Background, PaletteId, RetilingCatalog, TileGraphic, TileId, graphics::encode_bgr555};
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
};

pub const LIGHT_WORLD_RAIN_PALETTE: PaletteId = 3;
pub const DARK_WORLD_RAIN_PALETTE: PaletteId = 8;
pub const PALETTE_ATTEMPTS: usize = 64;

/// Opaque BGR555 colors by authored palette. An empty entry represents transparent tiles.
pub type AreaPaletteProfile = BTreeMap<PaletteId, Vec<u16>>;
/// Area and theme. An unfinished theme can reference a missing tile without affecting other themes.
pub type PaletteProfiles =
    BTreeMap<usize, BTreeMap<String, Result<AreaPaletteProfile, (PaletteId, TileId)>>>;

pub fn select_palette_profiles<'a, 'b>(
    profiles: &'a PaletteProfiles,
    themes: impl IntoIterator<Item = (usize, &'b str)>,
) -> anyhow::Result<BTreeMap<usize, &'a AreaPaletteProfile>> {
    let mut selected = BTreeMap::new();
    for (id, theme) in themes {
        if let Some(area) = profiles.get(&id) {
            let profile = area
                .get(theme)
                .ok_or_else(|| anyhow::anyhow!("area ${id:02X} has no {theme} theme"))?;
            let profile = profile.as_ref().map_err(|(palette, tile)| {
                anyhow::anyhow!(
                    "area ${id:02X} {theme} references missing tile {tile} in palette {palette}"
                )
            })?;
            selected.insert(id, profile);
        }
    }
    Ok(selected)
}

fn collect_indices(graphic: &TileGraphic) -> u16 {
    let mut mask = 0u16;
    match graphic {
        TileGraphic::Custom { pixels } => {
            for &pixel in pixels.iter().flatten() {
                mask |= 1 << pixel;
            }
        }
        TileGraphic::Vanilla(tile) => {
            for &index in &tile.color_indexes {
                mask |= 1 << index;
            }
        }
    }
    mask & !1
}

pub fn build_palette_profiles(catalog: &RetilingCatalog) -> PaletteProfiles {
    let mut tile_masks = BTreeMap::new();
    for (&id, palette) in &catalog.palettes {
        let mut masks = Vec::new();
        for tile in &palette.tiles {
            masks.push(collect_indices(&tile.graphic));
        }
        for group in &palette.animated_tile_groups {
            let start = usize::from(group.base_tile);
            let tiles = &mut masks[start..start + 16];
            let mut mask = 0;
            for &tile in tiles.iter() {
                mask |= tile;
            }
            for frame in &group.frames {
                for graphic in frame {
                    mask |= collect_indices(graphic);
                }
            }
            tiles.fill(mask);
        }
        tile_masks.insert(id, masks);
    }

    let mut profiles = PaletteProfiles::new();
    for area in catalog.areas.values() {
        let Some(id) = area.vanilla_map_id else {
            continue;
        };
        let area_profiles = profiles.entry(usize::from(id)).or_default();
        'theme: for (theme, source) in &area.themes {
            let mut tiles = BTreeSet::new();
            let mut always_has_bg1 = false;
            for layer in &source.layers {
                let condition = layer.condition.as_deref().unwrap_or("").trim();
                if layer.background == Background::Bg1 && matches!(condition, "" | "true") {
                    always_has_bg1 = true;
                }
                for tile in &layer.grid.tiles {
                    tiles.insert((tile.palette, tile.tile));
                }
            }

            // A replacement can match only if every tile of its before-image is present.
            let mut needed = tiles.clone();
            for group in &catalog.dynamic_tile_groups {
                for variant in &group.variants {
                    if variant
                        .before
                        .tiles
                        .iter()
                        .all(|p| tiles.contains(&(p.palette, p.tile)))
                    {
                        for frame in &variant.after_frames {
                            for p in &frame.tiles {
                                needed.insert((p.palette, p.tile));
                            }
                        }
                    }
                }
            }
            let mut indices = BTreeMap::<PaletteId, u16>::new();
            for (palette, tile) in needed {
                let Some(mask) = tile_masks
                    .get(&palette)
                    .and_then(|m| m.get(usize::from(tile)))
                else {
                    area_profiles.insert(theme.clone(), Err((palette, tile)));
                    continue 'theme;
                };
                *indices.entry(palette).or_default() |= mask;
            }
            if !always_has_bg1 {
                // Rain graphics come from the ROM. Reserve all opaque entries for each
                // rain context that the area's selected layers could request.
                let default = if id < 0x40 {
                    LIGHT_WORLD_RAIN_PALETTE
                } else {
                    DARK_WORLD_RAIN_PALETTE
                };
                indices.entry(default).or_default();
                for palette in [LIGHT_WORLD_RAIN_PALETTE, DARK_WORLD_RAIN_PALETTE] {
                    if let Some(mask) = indices.get_mut(&palette) {
                        *mask = 0xfffe;
                    }
                }
            }
            let mut profile = AreaPaletteProfile::new();
            for (palette, mask) in indices {
                let mut colors = Vec::new();
                for (index, &rgb) in catalog.palettes[&palette].colors.iter().enumerate() {
                    if mask & (1 << index) != 0 {
                        let color = encode_bgr555(rgb);
                        if !colors.contains(&color) {
                            colors.push(color);
                        }
                    }
                }
                profile.insert(palette, colors);
            }
            area_profiles.insert(theme.clone(), Ok(profile));
        }
    }
    profiles
}

struct Component {
    members: Vec<usize>,
    colors: Vec<u16>,
}

#[derive(Default)]
pub struct AllocatedPalette {
    /// Hardware BG row relative to row 2.
    pub row: usize,
    /// Complete component table, starting at opaque index 1.
    pub colors: Vec<u16>,
}

pub fn allocate_palettes(
    areas: &BTreeMap<usize, &AreaPaletteProfile>,
    transitions: &BTreeSet<(usize, usize)>,
    seed: u64,
) -> Option<BTreeMap<PaletteId, AllocatedPalette>> {
    let mut required = AreaPaletteProfile::new();
    for profile in areas.values() {
        for (&id, colors) in *profile {
            let entries = required.entry(id).or_default();
            for color in colors {
                if !entries.contains(color) {
                    entries.push(*color);
                }
            }
        }
    }
    let mut ids = Vec::new();
    let mut indices = BTreeMap::new();
    let mut colors = Vec::new();
    let mut result = BTreeMap::new();
    for (id, entries) in required {
        if entries.is_empty() {
            result.insert(id, AllocatedPalette::default());
        } else {
            indices.insert(id, ids.len());
            ids.push(id);
            colors.push(entries);
        }
    }
    let mut conflicts = vec![vec![false; ids.len()]; ids.len()];
    let mut residents = BTreeMap::new();
    for (&area, profile) in areas {
        let mut palettes = Vec::new();
        for id in profile.keys() {
            if let Some(&i) = indices.get(id) {
                palettes.push(i);
            }
        }
        residents.insert(area, palettes);
    }
    for (a, b) in areas
        .keys()
        .map(|&id| (id, id))
        .chain(transitions.iter().copied())
    {
        let (Some(left), Some(right)) = (residents.get(&a), residents.get(&b)) else {
            continue;
        };
        for &i in left {
            for &j in right {
                if i != j {
                    conflicts[i][j] = true;
                    conflicts[j][i] = true;
                }
            }
        }
    }
    let mut order: Vec<_> = (0..ids.len()).collect();
    order.sort_by_cached_key(|&i| Reverse(conflicts[i].iter().filter(|&&edge| edge).count()));
    // Replay the same attempts at Save ROM without consuming the item-placement RNG.
    let mut rng = StdRng::seed_from_u64(seed);
    'attempt: for _ in 0..PALETTE_ATTEMPTS {
        let p = rng.random_range(0.2..=1.0);
        let mut queue = order.clone();
        let mut rows: [Vec<Component>; 6] = std::array::from_fn(|_| Vec::new());
        let mut merged = Vec::new();
        let mut touched = Vec::new();
        while !queue.is_empty() {
            let mut index = 0;
            while index + 1 < queue.len() && !rng.random_bool(p) {
                index += 1;
            }
            let i = queue.remove(index);
            let mut placed = false;
            // Neighbors assigned to the same row must load identical contents while scrolling.
            'row: for row in &mut rows {
                merged.clone_from(&colors[i]);
                touched.clear();
                for (k, component) in row.iter().enumerate() {
                    if component.members.iter().any(|&j| conflicts[i][j]) {
                        touched.push(k);
                        for color in &component.colors {
                            if !merged.contains(color) {
                                if merged.len() == 15 {
                                    continue 'row;
                                }
                                merged.push(*color);
                            }
                        }
                    }
                }
                let mut members = vec![i];
                for &k in touched.iter().rev() {
                    members.append(&mut row.swap_remove(k).members);
                }
                row.push(Component {
                    members,
                    colors: merged.clone(),
                });
                placed = true;
                break;
            }
            if !placed {
                continue 'attempt;
            }
        }
        for (row, components) in rows.into_iter().enumerate() {
            for component in components {
                for i in component.members {
                    result.insert(
                        ids[i],
                        AllocatedPalette {
                            row,
                            colors: component.colors.clone(),
                        },
                    );
                }
            }
        }
        return Some(result);
    }
    None
}
