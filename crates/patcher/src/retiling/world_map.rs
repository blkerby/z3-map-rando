use super::{
    areas::{Placement, ThemeArea},
    graphics::Palette,
};
use crate::graphics::encode_bgr555;
use anyhow::{Context, Result, ensure};
use retiling_catalog::{Background, PaletteId, RetilingCatalog};
use seed_data::WorldLayout;
use std::collections::BTreeMap;

pub struct CompiledWorldMap {
    pub graphics: Vec<u8>,
    pub tilemaps: Vec<u8>,
    pub palettes: Vec<u8>,
}

pub(super) fn build_world_map(
    areas: &[ThemeArea],
    palettes: &BTreeMap<PaletteId, Palette>,
    layout: &WorldLayout,
    catalog: &RetilingCatalog,
) -> Result<CompiledWorldMap> {
    let mut worlds = [vec![None; 4096], vec![None; 4096]];
    for (world, name) in ["Light World Clouds", "Dark World Clouds"]
        .into_iter()
        .enumerate()
    {
        let clouds = &catalog.areas[name].themes["Base"];
        for layer in &clouds.layers {
            if layer.background != Background::Bg2 || layer.name != "Main" {
                continue;
            }
            for placement in &layer.grid.tiles {
                worlds[world][usize::from(placement.y) * 64 + usize::from(placement.x)] =
                    Some(Placement {
                        palette: placement.palette,
                        tile: usize::from(placement.tile),
                        flip: placement.flip as u8,
                    });
            }
        }
    }
    for area in areas {
        if area.id >= 0x80 {
            continue;
        }
        let logical = area.id as u8 & 0x3f;
        let anchor = if layout.rooms.is_empty() {
            logical
        } else {
            layout
                .rooms
                .iter()
                .find(|room| room.vanilla_map_id == logical)
                .unwrap()
                .position
        };
        let origin_x = 16 + usize::from(anchor % 8) * 4;
        let origin_y = 16 + usize::from(anchor / 8) * 4;
        let world = if area.id < 0x40 { 0 } else { 1 };
        let width = area.width / 16;
        for (index, placement) in area.map_tiles.iter().enumerate() {
            let placement = placement.with_context(|| {
                format!(
                    "area ${:02X} has no authored map tile at ({}, {}); update the retiling source",
                    area.id,
                    index % width,
                    index / width
                )
            })?;
            worlds[world][(origin_y + index / width) * 64 + origin_x + index % width] =
                Some(placement);
        }
    }
    let mut result = CompiledWorldMap {
        graphics: Vec::new(),
        tilemaps: Vec::new(),
        palettes: Vec::new(),
    };
    for (world, placements) in worlds.into_iter().enumerate() {
        let mut color_ids = BTreeMap::from([(0, 0u8)]);
        let mut colors = vec![0; 256];
        let mut characters = vec![0; 0x4000];
        let mut tilemap = vec![0; 4096];
        // The engine uses character $EF to fill beyond the map; keep it blank.
        let mut tile_ids = BTreeMap::from([([0; 64], 0xefu8)]);
        let mut next_id = 0usize;
        for y in 0..64 {
            for x in 0..64 {
                let mut tile = [0; 64];
                if let Some(placement) = placements[y * 64 + x] {
                    let palette = &palettes[&placement.palette];
                    let source = &palette.tiles[placement.tile];
                    for dy in 0..8 {
                        for dx in 0..8 {
                            let tx = if placement.flip & 1 != 0 { 7 - dx } else { dx };
                            let ty = if placement.flip & 2 != 0 { 7 - dy } else { dy };
                            let color =
                                encode_bgr555(palette.colors[usize::from(source.pixels[ty][tx])]);
                            let index = if let Some(&index) = color_ids.get(&color) {
                                index
                            } else {
                                ensure!(
                                    color_ids.len() < 128,
                                    "world {world} map needs more than 128 background colors"
                                );
                                let index = color_ids.len() as u8;
                                color_ids.insert(color, index);
                                colors[usize::from(index) * 2..usize::from(index) * 2 + 2]
                                    .copy_from_slice(&color.to_le_bytes());
                                index
                            };
                            tile[dy * 8 + dx] = index;
                        }
                    }
                }
                let id = if let Some(&id) = tile_ids.get(&tile) {
                    id
                } else {
                    if next_id == 0xef {
                        next_id += 1;
                    }
                    ensure!(
                        next_id < 256,
                        "world {world} map needs more than 256 Mode 7 characters"
                    );
                    let id = next_id as u8;
                    tile_ids.insert(tile, id);
                    characters[next_id * 64..next_id * 64 + 64].copy_from_slice(&tile);
                    next_id += 1;
                    id
                };
                tilemap[(y / 32 * 2 + x / 32) * 1024 + y % 32 * 32 + x % 32] = id;
            }
        }
        eprintln!(
            "{} map: {}/256 Mode 7 tiles used (including reserved blank tile)",
            ["Light World", "Dark World"][world],
            tile_ids.len(),
        );
        result.graphics.extend(characters);
        result.tilemaps.extend(tilemap);
        result.palettes.extend(colors);
    }
    Ok(result)
}
