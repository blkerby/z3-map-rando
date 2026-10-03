use super::{TileKey, areas::ThemeArea, graphics::Palette};
use anyhow::{Context, Result, ensure};
use retiling_catalog::PaletteId;
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

pub(super) fn allocate_palettes(
    areas: &[ThemeArea],
    palettes: &BTreeMap<PaletteId, Palette>,
    scrollable_transitions: &BTreeSet<(usize, usize)>,
) -> Result<BTreeMap<PaletteId, usize>> {
    let mut used = BTreeSet::new();
    for area in areas {
        for &palette in &area.palettes {
            used.insert(palette);
        }
    }
    let mut full = BTreeSet::new();
    let mut conflicts = BTreeMap::new();
    for &id in &used {
        if palettes[&id].uses_upper_half {
            full.insert(id);
        }
        conflicts.insert(id, BTreeSet::new());
    }

    // Palettes in the same area or a scrolling neighbor must use different slots.
    for left_area in areas {
        for right_area in areas {
            if can_areas_coexist(left_area.id, right_area.id, scrollable_transitions) {
                for &left in &left_area.palettes {
                    for &right in &right_area.palettes {
                        if right != left {
                            conflicts.get_mut(&left).unwrap().insert(right);
                        }
                    }
                }
            }
        }
    }

    // Prioritize first assigning palettes that are full size (16 colors)
    // and have many conflicts.
    let mut order = used.into_iter().collect::<Vec<_>>();
    order.sort_by_key(|id| {
        (
            Reverse(full.contains(id)),
            Reverse(conflicts[id].len()),
            *id,
        )
    });
    let mut assignments = BTreeMap::new();

    // Recursively assign palettes with backtracking:
    ensure!(
        assign_palette(0, &order, &full, &conflicts, &mut assignments),
        "theme palettes cannot fit in six BG rows"
    );
    Ok(assignments)
}

fn assign_palette(
    index: usize,
    order: &[PaletteId],
    full: &BTreeSet<PaletteId>,
    conflicts: &BTreeMap<PaletteId, BTreeSet<PaletteId>>,
    assignments: &mut BTreeMap<PaletteId, usize>,
) -> bool {
    let Some(&id) = order.get(index) else {
        return true;
    };
    'candidate: for slot in 0..12 {
        if full.contains(&id) && slot % 2 != 0 {
            continue;
        }
        let end = slot + if full.contains(&id) { 2 } else { 1 };
        for other in &conflicts[&id] {
            let Some(&other_slot) = assignments.get(other) else {
                continue;
            };
            let other_end = other_slot + if full.contains(other) { 2 } else { 1 };
            if slot < other_end && other_slot < end {
                continue 'candidate;
            }
        }

        assignments.insert(id, slot);
        if assign_palette(index + 1, order, full, conflicts, assignments) {
            return true;
        }
        assignments.remove(&id);
    }
    false
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
        while !unassigned.is_empty() {
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
                for &candidate in &unassigned {
                    let mut score = 0;
                    for member in &tiles {
                        for area in &tile_areas[&(palette, candidate)] {
                            if tile_areas[member].contains(area) {
                                score += 1;
                            }
                        }
                    }
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

pub(super) fn build_scrollable_transition_pairs() -> BTreeSet<(usize, usize)> {
    let light_world_pairs = [
        (0x02, 0x0a),
        (0x03, 0x05),
        (0x05, 0x07),
        (0x0a, 0x12),
        (0x0f, 0x17),
        (0x10, 0x18),
        (0x11, 0x12),
        (0x11, 0x18),
        (0x12, 0x13),
        (0x12, 0x1a),
        (0x13, 0x14),
        (0x14, 0x15),
        (0x15, 0x16),
        (0x15, 0x1d),
        (0x16, 0x17),
        (0x18, 0x22),
        (0x18, 0x29),
        (0x1a, 0x1b),
        (0x1b, 0x25),
        (0x1b, 0x2b),
        (0x1b, 0x2c),
        (0x1d, 0x25),
        (0x1e, 0x2e),
        (0x1e, 0x2f),
        (0x25, 0x2d),
        (0x28, 0x29),
        (0x29, 0x2a),
        (0x2a, 0x32),
        (0x2b, 0x2c),
        (0x2b, 0x33),
        (0x2c, 0x2d),
        (0x2c, 0x34),
        (0x2d, 0x2e),
        (0x2d, 0x35),
        (0x2e, 0x35),
        (0x30, 0x3a),
        (0x32, 0x33),
        (0x33, 0x34),
        (0x33, 0x3b),
        (0x34, 0x3c),
        (0x35, 0x3f),
        (0x37, 0x3f),
        (0x3a, 0x3b),
        (0x3b, 0x3c),
        (0x3c, 0x3f),
    ];
    let mut pairs = BTreeSet::new();
    for (left, right) in light_world_pairs {
        pairs.insert((left, right));
        // The Dark World has no scrolling transition between $7A and $7B.
        if (left, right) != (0x3a, 0x3b) {
            pairs.insert((left + 0x40, right + 0x40));
        }
    }
    pairs
}

fn can_areas_coexist(
    left: usize,
    right: usize,
    scrollable_transitions: &BTreeSet<(usize, usize)>,
) -> bool {
    left == right || scrollable_transitions.contains(&(left.min(right), left.max(right)))
}
