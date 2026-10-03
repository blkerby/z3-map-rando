use anyhow::{Context, Result};
use clap::Parser;
use patcher::graphics::{TileFingerprintIndex, canonicalize_tile};
use retiling_catalog::{
    AnimatedTileGroup, Area, AreaTheme, Background, BackgroundLayering, BackgroundSettings, Color,
    Cutscene, CutsceneAction, CutsceneEvent, DynamicTileGroup, DynamicTileType, DynamicTileVariant,
    Flip, Layer, Palette, PaletteId, RetilingCatalog, Tile, TileGraphic, TileGrid, TileId,
    TilePlacement, VanillaTileReference, encode_catalog,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, btree_map::Entry},
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

type Pixels = [[u8; 8]; 8];

#[derive(Parser)]
#[command(about = "Build a ROM-free retiling catalog from an ALTTPRetiling project")]
struct Args {
    retiling_project: PathBuf,
    output_catalog: PathBuf,
    #[arg(long, default_value = "data/tile_fingerprints.json")]
    tile_fingerprints: PathBuf,
}

#[derive(Deserialize)]
struct SourcePalette {
    id: PaletteId,
    colors: [Color; 16],
    tiles: Vec<SourceTile>,
    #[serde(default)]
    animated_tile_groups: Vec<SourceAnimation>,
}

#[derive(Deserialize)]
struct SourceTile {
    priority: bool,
    collision: u8,
    h_flippable: bool,
    v_flippable: bool,
    pixels: Pixels,
}

#[derive(Deserialize)]
struct SourceAnimation {
    base_tile: TileId,
    frames: Vec<[Pixels; 16]>,
    frame_hold: u16,
    phase_offset: u16,
}

#[derive(Deserialize)]
struct SourceArea {
    vanilla_map_id: Option<u8>,
    bg_color: Color,
    #[serde(default)]
    bg_layering: BackgroundLayering,
    #[serde(default = "get_default_camera_follow")]
    bg_camera_follow_x: f32,
    #[serde(default)]
    bg_camera_drift_x: f32,
    #[serde(default = "get_default_camera_follow")]
    bg_camera_follow_y: f32,
    #[serde(default)]
    bg_camera_drift_y: f32,
    size: [u8; 2],
    layers: Vec<SourceLayer>,
}

fn get_default_camera_follow() -> f32 {
    1.0
}

#[derive(Deserialize)]
struct SourceLayer {
    name: String,
    background: Background,
    screens: Vec<SourceScreen>,
}

#[derive(Deserialize)]
struct SourceScreen {
    position: [u16; 2],
    size: [u16; 2],
    palettes: Vec<Vec<Option<PaletteId>>>,
    tiles: Vec<Vec<Option<TileId>>>,
    flips: Vec<Vec<Option<u8>>>,
}

#[derive(Deserialize)]
struct SourceCutscenes {
    cutscenes: Vec<SourceCutscene>,
}

#[derive(Deserialize)]
struct SourceCutscene {
    event: CutsceneEvent,
    actions: Vec<CutsceneAction>,
}

#[derive(Deserialize)]
struct SourceDynamicTiles {
    groups: Vec<SourceDynamicGroup>,
}

#[derive(Deserialize)]
struct SourceDynamicGroup {
    #[serde(rename = "type")]
    kind: DynamicTileType,
    variants: Vec<SourceDynamicVariant>,
}

#[derive(Deserialize)]
struct SourceDynamicVariant {
    before: SourceDynamicGrid,
    after_frames: Vec<SourceDynamicGrid>,
}

#[derive(Deserialize)]
struct SourceDynamicGrid {
    tiles: Vec<Vec<Option<SourcePlacement>>>,
}

#[derive(Deserialize)]
struct SourcePlacement {
    palette: PaletteId,
    tile: TileId,
    flip: u8,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let fingerprint_bytes = fs::read(&args.tile_fingerprints)
        .with_context(|| format!("failed to read {}", args.tile_fingerprints.display()))?;
    let index: TileFingerprintIndex = serde_json::from_slice(&fingerprint_bytes)
        .with_context(|| format!("failed to parse {}", args.tile_fingerprints.display()))?;
    let mut fingerprints = BTreeMap::new();
    for sheet in index.sheets {
        for (tile, fingerprint) in sheet.tile_fingerprints.into_iter().enumerate() {
            let location = (sheet.sheet, tile as u8);
            match fingerprints.entry(fingerprint) {
                Entry::Vacant(entry) => {
                    entry.insert(location);
                }
                Entry::Occupied(mut entry) => {
                    if location < *entry.get() {
                        entry.insert(location);
                    }
                }
            }
        }
    }
    let mut catalog = RetilingCatalog {
        palettes: BTreeMap::new(),
        areas: BTreeMap::new(),
        dynamic_tile_groups: Vec::new(),
    };

    for path in find_json_paths(&args.retiling_project.join("Palettes"))? {
        let source: SourcePalette = read_json(&path)?;
        let mut palette = Palette {
            name: path.file_stem().unwrap().to_str().unwrap().to_owned(),
            colors: source.colors,
            tiles: Vec::new(),
            animated_tile_groups: Vec::new(),
        };
        for source_tile in source.tiles {
            palette.tiles.push(Tile {
                graphic: sanitize_graphic(source_tile.pixels, &fingerprints),
                priority: source_tile.priority,
                collision: source_tile.collision,
                h_flippable: source_tile.h_flippable,
                v_flippable: source_tile.v_flippable,
            });
        }
        for group in source.animated_tile_groups {
            let mut frames = Vec::new();
            for frame in group.frames {
                frames.push(frame.map(|pixels| sanitize_graphic(pixels, &fingerprints)));
            }
            palette.animated_tile_groups.push(AnimatedTileGroup {
                base_tile: group.base_tile,
                frames,
                frame_hold: group.frame_hold,
                phase_offset: group.phase_offset,
            });
        }
        catalog.palettes.insert(source.id, palette);
    }

    let mut area_paths = Vec::new();
    for entry in fs::read_dir(args.retiling_project.join("Areas"))? {
        let path = entry?.path();
        if path.is_dir() {
            area_paths.push(path);
        }
    }
    area_paths.sort();
    for directory in area_paths {
        let name = directory.file_name().unwrap().to_str().unwrap().to_owned();
        for path in find_json_paths(&directory)? {
            let source: SourceArea = read_json(&path)?;
            let theme = path.file_stem().unwrap().to_str().unwrap().to_owned();
            let mut layers = Vec::new();
            for layer in source.layers {
                let mut tiles = BTreeMap::new();
                for screen in layer.screens {
                    for y in 0..usize::from(screen.size[1]) {
                        for x in 0..usize::from(screen.size[0]) {
                            if let (Some(palette), Some(tile), Some(flip)) = (
                                screen.palettes[y][x],
                                screen.tiles[y][x],
                                screen.flips[y][x],
                            ) {
                                let placement = TilePlacement {
                                    x: (screen.position[0] + x as u16) as u8,
                                    y: (screen.position[1] + y as u16) as u8,
                                    palette,
                                    tile,
                                    flip: convert_flip(flip),
                                };
                                tiles.insert((placement.y, placement.x), placement);
                            }
                        }
                    }
                }
                layers.push(Layer {
                    name: layer.name,
                    background: layer.background,
                    grid: TileGrid {
                        width: source.size[0] * 32,
                        height: source.size[1] * 32,
                        tiles: tiles.into_values().collect(),
                    },
                });
            }
            let cutscene_path = directory.join(&theme).join("cutscenes.json");
            let mut cutscenes = Vec::new();
            if cutscene_path.is_file() {
                let scripts: SourceCutscenes = read_json(&cutscene_path)?;
                for script in scripts.cutscenes {
                    cutscenes.push(Cutscene {
                        event: script.event,
                        actions: script.actions,
                    });
                }
            }
            let area = catalog.areas.entry(name.clone()).or_insert_with(|| Area {
                vanilla_map_id: source.vanilla_map_id,
                themes: BTreeMap::new(),
            });
            area.themes.insert(
                theme,
                AreaTheme {
                    background: BackgroundSettings {
                        color: source.bg_color,
                        layering: source.bg_layering,
                        camera_follow: [source.bg_camera_follow_x, source.bg_camera_follow_y],
                        camera_drift: [source.bg_camera_drift_x, source.bg_camera_drift_y],
                    },
                    layers,
                    cutscenes,
                },
            );
        }
    }

    let dynamic: SourceDynamicTiles =
        read_json(&args.retiling_project.join("DynamicTiles/replacements.json"))?;
    for group in dynamic.groups {
        let mut variants = Vec::new();
        for variant in group.variants {
            let mut after_frames = Vec::new();
            for frame in variant.after_frames {
                after_frames.push(convert_dynamic_grid(frame));
            }
            variants.push(DynamicTileVariant {
                before: convert_dynamic_grid(variant.before),
                after_frames,
            });
        }
        catalog.dynamic_tile_groups.push(DynamicTileGroup {
            kind: group.kind,
            variants,
        });
    }

    let bytes = encode_catalog(&catalog)?;
    fs::write(&args.output_catalog, &bytes)
        .with_context(|| format!("failed to write {}", args.output_catalog.display()))?;
    let mut vanilla_graphics = 0;
    let mut custom_graphics = 0;
    for palette in catalog.palettes.values() {
        for tile in &palette.tiles {
            match tile.graphic {
                TileGraphic::Vanilla(_) => vanilla_graphics += 1,
                TileGraphic::Custom { .. } => custom_graphics += 1,
            }
        }
        for group in &palette.animated_tile_groups {
            for frame in &group.frames {
                for graphic in frame {
                    match graphic {
                        TileGraphic::Vanilla(_) => vanilla_graphics += 1,
                        TileGraphic::Custom { .. } => custom_graphics += 1,
                    }
                }
            }
        }
    }
    let mut theme_count = 0;
    for area in catalog.areas.values() {
        theme_count += area.themes.len();
    }
    eprintln!(
        "{}: {} palettes, {} areas, {theme_count} area themes, {vanilla_graphics} vanilla references, {custom_graphics} custom graphics, {} bytes",
        args.output_catalog.display(),
        catalog.palettes.len(),
        catalog.areas.len(),
        bytes.len(),
    );
    Ok(())
}

fn sanitize_graphic(pixels: Pixels, fingerprints: &BTreeMap<String, (u8, u8)>) -> TileGraphic {
    let canonical = canonicalize_tile(&pixels);
    let mut fingerprint = String::with_capacity(64);
    for byte in Sha256::digest(canonical.pixels) {
        write!(fingerprint, "{byte:02x}").unwrap();
    }
    if let Some(&(sheet, tile)) = fingerprints.get(&fingerprint) {
        TileGraphic::Vanilla(VanillaTileReference {
            sheet,
            tile,
            color_indexes: canonical.color_indexes,
            flip: convert_flip(canonical.flip),
        })
    } else {
        TileGraphic::Custom { pixels }
    }
}

fn convert_flip(flip: u8) -> Flip {
    [Flip::None, Flip::Horizontal, Flip::Vertical, Flip::Both][usize::from(flip)]
}

fn convert_dynamic_grid(source: SourceDynamicGrid) -> TileGrid {
    let height = source.tiles.len() as u8;
    let width = source.tiles.first().map_or(0, |row| row.len()) as u8;
    let mut tiles = Vec::new();
    for (y, row) in source.tiles.into_iter().enumerate() {
        for (x, placement) in row.into_iter().enumerate() {
            if let Some(placement) = placement {
                tiles.push(TilePlacement {
                    x: x as u8,
                    y: y as u8,
                    palette: placement.palette,
                    tile: placement.tile,
                    flip: convert_flip(placement.flip),
                });
            }
        }
    }
    TileGrid {
        width,
        height,
        tiles,
    }
}

fn find_json_paths(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?
    {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}
