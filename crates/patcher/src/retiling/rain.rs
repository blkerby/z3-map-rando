use super::{
    graphics::{Palette, Tile},
    maps::Map16Builder,
};
use crate::{
    graphics::encode_bgr555,
    import::{OverworldAreaAssets, Tile16},
    rain_tilemap::RAIN_TILEMAP,
};
use anyhow::Result;
use retiling_catalog::PaletteId;
use std::collections::BTreeMap;

pub(super) const LIGHT_WORLD_RAIN_PALETTE: PaletteId = 3;
pub(super) const DARK_WORLD_RAIN_PALETTE: PaletteId = 8;
const RAIN_CHARACTERS: [u16; 6] = [0x01ed, 0x009b, 0x01b1, 0x01fd, 0x01a1, 0x01ff];
const RAIN_MAP16S: [u16; 8] = [
    0x026f, 0x0c62, 0x0c63, 0x0c64, 0x0c65, 0x0c66, 0x0c67, 0x0c68,
];

pub(super) fn add_rain_tiles(
    palettes: &mut BTreeMap<PaletteId, Palette>,
    source: &OverworldAreaAssets,
    palette_id: PaletteId,
) -> Vec<usize> {
    let mut target_colors = Vec::new();
    for &color in &palettes[&palette_id].colors {
        target_colors.push(encode_bgr555(color));
    }
    let mut tiles = Vec::with_capacity(RAIN_CHARACTERS.len());
    for &character in &RAIN_CHARACTERS {
        let row = usize::from(character) / 16;
        let column = usize::from(character) % 16;
        let encoded = &source.character_rows[row][column * 32..column * 32 + 32];
        let mut pixels = [[0; 8]; 8];
        for (y, pixels) in pixels.iter_mut().enumerate() {
            for (x, pixel) in pixels.iter_mut().enumerate() {
                let mask = 0x80 >> x;
                let mut source_pixel = 0;
                for plane in 0..4 {
                    let offset = plane / 2 * 16 + y * 2 + plane % 2;
                    if encoded[offset] & mask != 0 {
                        source_pixel |= 1 << plane;
                    }
                }
                if source_pixel == 0 {
                    continue;
                }
                let offset = source_pixel * 2;
                let color = u16::from_le_bytes([
                    source.palette_rows[5][offset],
                    source.palette_rows[5][offset + 1],
                ]);
                *pixel = target_colors
                    .iter()
                    .position(|&target| target == color)
                    .unwrap() as u8;
            }
        }
        let palette = palettes.get_mut(&palette_id).unwrap();
        tiles.push(palette.tiles.len());
        palette.tiles.push(Tile {
            priority: false,
            collision: 0,
            pixels,
            flips: [0, 1, 2, 3],
        });
    }
    tiles
}

pub(super) fn build_rain_maps(
    vanilla_tiles: &[Tile16],
    rain_tiles: &[Vec<usize>; 2],
    map16: &mut Map16Builder<'_>,
) -> Result<[Vec<u8>; 2]> {
    let mut maps = [Vec::with_capacity(0x800), Vec::with_capacity(0x800)];
    for context in 0..2 {
        let palette = if context == 0 {
            LIGHT_WORLD_RAIN_PALETTE
        } else {
            DARK_WORLD_RAIN_PALETTE
        };
        let mut replacements = BTreeMap::new();
        for &source_id in &RAIN_MAP16S {
            let mut words = [0; 4];
            for (quadrant, source) in vanilla_tiles[usize::from(source_id)].iter().enumerate() {
                let source = source.to_vram_tilemap_word();
                let character = source & 0x03ff;
                let tile_index = RAIN_CHARACTERS
                    .iter()
                    .position(|&candidate| candidate == character)
                    .unwrap();
                let tile = rain_tiles[context][tile_index];
                words[quadrant] = u16::try_from(map16.character_slots[&(palette, tile)])?
                    | u16::try_from(2 + map16.palette_slots[&palette] / 2)? << 10
                    | source & 0xe000;
            }
            let id = map16.intern_background(words)?;
            replacements.insert(source_id, id);
        }
        for _ in 0..2 {
            for row in &RAIN_TILEMAP {
                for source in row {
                    maps[context].extend_from_slice(&replacements[source].to_le_bytes());
                }
            }
        }
    }
    Ok(maps)
}
