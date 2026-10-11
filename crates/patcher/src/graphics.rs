use crate::import::Importer;
use anyhow::Result;
use retiling_catalog::{Flip, TileGraphic, graphics::canonicalize_tile};
use std::collections::BTreeMap;

/// Resolve catalog graphics while decoding each referenced ROM sheet only once.
pub struct TileGraphicResolver<'a> {
    importer: &'a Importer,
    sheets: BTreeMap<u8, Vec<[[u8; 8]; 8]>>,
}

impl<'a> TileGraphicResolver<'a> {
    pub fn create(importer: &'a Importer) -> Self {
        Self {
            importer,
            sheets: BTreeMap::new(),
        }
    }

    pub fn resolve(&mut self, graphic: &TileGraphic) -> Result<[[u8; 8]; 8]> {
        let reference = match graphic {
            TileGraphic::Custom { pixels } => return Ok(*pixels),
            TileGraphic::Vanilla(reference) => reference,
        };
        if !self.sheets.contains_key(&reference.sheet) {
            self.sheets.insert(
                reference.sheet,
                self.importer
                    .read_background_graphics_sheet(reference.sheet)?,
            );
        }
        let canonical =
            canonicalize_tile(&self.sheets[&reference.sheet][usize::from(reference.tile)]);
        let mut pixels = [[0; 8]; 8];
        for (y, row) in pixels.iter_mut().enumerate() {
            let source_y = match reference.flip {
                Flip::Vertical | Flip::Both => 7 - y,
                _ => y,
            };
            for (x, pixel) in row.iter_mut().enumerate() {
                let source_x = match reference.flip {
                    Flip::Horizontal | Flip::Both => 7 - x,
                    _ => x,
                };
                let index = canonical.pixels[source_y * 8 + source_x];
                *pixel = reference.color_indexes[usize::from(index)];
            }
        }
        Ok(pixels)
    }
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

pub(crate) fn encode_bgr555([red, green, blue]: [u8; 3]) -> u16 {
    u16::from(red) | u16::from(green) << 5 | u16::from(blue) << 10
}

pub(crate) fn encode_4bpp_tile(pixels: &[[u8; 8]; 8], upper_half: bool) -> [u8; 32] {
    let mut output = [0; 32];
    for y in 0..8 {
        for x in 0..8 {
            let pixel = pixels[y][x];
            let pixel = if upper_half && pixel != 0 {
                pixel + 8
            } else {
                pixel
            };
            let mask = 0x80 >> x;
            if pixel & 1 != 0 {
                output[y * 2] |= mask;
            }
            if pixel & 2 != 0 {
                output[y * 2 + 1] |= mask;
            }
            if pixel & 4 != 0 {
                output[16 + y * 2] |= mask;
            }
            if pixel & 8 != 0 {
                output[16 + y * 2 + 1] |= mask;
            }
        }
    }
    output
}
