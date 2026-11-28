use crate::{Patcher, PcAddr, SnesAddr, import::Importer};
use anyhow::Result;
use retiling_catalog::{Flip, TileGraphic, graphics::canonicalize_tile};
use std::collections::BTreeMap;

/// Fit the vanilla A–Z glyphs (IDs $AA–$C3) and their outlines into eight pixels,
/// by shifting them left a pixel.
pub(crate) fn shift_uppercase_font(patcher: &mut Patcher, rom: &[u8]) -> Result<()> {
    let font_start = PcAddr::from(SnesAddr(0x8e9173));
    let masks_start = PcAddr::from(SnesAddr(0x8ebb96)).0 as usize;
    let mut source = font_start.0 as usize;
    let mut packed = Vec::new();
    for mask in rom[masks_start..masks_start + 26 * 5].chunks_exact(5) {
        // Each set bit supplies one byte, starting at byte four of the decompressed glyph.
        let mut glyph = [0u8; 44];
        let mut stored_offsets = Vec::new();
        for (group, &bits) in mask.iter().enumerate() {
            for bit in 0..8 {
                if bits & (0x80 >> bit) != 0 {
                    let offset = 4 + group * 8 + bit;
                    glyph[offset] = rom[source];
                    source += 1;
                    stored_offsets.push(offset);
                }
            }
        }
        // Each half has an 8×8 tile followed by three vertical columns, with two planes each.
        for half in glyph.chunks_exact_mut(22) {
            for row in 0..8 {
                for plane in 0..2 {
                    let ninth_pixel = (half[16 + plane] >> (7 - row)) & 1;
                    let byte = &mut half[2 * row + plane];
                    *byte = (*byte << 1) | ninth_pixel;
                }
            }
            half[16..].fill(0);
        }
        for offset in stored_offsets {
            packed.push(glyph[offset]);
        }
    }
    patcher.context("uppercase font").write(font_start, packed)
}

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

pub(crate) use retiling_catalog::graphics::encode_bgr555;

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
