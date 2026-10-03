use super::{
    TileKey,
    allocation::{CHARACTER_CAPACITY, CharacterSlots},
    areas::ThemeArea,
};
use crate::{
    graphics::{TileGraphicResolver, encode_4bpp_tile, encode_bgr555},
    import::OverworldAnimationTrack,
};
use anyhow::{Result, ensure};
use retiling_catalog::{PaletteId, RetilingCatalog};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Pixels = [[u8; 8]; 8];

pub(super) struct Palette {
    pub(super) colors: [[u8; 3]; 16],
    pub(super) tiles: Vec<Tile>,
    pub(super) animated_tile_groups: Vec<AnimatedTileGroup>,
    pub(super) uses_upper_half: bool,
}

pub(super) struct AnimatedTileGroup {
    pub(super) base_tile: usize,
    pub(super) frames: Vec<Vec<Pixels>>,
    pub(super) frame_hold: u8,
    pub(super) phase_offset: usize,
}

pub(super) struct Tile {
    pub(super) priority: bool,
    pub(super) collision: u8,
    pub(super) pixels: Pixels,
    pub(super) flips: [u8; 4],
}

impl Tile {
    pub(super) fn canonicalize_flips(&mut self) {
        let mut flips = [0, 1, 2, 3];
        if !(0x10..0x1c).contains(&self.collision) {
            for flip in 1..4 {
                for candidate in 0..flip {
                    let mut identical = true;
                    for y in 0..8 {
                        for x in 0..8 {
                            let flip_x = if flip & 1 != 0 { 7 - x } else { x };
                            let flip_y = if flip & 2 != 0 { 7 - y } else { y };
                            let candidate_x = if candidate & 1 != 0 { 7 - x } else { x };
                            let candidate_y = if candidate & 2 != 0 { 7 - y } else { y };
                            if self.pixels[flip_y][flip_x] != self.pixels[candidate_y][candidate_x]
                            {
                                identical = false;
                                break;
                            }
                        }
                        if !identical {
                            break;
                        }
                    }
                    if identical {
                        flips[flip] = candidate as u8;
                        break;
                    }
                }
            }
        }
        self.flips = flips;
    }
}

pub(super) fn resolve_palettes(
    catalog: &RetilingCatalog,
    graphics: &mut TileGraphicResolver<'_>,
) -> Result<BTreeMap<PaletteId, Palette>> {
    let mut palettes = BTreeMap::new();
    for (&id, source) in &catalog.palettes {
        let mut palette = Palette {
            colors: source.colors,
            tiles: Vec::new(),
            animated_tile_groups: Vec::new(),
            uses_upper_half: false,
        };
        for tile in &source.tiles {
            palette.tiles.push(Tile {
                priority: tile.priority,
                collision: tile.collision,
                pixels: graphics.resolve(&tile.graphic)?,
                flips: [0, 1, 2, 3],
            });
        }
        for group in &source.animated_tile_groups {
            let mut frames = Vec::new();
            for frame in &group.frames {
                let mut tiles = Vec::new();
                for graphic in frame {
                    tiles.push(graphics.resolve(graphic)?);
                }
                frames.push(tiles);
            }
            palette.animated_tile_groups.push(AnimatedTileGroup {
                base_tile: usize::from(group.base_tile),
                frames,
                frame_hold: u8::try_from(group.frame_hold)?,
                phase_offset: usize::from(group.phase_offset),
            });
        }
        palette.uses_upper_half = palette
            .tiles
            .iter()
            .any(|tile| tile.pixels.iter().flatten().any(|&pixel| pixel >= 8));
        if !palette.uses_upper_half {
            for group in &palette.animated_tile_groups {
                for frame in &group.frames {
                    for tile in frame {
                        if tile.iter().flatten().any(|&pixel| pixel >= 8) {
                            palette.uses_upper_half = true;
                        }
                    }
                }
            }
        }
        palettes.insert(id, palette);
    }
    Ok(palettes)
}

pub(super) fn build_palette_rows(
    area: &ThemeArea,
    palettes: &BTreeMap<PaletteId, Palette>,
    slots: &BTreeMap<PaletteId, usize>,
) -> [[u8; 32]; 6] {
    let mut rows = [[0; 32]; 6];
    for id in &area.palettes {
        let palette = &palettes[id];
        let half = slots[id];
        let row = half / 2;
        let start = if palette.uses_upper_half {
            1
        } else {
            half % 2 * 8 + 1
        };
        let count = if palette.uses_upper_half { 15 } else { 7 };
        for (index, color) in palette.colors[1..=count].iter().enumerate() {
            let output = (start + index) * 2;
            rows[row][output..output + 2].copy_from_slice(&encode_bgr555(*color).to_le_bytes());
        }
    }
    rows
}

pub(super) fn build_character_rows(
    tiles: &BTreeSet<TileKey>,
    palettes: &BTreeMap<PaletteId, Palette>,
    palette_slots: &BTreeMap<PaletteId, usize>,
    slots: &CharacterSlots,
) -> Result<[[u8; 512]; 60]> {
    let mut rows = [[0; 512]; 60];
    for &(palette, tile) in tiles {
        let slot = slots[&(palette, tile)];
        ensure!(slot < CHARACTER_CAPACITY);
        let pixels = &palettes[&palette].tiles[tile].pixels;
        let upper_half = !palettes[&palette].uses_upper_half && palette_slots[&palette] % 2 == 1;
        let encoded = encode_4bpp_tile(pixels, upper_half);
        let offset = slot % 16 * 32;
        rows[slot / 16][offset..offset + 32].copy_from_slice(&encoded);
    }
    Ok(rows)
}

pub(super) fn build_animation_tracks(
    tiles: &BTreeSet<TileKey>,
    palettes: &BTreeMap<PaletteId, Palette>,
    palette_slots: &BTreeMap<PaletteId, usize>,
    character_slots: &CharacterSlots,
) -> Vec<OverworldAnimationTrack> {
    let mut tracks = Vec::new();
    for (&palette_id, palette) in palettes {
        for group in &palette.animated_tile_groups {
            if !tiles.contains(&(palette_id, group.base_tile)) {
                continue;
            }
            let mut frames = Vec::with_capacity(group.frames.len() + 1);
            for frame_index in 0..=group.frames.len() {
                let mut row = [0; 512];
                for tile_index in 0..16 {
                    let pixels = if frame_index == 0 {
                        &palette.tiles[group.base_tile + tile_index].pixels
                    } else {
                        &group.frames[frame_index - 1][tile_index]
                    };
                    let upper_half =
                        !palette.uses_upper_half && palette_slots[&palette_id] % 2 == 1;
                    row[tile_index * 32..tile_index * 32 + 32]
                        .copy_from_slice(&encode_4bpp_tile(pixels, upper_half));
                }
                frames.push(vec![row]);
            }
            tracks.push(OverworldAnimationTrack {
                destination_rows: vec![
                    (character_slots[&(palette_id, group.base_tile)] / 16) as u8,
                ],
                frames,
                frame_hold: group.frame_hold,
                phase_offset: group.phase_offset,
            });
        }
    }
    tracks
}
