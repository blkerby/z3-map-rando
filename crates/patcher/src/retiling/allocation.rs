use super::{TileKey, graphics::Palette};
use anyhow::{Context, Result};
use retiling_catalog::{PaletteId, resolved::ThemeArea};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
};

pub(super) const CHARACTER_CAPACITY: usize = 960;
pub(super) type CharacterSlots = BTreeMap<TileKey, usize>;
type AreaTiles = Vec<BTreeSet<TileKey>>;

struct GraphicGroup {
    tiles: Vec<TileKey>,
    areas: BTreeSet<usize>,
}

pub(super) fn allocate_characters(
    areas: &[ThemeArea],
    palettes: &BTreeMap<PaletteId, Palette>,
    scrollable_transitions: &BTreeSet<(usize, usize)>,
) -> Result<(CharacterSlots, AreaTiles)> {
    let mut tile_areas = BTreeMap::<TileKey, BTreeSet<usize>>::new();
    for area in areas {
        for placement in &area.placements {
            tile_areas
                .entry((placement.palette, placement.tile))
                .or_default()
                .insert(area.id);
        }
        for &tile in &area.extra_tiles {
            tile_areas.entry(tile).or_default().insert(area.id);
        }
    }

    let mut groups = Vec::new();
    let mut palette_tiles = BTreeMap::<PaletteId, BTreeSet<usize>>::new();
    for &(palette, tile) in tile_areas.keys() {
        palette_tiles.entry(palette).or_default().insert(tile);
    }
    for (palette, mut unassigned) in palette_tiles {
        for animation in &palettes[&palette].animated_tile_groups {
            let mut tiles = Vec::with_capacity(16);
            let mut areas = BTreeSet::new();
            for tile in animation.base_tile..animation.base_tile + 16 {
                let key = (palette, tile);
                if let Some(tile_areas) = tile_areas.get(&key) {
                    areas.extend(tile_areas);
                }
                unassigned.remove(&tile);
                tiles.push(key);
            }
            if !areas.is_empty() {
                groups.push(GraphicGroup { tiles, areas });
            }
        }
        let mut scores = vec![0; palettes[&palette].tiles.len()];
        while !unassigned.is_empty() {
            scores.fill(0);
            let mut seed = *unassigned.first().unwrap();
            for &tile in &unassigned {
                if tile_areas[&(palette, tile)].len() > tile_areas[&(palette, seed)].len() {
                    seed = tile;
                }
            }
            unassigned.remove(&seed);
            let mut tiles = vec![(palette, seed)];
            while tiles.len() < 16 && !unassigned.is_empty() {
                let mut best = *unassigned.first().unwrap();
                let mut best_score = 0;
                // The running scores already include every earlier member of this row.
                let member_areas = &tile_areas[tiles.last().unwrap()];
                for &candidate in &unassigned {
                    for area in &tile_areas[&(palette, candidate)] {
                        if member_areas.contains(area) {
                            scores[candidate] += 1;
                        }
                    }
                    let score = scores[candidate];
                    if score > best_score || score == best_score && candidate < best {
                        best = candidate;
                        best_score = score;
                    }
                }
                unassigned.remove(&best);
                tiles.push((palette, best));
            }
            let mut areas = BTreeSet::new();
            for tile in &tiles {
                areas.extend(&tile_areas[tile]);
            }
            groups.push(GraphicGroup { tiles, areas });
        }
    }

    let mut neighboring_areas = BTreeMap::<usize, BTreeSet<usize>>::new();
    for left in areas {
        for right in areas {
            if can_areas_coexist(left.id, right.id, scrollable_transitions) {
                neighboring_areas
                    .entry(left.id)
                    .or_default()
                    .insert(right.id);
            }
        }
    }
    let mut conflicts = vec![BTreeSet::new(); groups.len()];
    for left in 0..groups.len() {
        for right in left + 1..groups.len() {
            let mut conflict = false;
            for area in &groups[left].areas {
                if neighboring_areas[area]
                    .iter()
                    .any(|neighbor| groups[right].areas.contains(neighbor))
                {
                    conflict = true;
                    break;
                }
            }
            if conflict {
                conflicts[left].insert(right);
                conflicts[right].insert(left);
            }
        }
    }

    let mut order = (0..groups.len()).collect::<Vec<_>>();
    order.sort_by_key(|&group| (Reverse(conflicts[group].len()), group));
    let mut group_rows = BTreeMap::new();
    for group in order {
        let mut blocked = BTreeSet::new();
        for other in &conflicts[group] {
            if let Some(&row) = group_rows.get(other) {
                blocked.insert(row);
            }
        }
        let row = (0..CHARACTER_CAPACITY / 16 - 1)
            .find(|row| !blocked.contains(row))
            .context("Graphics exceed the existing stable character rows")?;
        group_rows.insert(group, row);
    }

    let mut slots = BTreeMap::new();
    for (group, group_data) in groups.iter().enumerate() {
        for (column, &tile) in group_data.tiles.iter().enumerate() {
            slots.insert(tile, group_rows[&group] * 16 + column);
        }
    }

    let mut area_tiles = Vec::with_capacity(areas.len());
    for area in areas {
        let mut tiles = BTreeSet::new();
        for group in &groups {
            if group.areas.contains(&area.id) {
                tiles.extend(&group.tiles);
            }
        }
        area_tiles.push(tiles);
    }
    Ok((slots, area_tiles))
}

fn can_areas_coexist(
    left: usize,
    right: usize,
    scrollable_transitions: &BTreeSet<(usize, usize)>,
) -> bool {
    left == right || scrollable_transitions.contains(&(left.min(right), left.max(right)))
}
