use super::{BackgroundSettings, TileKey, graphics::Palette};
use crate::graphics::encode_bgr555;
use anyhow::{Context, Result, ensure};
use retiling_catalog::{Background, CutsceneAction, CutsceneEvent, PaletteId, RetilingCatalog};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Placement {
    pub(super) palette: PaletteId,
    pub(super) tile: usize,
    pub(super) flip: u8,
}

pub(super) struct ThemeArea {
    pub(super) id: usize,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) placements: Vec<Placement>,
    pub(super) palettes: BTreeSet<PaletteId>,
    pub(super) extra_tiles: BTreeSet<TileKey>,
    pub(super) overworld_overlay: Option<StateLayer>,
}

pub(super) struct Bg1Variant {
    pub(super) name: String,
    pub(super) area: usize,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) placements: Vec<Option<Placement>>,
}

#[derive(Clone)]
pub(super) struct StateLayer {
    pub(super) placements: Vec<Option<Placement>>,
}

pub(super) struct ThemeCutscene {
    pub(super) trigger: u8,
    pub(super) area: usize,
    pub(super) actions: Vec<CutsceneAction>,
    pub(super) layers: BTreeMap<String, StateLayer>,
}

impl Placement {
    pub(super) fn canonicalize_flip(&mut self, palettes: &BTreeMap<PaletteId, Palette>) {
        self.flip = palettes[&self.palette].tiles[self.tile].flips[usize::from(self.flip)];
    }
}

pub(super) struct SelectedAreas {
    pub(super) areas: Vec<ThemeArea>,
    pub(super) background_colors: [u16; 0xa0],
    pub(super) bg1_variants: Vec<Bg1Variant>,
    pub(super) background_settings: BTreeMap<usize, BackgroundSettings>,
    pub(super) cutscenes: Vec<ThemeCutscene>,
}

pub(super) fn select_areas(catalog: &RetilingCatalog, theme_name: &str) -> Result<SelectedAreas> {
    let mut result = Vec::new();
    let mut background_colors = [0x8000; 0xa0];
    let mut bg1_variants = Vec::new();
    let mut background_settings = BTreeMap::new();
    let mut cutscenes = Vec::new();
    let mut found_cutscenes = [false; 5];
    for (name, source) in &catalog.areas {
        let Some(area_id) = source.vanilla_map_id else {
            continue;
        };
        let Some(area) = source.themes.get(theme_name) else {
            continue;
        };
        let area_id = usize::from(area_id);
        let mut source_scripts = Vec::new();
        let mut cutscene_layer_names = BTreeSet::new();
        for source in &area.cutscenes {
            let (trigger, expected_area) = match source.event {
                CutsceneEvent::PalaceOfDarknessEntranceOpened => (1, 0x5e),
                CutsceneEvent::SkullWoodsEntranceOpened => (2, 0x40),
                CutsceneEvent::MiseryMireEntranceOpened => (3, 0x70),
                CutsceneEvent::TurtleRockEntranceOpened => (4, 0x47),
                CutsceneEvent::GanonsTowerEntranceOpened => (5, 0x43),
            };
            ensure!(
                area_id == expected_area,
                "cutscene event {:?} belongs to area ${expected_area:02X}, not ${:02X}",
                source.event,
                area_id,
            );
            ensure!(
                !found_cutscenes[trigger - 1],
                "duplicate cutscene event {:?}",
                source.event,
            );
            ensure!(
                matches!(source.actions.last(), Some(CutsceneAction::End)),
                "cutscene event {:?} must end with an end action",
                source.event,
            );
            found_cutscenes[trigger - 1] = true;
            for action in &source.actions {
                if let CutsceneAction::Draw { layer } = action {
                    cutscene_layer_names.insert(layer.clone());
                }
            }
            source_scripts.push((trigger as u8, source.actions.clone()));
        }
        let mut area_palettes = BTreeSet::new();
        let mut area_extra_tiles = BTreeSet::new();
        let width = usize::from(area.layers[0].grid.width);
        let height = usize::from(area.layers[0].grid.height);
        let mut area_tiles = vec![vec![None; width]; height];
        let mut bg1_layers = Vec::new();
        let mut cutscene_layers = BTreeMap::new();
        let mut overworld_overlay = None;
        for layer in &area.layers {
            let mut layer_tiles = vec![vec![None; width]; height];
            for placement in &layer.grid.tiles {
                layer_tiles[usize::from(placement.y)][usize::from(placement.x)] = Some(Placement {
                    palette: placement.palette,
                    tile: usize::from(placement.tile),
                    flip: placement.flip as u8,
                });
            }
            let is_overworld_overlay = matches!(
                (area_id, layer.name.as_str()),
                (0x02, "Lumberjack")
                    | (0x07, "Turtle Rock Portal")
                    | (0x18, "Bird Statue")
                    | (0x3b | 0x7b, "Drained")
                    | (0x58, "Thieves' Town")
                    | (0x5b, "Pyramid Hole")
                    | (0x62, "Hidden Stairs")
            ) && layer.background == Background::Bg2;
            if cutscene_layer_names.contains(&layer.name) || is_overworld_overlay {
                ensure!(
                    layer.background == Background::Bg2,
                    "cutscene layer {} must use BG2: {}",
                    layer.name,
                    name,
                );
                let mut placements = Vec::with_capacity(width * height);
                for row in &layer_tiles {
                    for &placement in row {
                        placements.push(placement);
                    }
                }
                for placement in placements.iter().flatten() {
                    area_palettes.insert(placement.palette);
                    area_extra_tiles.insert((placement.palette, placement.tile));
                }
                let state_layer = StateLayer { placements };
                if is_overworld_overlay {
                    overworld_overlay = Some(state_layer.clone());
                }
                if cutscene_layer_names.contains(&layer.name) {
                    cutscene_layers.insert(layer.name.clone(), state_layer);
                }
                continue;
            }
            match layer.background {
                Background::Bg1 => bg1_layers.push((layer.name.clone(), layer_tiles)),
                Background::Bg2 if layer.name == "Main" => {
                    for y in 0..height {
                        for x in 0..width {
                            if layer_tiles[y][x].is_some() {
                                area_tiles[y][x] = layer_tiles[y][x];
                            }
                        }
                    }
                }
                Background::Bg2 => {}
            }
        }
        for layer in &cutscene_layer_names {
            ensure!(
                cutscene_layers.contains_key(layer),
                "cutscene references missing layer {layer}: {}",
                name,
            );
        }
        for (trigger, actions) in source_scripts {
            cutscenes.push(ThemeCutscene {
                trigger,
                area: area_id,
                actions,
                layers: cutscene_layers.clone(),
            });
        }

        if area_id == 0x00 || area_id == 0x80 {
            for (name, tiles) in bg1_layers {
                let mut placements = Vec::with_capacity(width * height);
                for row in tiles {
                    placements.extend(row);
                }
                for placement in placements.iter().flatten() {
                    area_palettes.insert(placement.palette);
                    area_extra_tiles.insert((placement.palette, placement.tile));
                }
                bg1_variants.push(Bg1Variant {
                    name,
                    area: area_id,
                    width,
                    height,
                    placements,
                });
            }
        } else if !bg1_layers.is_empty() {
            let mut tiles = vec![vec![None; width]; height];
            for (_, layer_tiles) in bg1_layers {
                for y in 0..height {
                    for x in 0..width {
                        if layer_tiles[y][x].is_some() {
                            tiles[y][x] = layer_tiles[y][x];
                        }
                    }
                }
            }
            let mut placements = Vec::with_capacity(width * height);
            for row in tiles {
                placements.extend(row);
            }
            for placement in placements.iter().flatten() {
                area_palettes.insert(placement.palette);
                area_extra_tiles.insert((placement.palette, placement.tile));
            }
            bg1_variants.push(Bg1Variant {
                name: "BG1".to_string(),
                area: area_id,
                width,
                height,
                placements,
            });
        }

        for map_y in 0..height / 64 {
            for map_x in 0..width / 64 {
                let id = area_id + map_x + map_y * 8;
                background_colors[id] = encode_bgr555(area.background.color);
                background_settings.insert(
                    id,
                    BackgroundSettings {
                        layering: area.background.layering,
                        camera_follow_x: area.background.camera_follow[0],
                        camera_drift_x: area.background.camera_drift[0],
                        camera_follow_y: area.background.camera_follow[1],
                        camera_drift_y: area.background.camera_drift[1],
                    },
                );
            }
        }
        let mut placements = Vec::with_capacity(width * height);
        for (y, row) in area_tiles.into_iter().enumerate() {
            for (x, placement) in row.into_iter().enumerate() {
                let placement = placement
                    .with_context(|| format!("transparent BG2 tile at ({x}, {y}): {}", name))?;
                area_palettes.insert(placement.palette);
                placements.push(placement);
            }
        }
        result.push(ThemeArea {
            id: area_id,
            width,
            height,
            placements,
            palettes: area_palettes,
            extra_tiles: area_extra_tiles,
            overworld_overlay,
        });
    }
    for found in found_cutscenes {
        ensure!(
            found,
            "theme {theme_name} must define all five dungeon entrance cutscenes",
        );
    }
    result.sort_by_key(|area| area.id);
    Ok(SelectedAreas {
        areas: result,
        background_colors,
        bg1_variants,
        background_settings,
        cutscenes,
    })
}
