use super::{
    TileKey,
    allocation::{CHARACTER_CAPACITY, CharacterSlots},
};
use crate::{
    graphics::{TileGraphicResolver, encode_4bpp_tile},
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
        palettes.insert(id, palette);
    }
    Ok(palettes)
}

fn encode_palette_tile(pixels: &Pixels, indices: &[u8; 16]) -> [u8; 32] {
    let remapped = pixels.map(|row| row.map(|i| indices[usize::from(i)]));
    encode_4bpp_tile(&remapped, false)
}

pub(super) fn build_character_rows(
    tiles: &BTreeSet<TileKey>,
    palettes: &BTreeMap<PaletteId, Palette>,
    color_indices: &BTreeMap<PaletteId, [u8; 16]>,
    slots: &CharacterSlots,
) -> Result<[[u8; 512]; 60]> {
    let mut rows = [[0; 512]; 60];
    for &(palette, tile) in tiles {
        let slot = slots[&(palette, tile)];
        ensure!(slot < CHARACTER_CAPACITY);
        let pixels = &palettes[&palette].tiles[tile].pixels;
        let encoded = encode_palette_tile(pixels, &color_indices[&palette]);
        let offset = slot % 16 * 32;
        rows[slot / 16][offset..offset + 32].copy_from_slice(&encoded);
    }
    Ok(rows)
}

pub(super) fn build_animation_tracks(
    tiles: &BTreeSet<TileKey>,
    palettes: &BTreeMap<PaletteId, Palette>,
    color_indices: &BTreeMap<PaletteId, [u8; 16]>,
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
                    row[tile_index * 32..tile_index * 32 + 32]
                        .copy_from_slice(&encode_palette_tile(pixels, &color_indices[&palette_id]));
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
