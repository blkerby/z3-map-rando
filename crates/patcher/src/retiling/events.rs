use super::{
    TileKey,
    areas::{Placement, StateLayer, ThemeArea, ThemeCutscene},
    maps::Map16Builder,
};
use crate::asset_bundle::{
    CompiledCutscene, CompiledOverworldOverlay, DYNAMIC_TILE_GROUP_COUNT, DynamicTileEntry,
};
use anyhow::Result;
use retiling_catalog::{CutsceneAction, DynamicTileType, PaletteId};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const VANILLA_OVERWORLD_OVERLAYS: [(u8, &str); 23] = [
    (0x02, "Lumberjack tree"),
    (0x07, "Turtle Rock portal"),
    (0x13, "Sanctuary stairs"),
    (0x14, "King's Tomb"),
    (0x18, "Bird Statue"),
    (0x1b, "Hyrule Castle gate"),
    (0x2b, "Bonk Fairies stairs"),
    (0x30, "Checkerboard Cave stairs"),
    (0x37, "Ice Rod Cave stairs"),
    (0x3a, "Desert thief stairs"),
    (0x3b, "drained dam"),
    (0x40, "Skull Woods entrance"),
    (0x43, "Ganon's Tower entrance"),
    (0x45, "Hookshot Cave stairs"),
    (0x47, "Turtle Rock entrance"),
    (0x58, "Thieves' Town entrance"),
    (0x5b, "Pyramid hole"),
    (0x5e, "Palace of Darkness entrance"),
    (0x62, "peg-puzzle stairs"),
    (0x6b, "Dark Bonk Fairies stairs"),
    (0x70, "Misery Mire entrance"),
    (0x77, "Shopping Mall stairs"),
    (0x7b, "drained dam"),
];

pub(super) type DynamicTiling = Vec<Vec<Placement>>;

pub(super) struct DynamicTileGroup {
    pub(super) kind: DynamicTileType,
    pub(super) variants: Vec<DynamicTileVariant>,
}

pub(super) struct DynamicTileVariant {
    pub(super) before: DynamicTiling,
    pub(super) after_frames: Vec<DynamicTiling>,
    pub(super) used: bool,
}

pub(super) fn expand_dynamic_grid(grid: &retiling_catalog::TileGrid) -> DynamicTiling {
    let width = usize::from(grid.width);
    let height = usize::from(grid.height);
    let mut tiles = vec![
        vec![
            Placement {
                palette: 0,
                tile: 0,
                flip: 0
            };
            width
        ];
        height
    ];
    for placement in &grid.tiles {
        tiles[usize::from(placement.y)][usize::from(placement.x)] = Placement {
            palette: placement.palette,
            tile: usize::from(placement.tile),
            flip: placement.flip as u8,
        };
    }
    tiles
}

pub(super) fn add_dynamic_dependencies(
    areas: &mut [ThemeArea],
    dynamic_tiles: &mut [DynamicTileGroup],
) {
    let mut area_palettes = BTreeMap::<usize, BTreeSet<PaletteId>>::new();
    let mut area_tiles = BTreeMap::<usize, BTreeSet<TileKey>>::new();
    for group in dynamic_tiles {
        for variant in &mut group.variants {
            let height = variant.before.len();
            let width = variant.before[0].len();
            let mut matching_areas = BTreeSet::new();
            for area in areas.iter() {
                for start_y in 0..=area.height - height {
                    'position: for start_x in 0..=area.width - width {
                        for y in 0..height {
                            for x in 0..width {
                                if area.placements[(start_y + y) * area.width + start_x + x]
                                    != variant.before[y][x]
                                {
                                    continue 'position;
                                }
                            }
                        }
                        matching_areas.insert(area.id);
                    }
                }
            }
            for frame in &variant.after_frames {
                for row in frame {
                    for placement in row {
                        for &area in &matching_areas {
                            area_palettes
                                .entry(area)
                                .or_default()
                                .insert(placement.palette);
                            area_tiles
                                .entry(area)
                                .or_default()
                                .insert((placement.palette, placement.tile));
                        }
                    }
                }
            }
            variant.used = !matching_areas.is_empty();
        }
    }

    for area in areas {
        if let Some(palettes) = area_palettes.get(&area.id) {
            area.palettes.extend(palettes);
        }
        if let Some(tiles) = area_tiles.get(&area.id) {
            area.extra_tiles.extend(tiles);
        }
    }
}

pub(super) fn build_cutscenes(
    areas: &[ThemeArea],
    cutscenes: &[ThemeCutscene],
    map16: &mut Map16Builder<'_>,
) -> Result<(Vec<CompiledCutscene>, Vec<CompiledOverworldOverlay>)> {
    let mut result = Vec::new();
    let mut overlays = Vec::new();
    for cutscene in cutscenes {
        let area = areas.iter().find(|area| area.id == cutscene.area).unwrap();
        let mut state = area.placements.clone();
        let mut persistent = BTreeMap::new();
        let mut script = Vec::new();
        for action in &cutscene.actions {
            match action {
                CutsceneAction::Wait { frames } => {
                    script.push(1);
                    script.push(*frames);
                }
                CutsceneAction::PlaySound { channel, sound } => {
                    script.push(2);
                    script.push(*channel);
                    script.push(*sound);
                }
                CutsceneAction::PlayMusic { song } => {
                    script.push(3);
                    script.push(*song);
                }
                CutsceneAction::Draw { layer } => {
                    let layer = &cutscene.layers[layer];
                    let writes = build_layer_writes(area, &mut state, layer, map16)?;
                    script.push(4);
                    script.push(u8::try_from(writes.len())?);
                    for (offset, id) in writes {
                        script.extend_from_slice(&offset.to_le_bytes());
                        script.extend_from_slice(&id.to_le_bytes());
                        persistent.insert(offset, id);
                    }
                }
                CutsceneAction::SetComplete => script.push(5),
                CutsceneAction::StartShake => script.push(6),
                CutsceneAction::StopShake => script.push(7),
                CutsceneAction::End => script.push(0),
            }
        }
        let mut persistent_writes = Vec::with_capacity(persistent.len());
        for write in persistent {
            persistent_writes.push(write);
        }
        let mut cutscene_areas = Vec::new();
        for map_y in 0..area.height / 64 {
            for map_x in 0..area.width / 64 {
                cutscene_areas.push(u8::try_from(area.id + map_x + map_y * 8)?);
            }
        }
        result.push(CompiledCutscene {
            trigger: cutscene.trigger,
            script,
        });
        overlays.push(CompiledOverworldOverlay {
            areas: cutscene_areas,
            writes: persistent_writes,
        });
    }
    result.sort_by_key(|cutscene| cutscene.trigger);
    Ok((result, overlays))
}

pub(super) fn build_dynamic_overworld_overlays(
    areas: &[ThemeArea],
    screen_maps: &BTreeMap<usize, Vec<u8>>,
    dynamic_tile_groups: &[Vec<DynamicTileEntry>],
) -> Result<Vec<CompiledOverworldOverlay>> {
    // These fixed locations are the persistent event-bit overlays selected by
    // vanilla. Ordinary dynamic-tile interactions remain location-independent.
    let specs = [
        (0x13, 0x0506, DynamicTileType::SecretStairs),
        (0x14, 0x0532, DynamicTileType::GraveStairs),
        (0x1b, 0x13bc, DynamicTileType::HyruleCastleGate),
        (0x2b, 0x0330, DynamicTileType::SecretStairs),
        (0x30, 0x0358, DynamicTileType::SecretStairs),
        (0x37, 0x040c, DynamicTileType::SecretStairs),
        (0x3a, 0x0a1e, DynamicTileType::SecretStairs),
        (0x45, 0x0868, DynamicTileType::SecretStairs),
        (0x6b, 0x0330, DynamicTileType::SecretStairs),
        (0x77, 0x040c, DynamicTileType::SecretStairs),
    ];
    let mut overlays = Vec::new();
    for (area_id, offset, kind) in specs {
        let Some(area) = areas.iter().find(|area| area.id == area_id) else {
            continue;
        };
        let x = offset % 0x80 / 2;
        let y = offset / 0x80;
        let screen_id = area.id + x / 32 + y / 32 * 8;
        let Some(map) = screen_maps.get(&screen_id) else {
            continue;
        };
        let map_index = ((y % 32) * 32 + x % 32) * 2;
        let source = u16::from_le_bytes([map[map_index], map[map_index + 1]]);
        let Some(entry) = dynamic_tile_groups[kind as usize]
            .iter()
            .find(|entry| entry.source == source)
        else {
            continue;
        };
        let Some(after) = entry.after_frames.first() else {
            continue;
        };
        let origin_x = isize::try_from(x)? + isize::from(entry.x_offset);
        let origin_y = isize::try_from(y)? + isize::from(entry.y_offset);
        let width = usize::from(entry.width);
        let mut writes = Vec::with_capacity(after.len());
        for row in 0..entry.height {
            for column in 0..entry.width {
                let write_offset =
                    (origin_y + isize::from(row)) * 0x80 + (origin_x + isize::from(column)) * 2;
                writes.push((
                    u16::try_from(write_offset)?,
                    after[usize::from(row) * width + usize::from(column)],
                ));
            }
        }
        let mut overlay_areas = Vec::new();
        for map_y in 0..area.height / 64 {
            for map_x in 0..area.width / 64 {
                overlay_areas.push(u8::try_from(area.id + map_x + map_y * 8)?);
            }
        }
        overlays.push(CompiledOverworldOverlay {
            areas: overlay_areas,
            writes,
        });
    }
    Ok(overlays)
}

pub(super) fn build_overworld_overlays(
    areas: &[ThemeArea],
    map16: &mut Map16Builder<'_>,
) -> Result<Vec<CompiledOverworldOverlay>> {
    let mut overlays = Vec::new();
    for area in areas {
        let Some(layer) = &area.overworld_overlay else {
            continue;
        };
        let mut state = area.placements.clone();
        let writes = build_layer_writes(area, &mut state, layer, map16)?;
        let mut overlay_areas = Vec::new();
        for map_y in 0..area.height / 64 {
            for map_x in 0..area.width / 64 {
                overlay_areas.push(u8::try_from(area.id + map_x + map_y * 8)?);
            }
        }
        overlays.push(CompiledOverworldOverlay {
            areas: overlay_areas,
            writes,
        });
    }
    Ok(overlays)
}

fn build_layer_writes(
    area: &ThemeArea,
    state: &mut [Placement],
    layer: &StateLayer,
    map16: &mut Map16Builder<'_>,
) -> Result<Vec<(u16, u16)>> {
    let mut cells = BTreeSet::new();
    for (index, &placement) in layer.placements.iter().enumerate() {
        let Some(placement) = placement else {
            continue;
        };
        state[index] = placement;
        cells.insert((index % area.width / 2, index / area.width / 2));
    }
    let mut writes = Vec::with_capacity(cells.len());
    for (x, y) in cells {
        let top_left = y * 2 * area.width + x * 2;
        let id = map16.intern_tile([
            state[top_left],
            state[top_left + 1],
            state[top_left + area.width],
            state[top_left + area.width + 1],
        ])?;
        let offset = u16::try_from(y * 0x80 + x * 2)?;
        writes.push((offset, id));
    }
    Ok(writes)
}

pub(super) fn build_dynamic_tile_groups(
    dynamic_tiles: &[DynamicTileGroup],
    map16: &mut Map16Builder<'_>,
) -> Result<Vec<Vec<DynamicTileEntry>>> {
    let mut result = Vec::with_capacity(DYNAMIC_TILE_GROUP_COUNT);
    for _ in 0..DYNAMIC_TILE_GROUP_COUNT {
        result.push(Vec::new());
    }
    for group in dynamic_tiles {
        for variant in &group.variants {
            if !variant.used {
                continue;
            }
            let before = build_tiling(&variant.before, map16)?;
            let mut after_frames = Vec::with_capacity(variant.after_frames.len());
            for frame in &variant.after_frames {
                after_frames.push(build_tiling(frame, map16)?);
            }
            let width = variant.before[0].len() / 2;
            let height = variant.before.len() / 2;
            if matches!(
                group.kind,
                DynamicTileType::CutGrass
                    | DynamicTileType::DigTerrain
                    | DynamicTileType::GreenBush
                    | DynamicTileType::HeavyBush
                    | DynamicTileType::HammerPeg
                    | DynamicTileType::LiftSign
                    | DynamicTileType::SmallGrayRock
                    | DynamicTileType::SmallBlackRock
                    | DynamicTileType::SecretHole
                    | DynamicTileType::SecretPortal
                    | DynamicTileType::SecretBombableEntrance
                    | DynamicTileType::WoodenDoor
                    | DynamicTileType::SanctuaryDoor
                    | DynamicTileType::HyruleCastleDoor
                    | DynamicTileType::GraveCorpse
                    | DynamicTileType::GraveStairs
                    | DynamicTileType::GravePit
                    | DynamicTileType::HyruleCastleGate
            ) {
                result[group.kind as usize].push(DynamicTileEntry {
                    source: before[0],
                    x_offset: 0,
                    y_offset: 0,
                    width: u8::try_from(width)?,
                    height: u8::try_from(height)?,
                    before,
                    after_frames,
                });
                continue;
            }
            for (index, &source) in before.iter().enumerate() {
                let mut is_anchor = false;
                for quadrant in map16.properties.iter() {
                    let property = quadrant[usize::from(source)];
                    let matches_anchor = match group.kind {
                        DynamicTileType::LargeGrayRock => property == 0x55,
                        DynamicTileType::LargeBlackRock => property == 0x56,
                        DynamicTileType::RockPile => property == 0x57,
                        DynamicTileType::SecretStairs => property == 0x55 || property == 0x57,
                        _ => false,
                    };
                    if matches_anchor {
                        is_anchor = true;
                        break;
                    }
                }
                if !is_anchor {
                    continue;
                }
                result[group.kind as usize].push(DynamicTileEntry {
                    source,
                    x_offset: -i8::try_from(index % width)?,
                    y_offset: -i8::try_from(index / width)?,
                    width: u8::try_from(width)?,
                    height: u8::try_from(height)?,
                    before: before.clone(),
                    after_frames: after_frames.clone(),
                });
            }
        }
    }
    Ok(result)
}

fn build_tiling(tiling: &DynamicTiling, map16: &mut Map16Builder<'_>) -> Result<Vec<u16>> {
    let mut result = Vec::new();
    for y in (0..tiling.len()).step_by(2) {
        for x in (0..tiling[0].len()).step_by(2) {
            result.push(map16.intern_tile([
                tiling[y][x],
                tiling[y][x + 1],
                tiling[y + 1][x],
                tiling[y + 1][x + 1],
            ])?);
        }
    }
    Ok(result)
}
