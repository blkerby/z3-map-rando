use super::{
    allocation::CharacterSlots,
    areas::{Bg1Variant, Placement, ThemeArea},
    graphics::Palette,
};
use crate::import::{Tile8, Tile16};
use anyhow::{Result, ensure};
use retiling_catalog::PaletteId;
use std::collections::BTreeMap;

pub(super) const MAP16_CAPACITY: usize = 0x4000;
const TRANSPARENT_CHARACTER: u16 = 0x03bf;

pub(super) struct Map16Builder<'a> {
    palettes: &'a BTreeMap<PaletteId, Palette>,
    pub(super) palette_slots: &'a BTreeMap<PaletteId, usize>,
    pub(super) character_slots: &'a CharacterSlots,
    pub(super) definitions: Vec<[u16; 4]>,
    pub(super) properties: [Vec<u8>; 4],
    definition_ids: BTreeMap<[Placement; 4], u16>,
    pub(super) background_ids: BTreeMap<[u16; 4], u16>,
}

impl<'a> Map16Builder<'a> {
    pub(super) fn create(
        vanilla_tiles: &[Tile16],
        vanilla_tile_types: &[u8],
        palettes: &'a BTreeMap<PaletteId, Palette>,
        palette_slots: &'a BTreeMap<PaletteId, usize>,
        character_slots: &'a CharacterSlots,
    ) -> Self {
        let mut definitions = Vec::with_capacity(vanilla_tiles.len());
        for tile in vanilla_tiles {
            definitions.push(tile.map(Tile8::to_vram_tilemap_word));
        }
        let mut properties: [Vec<u8>; 4] = std::array::from_fn(|_| vec![0; MAP16_CAPACITY]);
        for (id, tile) in definitions.iter().enumerate() {
            for quadrant in 0..4 {
                let tile_type = vanilla_tile_types[usize::from(tile[quadrant] & 0x01ff)];
                properties[quadrant][id] = if (0x10..0x1c).contains(&tile_type) {
                    tile_type | ((tile[quadrant] >> 14) as u8 & 1)
                } else {
                    tile_type
                };
            }
        }
        Self {
            palettes,
            palette_slots,
            character_slots,
            definitions,
            properties,
            definition_ids: BTreeMap::new(),
            background_ids: BTreeMap::new(),
        }
    }

    pub(super) fn encode_placement(&self, placement: Placement) -> Result<u16> {
        let tile = &self.palettes[&placement.palette].tiles[placement.tile];
        Ok(
            u16::try_from(self.character_slots[&(placement.palette, placement.tile)])?
                | u16::try_from(2 + self.palette_slots[&placement.palette] / 2)? << 10
                | if tile.priority { 1 << 13 } else { 0 }
                | u16::from(placement.flip) << 14,
        )
    }

    pub(super) fn intern_tile(&mut self, placements: [Placement; 4]) -> Result<u16> {
        let mut words = [0; 4];
        let mut props = [0; 4];
        for (quadrant, placement) in placements.iter().copied().enumerate() {
            let palette = &self.palettes[&placement.palette];
            let tile = &palette.tiles[placement.tile];
            words[quadrant] = self.encode_placement(placement)?;
            props[quadrant] = if (0x10..0x1c).contains(&tile.collision) {
                tile.collision ^ placement.flip
            } else {
                tile.collision
            };
        }
        if let Some(&id) = self.definition_ids.get(&placements) {
            return Ok(id);
        }

        ensure!(
            self.definitions.len() < MAP16_CAPACITY,
            "theme Map16 definitions exceed {MAP16_CAPACITY}"
        );
        let id = u16::try_from(self.definitions.len())?;
        self.definitions.push(words);
        for quadrant in 0..4 {
            self.properties[quadrant][usize::from(id)] = props[quadrant];
        }
        self.definition_ids.insert(placements, id);
        Ok(id)
    }

    pub(super) fn intern_background(&mut self, words: [u16; 4]) -> Result<u16> {
        if let Some(&id) = self.background_ids.get(&words) {
            return Ok(id);
        }
        let id = u16::try_from(self.definitions.len())?;
        self.definitions.push(words);
        self.background_ids.insert(words, id);
        Ok(id)
    }
}

pub(super) fn build_map(
    area: &ThemeArea,
    area_map_x: usize,
    area_map_y: usize,
    map16: &mut Map16Builder<'_>,
) -> Result<Vec<u8>> {
    let mut map = Vec::with_capacity(0x800);
    for map_y in 0..32 {
        for map_x in 0..32 {
            let x = area_map_x * 64 + map_x * 2;
            let y = area_map_y * 64 + map_y * 2;
            let id = map16.intern_tile([
                area.placements[y * area.width + x],
                area.placements[y * area.width + x + 1],
                area.placements[(y + 1) * area.width + x],
                area.placements[(y + 1) * area.width + x + 1],
            ])?;
            map.extend_from_slice(&id.to_le_bytes());
        }
    }
    Ok(map)
}

pub(super) fn build_bg1_maps(
    variant: &Bg1Variant,
    map16: &mut Map16Builder<'_>,
) -> Result<BTreeMap<usize, Vec<u8>>> {
    let Bg1Variant {
        area,
        width,
        height,
        placements,
        ..
    } = variant;
    let (area, width, height) = (*area, *width, *height);
    let mut maps = BTreeMap::new();
    for area_y in 0..height / 64 {
        for area_x in 0..width / 64 {
            let mut output = Vec::with_capacity(0x800);
            for map_y in 0..32 {
                for map_x in 0..32 {
                    let x = area_x * 64 + map_x * 2;
                    let y = area_y * 64 + map_y * 2;
                    let source = [
                        placements[y * width + x],
                        placements[y * width + x + 1],
                        placements[(y + 1) * width + x],
                        placements[(y + 1) * width + x + 1],
                    ];
                    let mut words = [TRANSPARENT_CHARACTER; 4];
                    for (index, placement) in source.into_iter().enumerate() {
                        let Some(placement) = placement else {
                            continue;
                        };
                        words[index] = map16.encode_placement(placement)?;
                    }
                    let id = map16.intern_background(words)?;
                    output.extend_from_slice(&id.to_le_bytes());
                }
            }
            maps.insert(area + area_x + area_y * 8, output);
        }
    }
    Ok(maps)
}
