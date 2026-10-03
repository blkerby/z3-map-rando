use serde::{Deserialize, Serialize};

pub const TILE_CANONICALIZATION_REVISION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct TileFingerprintIndex {
    pub canonicalization_revision: u32,
    pub rom_sha256: String,
    pub notes: Vec<String>,
    pub sheets: Vec<TileFingerprintSheet>,
}

#[derive(Serialize, Deserialize)]
pub struct TileFingerprintSheet {
    pub sheet: u8,
    /// Array position is the tile offset within the sheet. Duplicates are retained.
    pub tile_fingerprints: Vec<String>,
}

pub struct CanonicalTile {
    pub pixels: [u8; 64],
    /// Map from canonical color indexes to the original tile's color indexes.
    pub color_indexes: Vec<u8>,
    /// Bit 0 flips horizontally; bit 1 flips vertically. Both flips are self-inverse.
    pub flip: u8,
}

pub fn canonicalize_tile(tile: &[[u8; 8]; 8]) -> CanonicalTile {
    let mut best = CanonicalTile {
        pixels: [u8::MAX; 64],
        color_indexes: Vec::new(),
        flip: 0,
    };
    // Preserve the first orientation on ties: none, horizontal, vertical, both.
    for flip in 0..4 {
        let mut pixels = [0; 64];
        let mut color_indexes = Vec::new();
        let mut canonical_indexes = [u8::MAX; 256];
        for y in 0..8 {
            let source_y = if flip & 2 != 0 { 7 - y } else { y };
            for x in 0..8 {
                let source_x = if flip & 1 != 0 { 7 - x } else { x };
                let color = tile[source_y][source_x];
                let index = &mut canonical_indexes[usize::from(color)];
                if *index == u8::MAX {
                    *index = color_indexes.len() as u8;
                    color_indexes.push(color);
                }
                pixels[y * 8 + x] = *index;
            }
        }
        if pixels < best.pixels {
            best = CanonicalTile {
                pixels,
                color_indexes,
                flip,
            };
        }
    }
    best
}

pub fn decode_3bpp_tiles(data: &[u8]) -> Vec<[[u8; 8]; 8]> {
    let mut tiles = Vec::with_capacity(64);
    for tile_index in 0..64 {
        let mut tile = [[0; 8]; 8];
        for (y, row) in tile.iter_mut().enumerate() {
            for (x, pixel) in row.iter_mut().enumerate() {
                let c0 = (data[tile_index * 24 + y * 2] >> (7 - x)) & 1;
                let c1 = (data[tile_index * 24 + y * 2 + 1] >> (7 - x)) & 1;
                let c2 = (data[tile_index * 24 + y + 16] >> (7 - x)) & 1;
                *pixel = c0 | c1 << 1 | c2 << 2;
            }
        }
        tiles.push(tile);
    }
    tiles
}
