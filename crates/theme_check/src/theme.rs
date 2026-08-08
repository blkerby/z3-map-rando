use anyhow::{Context, Result, ensure};
use engine_check::asset_bundle::{DYNAMIC_TILE_GROUP_COUNT, DynamicTileEntry};
use patcher::import::{
    OverworldAnimationTrack, OverworldAreaAssets, OverworldBackgroundSettings, Tile8, Tile16,
};
use serde::Deserialize;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const MAP16_CAPACITY: usize = 0x4000;
const CHARACTER_CAPACITY: usize = 960;
const TRANSPARENT_CHARACTER: u16 = 0x03bf;
type TileKey = (u8, usize);
type CharacterSlots = BTreeMap<TileKey, usize>;
type ScreenTiles = Vec<BTreeSet<TileKey>>;
type Pixels = Vec<Vec<u8>>;

struct GraphicGroup {
    tiles: Vec<TileKey>,
    areas: BTreeSet<usize>,
}

#[derive(Deserialize)]
struct Palette {
    id: u8,
    colors: Vec<[u8; 3]>,
    tiles: Vec<Tile>,
    animated_tile_groups: Vec<AnimatedTileGroup>,
    #[serde(skip)]
    uses_upper_half: bool,
}

#[derive(Deserialize)]
struct AnimatedTileGroup {
    base_tile: usize,
    frames: Vec<Vec<Pixels>>,
    frame_hold: u8,
    phase_offset: usize,
}

#[derive(Deserialize)]
struct Tile {
    priority: bool,
    collision: u8,
    pixels: Pixels,
}

#[derive(Deserialize)]
struct Area {
    vanilla_map_id: usize,
    bg_color: [u8; 3],
    bg_layering: BackgroundLayering,
    bg_camera_follow_x: f32,
    bg_camera_drift_x: f32,
    bg_camera_follow_y: f32,
    bg_camera_drift_y: f32,
    size: [usize; 2],
    layers: Vec<SourceLayer>,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Background {
    Bg1,
    Bg2,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundLayering {
    None,
    HalfAdd,
    Backdrop,
}

#[derive(Deserialize)]
struct SourceLayer {
    name: String,
    background: Background,
    screens: Vec<SourceScreen>,
}

#[derive(Deserialize)]
struct SourceScreen {
    position: [usize; 2],
    size: [usize; 2],
    palettes: Vec<Vec<Option<u8>>>,
    tiles: Vec<Vec<Option<usize>>>,
    flips: Vec<Vec<Option<u8>>>,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Placement {
    palette: u8,
    tile: usize,
    flip: u8,
}

struct ThemeScreen {
    id: usize,
    asset_group: usize,
    placements: Vec<Placement>,
    palettes: BTreeSet<u8>,
    extra_tiles: BTreeSet<TileKey>,
}

struct Bg1Variant {
    name: String,
    area: usize,
    width: usize,
    height: usize,
    placements: Vec<Option<Placement>>,
}

pub struct CompiledBg1Variant {
    pub name: String,
    pub maps: BTreeMap<usize, Vec<u8>>,
}

pub struct BackgroundSettings {
    pub layering: BackgroundLayering,
    pub camera_follow_x: f32,
    pub camera_drift_x: f32,
    pub camera_follow_y: f32,
    pub camera_drift_y: f32,
}

#[derive(Deserialize)]
struct DynamicTiles {
    groups: Vec<DynamicTileGroup>,
}

#[derive(Deserialize)]
struct DynamicTileGroup {
    #[serde(rename = "type")]
    kind: DynamicTileType,
    variants: Vec<DynamicTileVariant>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DynamicTileType {
    CutGrass,
    DigTerrain,
    GreenBush,
    HeavyBush,
    HammerPeg,
    LiftSign,
    SmallGrayRock,
    SmallBlackRock,
    LargeGrayRock,
    LargeBlackRock,
    RockPile,
    SecretHole,
    SecretPortal,
    SecretBombableEntrance,
    SecretStairs,
    WoodenDoor,
    SanctuaryDoor,
    HyruleCastleDoor,
    GraveCorpse,
    GraveStairs,
    GravePit,
}

impl DynamicTileType {
    fn get_index(self) -> usize {
        self as usize
    }

    fn is_single_cell(self) -> bool {
        matches!(
            self,
            Self::CutGrass
                | Self::DigTerrain
                | Self::GreenBush
                | Self::HeavyBush
                | Self::HammerPeg
                | Self::LiftSign
                | Self::SmallGrayRock
                | Self::SmallBlackRock
                | Self::SecretHole
                | Self::SecretPortal
        )
    }

    fn is_anchor_property(self, property: u8) -> bool {
        match self {
            Self::LargeGrayRock => property == 0x55,
            Self::LargeBlackRock => property == 0x56,
            Self::RockPile => property == 0x57,
            Self::SecretStairs => property == 0x55 || property == 0x57,
            _ => false,
        }
    }
}

#[derive(Deserialize)]
struct DynamicTileVariant {
    before: DynamicTiling,
    after_frames: Vec<DynamicTiling>,
    #[serde(skip)]
    used: bool,
}

#[derive(Deserialize)]
struct DynamicTiling {
    tiles: Vec<Vec<Placement>>,
}

pub struct CompiledTheme {
    pub screen_maps: BTreeMap<usize, Vec<u8>>,
    pub bg1_variants: BTreeMap<usize, Vec<CompiledBg1Variant>>,
    pub background_settings: BTreeMap<usize, BackgroundSettings>,
    pub map16_definitions: [Vec<u8>; 4],
    pub map16_properties: [Vec<u8>; 4],
    pub background_colors: [u16; 0xa0],
    pub area_assets: Vec<OverworldAreaAssets>,
    pub dynamic_tile_groups: Vec<Vec<DynamicTileEntry>>,
    pub screen_count: usize,
    pub palette_count: usize,
    pub character_count: usize,
    pub map16_count: usize,
    pub fullest_palette_screen: usize,
    pub fullest_palette_half_slots: usize,
}

pub fn compile(
    root: &Path,
    vanilla_tiles: &[Tile16],
    vanilla_tile_types: &[u8],
    mut area_assets: Vec<OverworldAreaAssets>,
) -> Result<CompiledTheme> {
    let palettes = load_palettes(root)?;
    let mut dynamic_tiles: DynamicTiles = read_json(&root.join("DynamicTiles/replacements.json"))?;
    let (mut screens, background_colors, mut bg1_variants, background_settings) =
        load_screens(root)?;

    let mut canonical_flips = BTreeMap::new();
    for (&palette_id, palette) in &palettes {
        for (tile_id, tile) in palette.tiles.iter().enumerate() {
            let mut flips = [0, 1, 2, 3];
            if !(0x10..0x1c).contains(&tile.collision) {
                for flip in 1..4 {
                    for candidate in 0..flip {
                        let mut identical = true;
                        for y in 0..8 {
                            for x in 0..8 {
                                let flip_x = if flip & 1 != 0 { 7 - x } else { x };
                                let flip_y = if flip & 2 != 0 { 7 - y } else { y };
                                let candidate_x = if candidate & 1 != 0 { 7 - x } else { x };
                                let candidate_y = if candidate & 2 != 0 { 7 - y } else { y };
                                if tile.pixels[flip_y][flip_x]
                                    != tile.pixels[candidate_y][candidate_x]
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
            canonical_flips.insert((palette_id, tile_id), flips);
        }
    }
    for screen in &mut screens {
        for placement in &mut screen.placements {
            placement.flip =
                canonical_flips[&(placement.palette, placement.tile)][usize::from(placement.flip)];
        }
    }
    for variant in &mut bg1_variants {
        for placement in variant.placements.iter_mut().flatten() {
            placement.flip =
                canonical_flips[&(placement.palette, placement.tile)][usize::from(placement.flip)];
        }
    }
    for group in &mut dynamic_tiles.groups {
        for variant in &mut group.variants {
            for row in &mut variant.before.tiles {
                for placement in row {
                    placement.flip = canonical_flips[&(placement.palette, placement.tile)]
                        [usize::from(placement.flip)];
                }
            }
            for frame in &mut variant.after_frames {
                for row in &mut frame.tiles {
                    for placement in row {
                        placement.flip = canonical_flips[&(placement.palette, placement.tile)]
                            [usize::from(placement.flip)];
                    }
                }
            }
        }
    }
    add_dynamic_dependencies(&mut screens, &mut dynamic_tiles);
    ensure!(area_assets.len() == 0xa0);

    let palette_slots = allocate_palettes(&screens, &palettes)?;
    let mut fullest_palette = (0, 0);
    for screen in &screens {
        let mut occupied = [false; 12];
        for palette in &screen.palettes {
            let slot = palette_slots[palette];
            occupied[slot] = true;
            if palettes[palette].uses_upper_half {
                occupied[slot + 1] = true;
            }
        }
        let count = occupied.into_iter().filter(|occupied| *occupied).count();
        if count > fullest_palette.1 {
            fullest_palette = (screen.id, count);
        }
    }
    let (character_slots, screen_tiles) = allocate_characters(&screens, &palettes)?;
    for variant in &bg1_variants {
        let screen_index = screens
            .iter()
            .position(|screen| screen.asset_group == variant.area)
            .with_context(|| format!("BG1 area {:02X} has no BG2 asset record", variant.area))?;
        for placement in variant.placements.iter().flatten() {
            ensure!(
                screens[screen_index].palettes.contains(&placement.palette)
                    && screen_tiles[screen_index].contains(&(placement.palette, placement.tile)),
                "BG1 variant {} references an asset absent from area {:02X}",
                variant.name,
                variant.area
            );
        }
    }
    let character_count = character_slots
        .values()
        .copied()
        .max()
        .map_or(0, |slot| slot + 1);
    ensure!(
        character_count <= CHARACTER_CAPACITY,
        "Desert needs {character_count} stable character slots, but the existing row ABI exposes {CHARACTER_CAPACITY}"
    );

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
    let mut definition_ids = BTreeMap::new();

    let mut screen_maps = BTreeMap::new();
    for (screen, tiles) in screens.iter().zip(&screen_tiles) {
        let palette_rows = build_palette_rows(screen, &palettes, &palette_slots)?;
        let character_rows =
            build_character_rows(tiles, &palettes, &palette_slots, &character_slots)?;
        let animation_tracks =
            build_animation_tracks(tiles, &palettes, &palette_slots, &character_slots);
        let map = build_map(
            screen,
            &palettes,
            &palette_slots,
            &character_slots,
            &mut definitions,
            &mut properties,
            &mut definition_ids,
        )?;
        screen_maps.insert(screen.id, map);

        let sprites = area_assets[screen.id].sprite_variants.clone();
        let entrances = area_assets[screen.id].entrances.clone();
        let pit_entrances = area_assets[screen.id].pit_entrances.clone();
        let mut transition_palette_rows = vec![false; 6];
        for palette in &screen.palettes {
            transition_palette_rows[palette_slots[palette] / 2] = true;
        }
        let mut transition_character_rows = vec![false; 60];
        for tile in tiles {
            transition_character_rows[character_slots[tile] / 16] = true;
        }
        area_assets[screen.id] = OverworldAreaAssets {
            palette_rows: palette_rows.to_vec(),
            character_rows: character_rows.to_vec(),
            transition_palette_rows,
            transition_character_rows,
            animation_tracks,
            sprite_variants: sprites,
            entrances,
            pit_entrances,
            background: OverworldBackgroundSettings {
                layering: match background_settings[&screen.id].layering {
                    BackgroundLayering::None => 0,
                    BackgroundLayering::HalfAdd => 1,
                    BackgroundLayering::Backdrop => 2,
                },
                camera_follow_x: encode_eighths(background_settings[&screen.id].camera_follow_x)?,
                camera_drift_x: encode_eighths(background_settings[&screen.id].camera_drift_x)?,
                camera_follow_y: encode_eighths(background_settings[&screen.id].camera_follow_y)?,
                camera_drift_y: encode_eighths(background_settings[&screen.id].camera_drift_y)?,
            },
        };
    }

    let dynamic_tile_groups = build_dynamic_tile_groups(
        &dynamic_tiles,
        &palettes,
        &palette_slots,
        &character_slots,
        &mut definitions,
        &mut properties,
        &mut definition_ids,
    )?;

    let mut bg1_definition_ids = BTreeMap::new();
    for (id, &words) in definitions.iter().enumerate() {
        bg1_definition_ids
            .entry(words)
            .or_insert(u16::try_from(id)?);
    }
    let mut compiled_bg1_variants = BTreeMap::<usize, Vec<CompiledBg1Variant>>::new();
    for variant in bg1_variants {
        let maps = build_bg1_maps(
            &variant.placements,
            variant.area,
            variant.width,
            variant.height,
            &palettes,
            &palette_slots,
            &character_slots,
            &mut definitions,
            &mut bg1_definition_ids,
        )?;
        compiled_bg1_variants
            .entry(variant.area)
            .or_default()
            .push(CompiledBg1Variant {
                name: variant.name,
                maps,
            });
    }

    ensure!(definitions.len() <= MAP16_CAPACITY);

    let mut map16_definitions = std::array::from_fn(|_| Vec::new());
    for tile in &definitions {
        for quadrant in 0..4 {
            map16_definitions[quadrant].extend_from_slice(&tile[quadrant].to_le_bytes());
        }
    }
    Ok(CompiledTheme {
        screen_count: screen_maps.len(),
        palette_count: palette_slots.len(),
        character_count,
        map16_count: definitions.len(),
        fullest_palette_screen: fullest_palette.0,
        fullest_palette_half_slots: fullest_palette.1,
        screen_maps,
        bg1_variants: compiled_bg1_variants,
        background_settings,
        map16_definitions,
        map16_properties: properties,
        background_colors,
        area_assets,
        dynamic_tile_groups,
    })
}

fn load_palettes(root: &Path) -> Result<BTreeMap<u8, Palette>> {
    let mut paths = find_json_paths(&root.join("Palettes"))?;
    paths.sort();
    let mut palettes = BTreeMap::new();
    for path in paths {
        let mut palette: Palette = read_json(&path)?;
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
        palettes.insert(palette.id, palette);
    }
    Ok(palettes)
}

fn load_screens(
    root: &Path,
) -> Result<(
    Vec<ThemeScreen>,
    [u16; 0xa0],
    Vec<Bg1Variant>,
    BTreeMap<usize, BackgroundSettings>,
)> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(root.join("Areas"))? {
        let Ok(entry) = entry else {
            continue;
        };
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let path = entry.path().join("Desert.json");
        if is_numbered_area(name) && path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();

    let mut result = Vec::new();
    let mut background_colors = [0x8000; 0xa0];
    let mut bg1_variants = Vec::new();
    let mut background_settings = BTreeMap::new();
    for path in paths {
        let area: Area = read_json(&path)?;
        let first_screen = result.len();
        let mut area_palettes = BTreeSet::new();
        let mut area_extra_tiles = BTreeSet::new();
        let width = area.size[0] * 32;
        let height = area.size[1] * 32;
        let mut area_tiles = vec![vec![None; width]; height];
        let mut bg1_layers = Vec::new();
        for layer in &area.layers {
            let mut layer_tiles = vec![vec![None; width]; height];
            for screen in &layer.screens {
                for y in 0..screen.size[1] {
                    for x in 0..screen.size[0] {
                        if let (Some(palette), Some(tile), Some(flip)) = (
                            screen.palettes[y][x],
                            screen.tiles[y][x],
                            screen.flips[y][x],
                        ) {
                            layer_tiles[screen.position[1] + y][screen.position[0] + x] =
                                Some(Placement {
                                    palette,
                                    tile,
                                    flip,
                                });
                        }
                    }
                }
            }
            match layer.background {
                Background::Bg1 => bg1_layers.push((layer.name.clone(), layer_tiles)),
                Background::Bg2 => {
                    for y in 0..height {
                        for x in 0..width {
                            if layer_tiles[y][x].is_some() {
                                area_tiles[y][x] = layer_tiles[y][x];
                            }
                        }
                    }
                }
            }
        }

        if area.vanilla_map_id == 0x80 {
            for (name, tiles) in bg1_layers {
                let placements: Vec<_> = tiles.into_iter().flatten().collect();
                for placement in placements.iter().flatten() {
                    area_palettes.insert(placement.palette);
                    area_extra_tiles.insert((placement.palette, placement.tile));
                }
                bg1_variants.push(Bg1Variant {
                    name,
                    area: area.vanilla_map_id,
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
            let placements: Vec<_> = tiles.into_iter().flatten().collect();
            for placement in placements.iter().flatten() {
                area_palettes.insert(placement.palette);
                area_extra_tiles.insert((placement.palette, placement.tile));
            }
            bg1_variants.push(Bg1Variant {
                name: "BG1".to_string(),
                area: area.vanilla_map_id,
                width,
                height,
                placements,
            });
        }

        for group_y in 0..area.size[1] / 2 {
            for group_x in 0..area.size[0] / 2 {
                let id = area.vanilla_map_id + group_x + group_y * 8;
                background_colors[id] = encode_bgr555(area.bg_color);
                background_settings.insert(
                    id,
                    BackgroundSettings {
                        layering: area.bg_layering,
                        camera_follow_x: area.bg_camera_follow_x,
                        camera_drift_x: area.bg_camera_drift_x,
                        camera_follow_y: area.bg_camera_follow_y,
                        camera_drift_y: area.bg_camera_drift_y,
                    },
                );
                let mut placements = Vec::with_capacity(64 * 64);
                for component_y in 0..2 {
                    for y in 0..32 {
                        for component_x in 0..2 {
                            for x in 0..32 {
                                let area_x = group_x * 64 + component_x * 32 + x;
                                let area_y = group_y * 64 + component_y * 32 + y;
                                let placement = area_tiles[area_y][area_x].with_context(|| {
                                    format!(
                                        "transparent BG2 tile at ({area_x}, {area_y}): {}",
                                        path.display()
                                    )
                                })?;
                                area_palettes.insert(placement.palette);
                                placements.push(placement);
                            }
                        }
                    }
                }
                result.push(ThemeScreen {
                    id,
                    asset_group: area.vanilla_map_id,
                    placements,
                    palettes: BTreeSet::new(),
                    extra_tiles: BTreeSet::new(),
                });
            }
        }
        for screen in &mut result[first_screen..] {
            screen.palettes = area_palettes.clone();
            screen.extra_tiles = area_extra_tiles.clone();
        }
    }
    result.sort_by_key(|screen| screen.id);
    Ok((result, background_colors, bg1_variants, background_settings))
}

fn add_dynamic_dependencies(screens: &mut [ThemeScreen], dynamic_tiles: &mut DynamicTiles) {
    let mut area_palettes = BTreeMap::<usize, BTreeSet<u8>>::new();
    let mut area_tiles = BTreeMap::<usize, BTreeSet<TileKey>>::new();
    for group in &mut dynamic_tiles.groups {
        for variant in &mut group.variants {
            let height = variant.before.tiles.len();
            let width = variant.before.tiles[0].len();
            let mut areas = BTreeSet::new();
            for screen in screens.iter() {
                for start_y in 0..=64 - height {
                    'position: for start_x in 0..=64 - width {
                        for y in 0..height {
                            for x in 0..width {
                                if screen.placements[(start_y + y) * 64 + start_x + x]
                                    != variant.before.tiles[y][x]
                                {
                                    continue 'position;
                                }
                            }
                        }
                        areas.insert(screen.asset_group);
                    }
                }
            }
            for frame in &variant.after_frames {
                for row in &frame.tiles {
                    for placement in row {
                        for &area in &areas {
                            area_palettes
                                .entry(area)
                                .or_default()
                                .insert(placement.palette);
                            area_tiles
                                .entry(area)
                                .or_default()
                                .insert((placement.palette, placement.tile));
                        }
                    }
                }
            }
            variant.used = !areas.is_empty();
        }
    }

    for screen in screens {
        if let Some(palettes) = area_palettes.get(&screen.asset_group) {
            screen.palettes.extend(palettes);
        }
        if let Some(tiles) = area_tiles.get(&screen.asset_group) {
            screen.extra_tiles.extend(tiles);
        }
    }
}

fn allocate_palettes(
    screens: &[ThemeScreen],
    palettes: &BTreeMap<u8, Palette>,
) -> Result<BTreeMap<u8, usize>> {
    let mut used = BTreeSet::new();
    for screen in screens {
        for &palette in &screen.palettes {
            used.insert(palette);
        }
    }
    let mut full = BTreeSet::new();
    let mut conflicts = BTreeMap::new();
    for &id in &used {
        if palettes[&id].uses_upper_half {
            full.insert(id);
        }
        conflicts.insert(id, BTreeSet::new());
    }

    // Palettes in the same area or a scrolling neighbor must use different slots.
    for left_screen in screens {
        for right_screen in screens {
            if areas_can_coexist(left_screen, right_screen) {
                for &left in &left_screen.palettes {
                    for &right in &right_screen.palettes {
                        if right != left {
                            conflicts.get_mut(&left).unwrap().insert(right);
                        }
                    }
                }
            }
        }
    }

    // Prioritize first assigning palettes that are full size (16 colors)
    // and have many conflicts.
    let mut order = used.into_iter().collect::<Vec<_>>();
    order.sort_by_key(|id| {
        (
            Reverse(full.contains(id)),
            Reverse(conflicts[id].len()),
            *id,
        )
    });
    let mut assignments = BTreeMap::new();

    // Recursively assign palettes with backtracking:
    ensure!(
        assign_palette(0, &order, &full, &conflicts, &mut assignments),
        "Desert palettes cannot fit in six BG rows"
    );
    Ok(assignments)
}

fn assign_palette(
    index: usize,
    order: &[u8],
    full: &BTreeSet<u8>,
    conflicts: &BTreeMap<u8, BTreeSet<u8>>,
    assignments: &mut BTreeMap<u8, usize>,
) -> bool {
    let Some(&id) = order.get(index) else {
        return true;
    };
    'candidate: for slot in 0..12 {
        if full.contains(&id) && slot % 2 != 0 {
            continue;
        }
        let occupied_end = if full.contains(&id) { slot + 1 } else { slot };
        let occupied_slots = slot..=occupied_end;
        for occupied_slot in occupied_slots {
            for other in &conflicts[&id] {
                let Some(&other_slot) = assignments.get(other) else {
                    continue;
                };
                let other_end = if full.contains(other) {
                    other_slot + 1
                } else {
                    other_slot
                };
                let other_slots = other_slot..=other_end;
                if other_slots.contains(&occupied_slot) {
                    continue 'candidate;
                }
            }
        }

        assignments.insert(id, slot);
        if assign_palette(index + 1, order, full, conflicts, assignments) {
            return true;
        }
        assignments.remove(&id);
    }
    false
}

fn allocate_characters(
    screens: &[ThemeScreen],
    palettes: &BTreeMap<u8, Palette>,
) -> Result<(CharacterSlots, ScreenTiles)> {
    let mut tile_areas = BTreeMap::<TileKey, BTreeSet<usize>>::new();
    for screen in screens {
        for placement in &screen.placements {
            tile_areas
                .entry((placement.palette, placement.tile))
                .or_default()
                .insert(screen.asset_group);
        }
        for &tile in &screen.extra_tiles {
            tile_areas
                .entry(tile)
                .or_default()
                .insert(screen.asset_group);
        }
    }

    let mut groups = Vec::new();
    let mut palette_tiles = BTreeMap::<u8, BTreeSet<usize>>::new();
    for &(palette, tile) in tile_areas.keys() {
        palette_tiles.entry(palette).or_default().insert(tile);
    }
    for (palette, mut unassigned) in palette_tiles {
        for animation in &palettes[&palette].animated_tile_groups {
            let mut tiles = Vec::with_capacity(16);
            let mut areas = BTreeSet::new();
            for tile in animation.base_tile..animation.base_tile + 16 {
                let key = (palette, tile);
                if let Some(tile_areas) = tile_areas.get(&key) {
                    areas.extend(tile_areas);
                }
                unassigned.remove(&tile);
                tiles.push(key);
            }
            if !areas.is_empty() {
                groups.push(GraphicGroup { tiles, areas });
            }
        }
        while !unassigned.is_empty() {
            let mut seed = *unassigned.first().unwrap();
            for &tile in &unassigned {
                if tile_areas[&(palette, tile)].len() > tile_areas[&(palette, seed)].len() {
                    seed = tile;
                }
            }
            unassigned.remove(&seed);
            let mut tiles = vec![(palette, seed)];
            while tiles.len() < 16 && !unassigned.is_empty() {
                let mut best = *unassigned.first().unwrap();
                let mut best_score = 0;
                for &candidate in &unassigned {
                    let mut score = 0;
                    for member in &tiles {
                        for area in &tile_areas[&(palette, candidate)] {
                            if tile_areas[member].contains(area) {
                                score += 1;
                            }
                        }
                    }
                    if score > best_score || score == best_score && candidate < best {
                        best = candidate;
                        best_score = score;
                    }
                }
                unassigned.remove(&best);
                tiles.push((palette, best));
            }
            let mut areas = BTreeSet::new();
            for tile in &tiles {
                areas.extend(&tile_areas[tile]);
            }
            groups.push(GraphicGroup { tiles, areas });
        }
    }

    let mut neighboring_areas = BTreeMap::<usize, BTreeSet<usize>>::new();
    for left in screens {
        for right in screens {
            if areas_can_coexist(left, right) {
                neighboring_areas
                    .entry(left.asset_group)
                    .or_default()
                    .insert(right.asset_group);
            }
        }
    }
    let mut conflicts = vec![BTreeSet::new(); groups.len()];
    for left in 0..groups.len() {
        for right in left + 1..groups.len() {
            let mut conflict = false;
            for area in &groups[left].areas {
                if neighboring_areas[area]
                    .iter()
                    .any(|neighbor| groups[right].areas.contains(neighbor))
                {
                    conflict = true;
                    break;
                }
            }
            if conflict {
                conflicts[left].insert(right);
                conflicts[right].insert(left);
            }
        }
    }

    let mut order = (0..groups.len()).collect::<Vec<_>>();
    order.sort_by_key(|&group| (Reverse(conflicts[group].len()), group));
    let mut group_rows = BTreeMap::new();
    for group in order {
        let mut blocked = BTreeSet::new();
        for other in &conflicts[group] {
            if let Some(&row) = group_rows.get(other) {
                blocked.insert(row);
            }
        }
        let row = (0..CHARACTER_CAPACITY / 16 - 1)
            .find(|row| !blocked.contains(row))
            .context("Graphics exceed the existing stable character rows")?;
        group_rows.insert(group, row);
    }

    let mut slots = BTreeMap::new();
    for (group, group_data) in groups.iter().enumerate() {
        for (column, &tile) in group_data.tiles.iter().enumerate() {
            slots.insert(tile, group_rows[&group] * 16 + column);
        }
    }

    let mut screen_tiles = Vec::with_capacity(screens.len());
    for screen in screens {
        let mut tiles = BTreeSet::new();
        for group in &groups {
            if group.areas.contains(&screen.asset_group) {
                tiles.extend(&group.tiles);
            }
        }
        screen_tiles.push(tiles);
    }
    Ok((slots, screen_tiles))
}

fn areas_can_coexist(left: &ThemeScreen, right: &ThemeScreen) -> bool {
    if left.asset_group == right.asset_group {
        return true;
    }
    if left.id >= 0x80 || right.id >= 0x80 || left.id / 0x40 != right.id / 0x40 {
        return false;
    }
    let left = left.id % 0x40;
    let right = right.id % 0x40;
    let horizontal = left / 8 == right / 8 && left.abs_diff(right) == 1;
    let vertical = left % 8 == right % 8 && left.abs_diff(right) == 8;
    horizontal || vertical
}

fn build_palette_rows(
    screen: &ThemeScreen,
    palettes: &BTreeMap<u8, Palette>,
    slots: &BTreeMap<u8, usize>,
) -> Result<[[u8; 32]; 6]> {
    let mut rows = [[0; 32]; 6];
    for id in &screen.palettes {
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
    Ok(rows)
}

fn build_character_rows(
    tiles: &BTreeSet<TileKey>,
    palettes: &BTreeMap<u8, Palette>,
    palette_slots: &BTreeMap<u8, usize>,
    slots: &CharacterSlots,
) -> Result<[[u8; 512]; 60]> {
    let mut rows = [[0; 512]; 60];
    for &(palette, tile) in tiles {
        let slot = slots[&(palette, tile)];
        ensure!(slot < CHARACTER_CAPACITY);
        let graphic = build_graphic(
            palette,
            &palettes[&palette].tiles[tile].pixels,
            palettes,
            palette_slots,
        );
        let encoded = encode_4bpp(&graphic);
        let offset = slot % 16 * 32;
        rows[slot / 16][offset..offset + 32].copy_from_slice(&encoded);
    }
    Ok(rows)
}

fn build_animation_tracks(
    tiles: &BTreeSet<TileKey>,
    palettes: &BTreeMap<u8, Palette>,
    palette_slots: &BTreeMap<u8, usize>,
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
                    let graphic = build_graphic(palette_id, pixels, palettes, palette_slots);
                    row[tile_index * 32..tile_index * 32 + 32]
                        .copy_from_slice(&encode_4bpp(&graphic));
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

fn build_dynamic_tile_groups(
    dynamic_tiles: &DynamicTiles,
    palettes: &BTreeMap<u8, Palette>,
    palette_slots: &BTreeMap<u8, usize>,
    character_slots: &CharacterSlots,
    definitions: &mut Vec<[u16; 4]>,
    properties: &mut [Vec<u8>; 4],
    definition_ids: &mut BTreeMap<[Placement; 4], u16>,
) -> Result<Vec<Vec<DynamicTileEntry>>> {
    let mut result = Vec::with_capacity(DYNAMIC_TILE_GROUP_COUNT);
    for _ in 0..DYNAMIC_TILE_GROUP_COUNT {
        result.push(Vec::new());
    }
    for group in &dynamic_tiles.groups {
        for variant in &group.variants {
            if !variant.used {
                continue;
            }
            let before = build_tiling(
                &variant.before,
                palettes,
                palette_slots,
                character_slots,
                definitions,
                properties,
                definition_ids,
            )?;
            let mut after_frames = Vec::with_capacity(variant.after_frames.len());
            for frame in &variant.after_frames {
                after_frames.push(build_tiling(
                    frame,
                    palettes,
                    palette_slots,
                    character_slots,
                    definitions,
                    properties,
                    definition_ids,
                )?);
            }
            let width = variant.before.tiles[0].len() / 2;
            let height = variant.before.tiles.len() / 2;
            if group.kind.is_single_cell()
                || matches!(
                    group.kind,
                    DynamicTileType::SecretBombableEntrance
                        | DynamicTileType::WoodenDoor
                        | DynamicTileType::SanctuaryDoor
                        | DynamicTileType::HyruleCastleDoor
                        | DynamicTileType::GraveCorpse
                        | DynamicTileType::GraveStairs
                        | DynamicTileType::GravePit
                )
            {
                result[group.kind.get_index()].push(DynamicTileEntry {
                    source: before[0],
                    x_offset: 0,
                    y_offset: 0,
                    width: u8::try_from(width)?,
                    height: u8::try_from(height)?,
                    before,
                    after_frames,
                });
                continue;
            }
            for (index, &source) in before.iter().enumerate() {
                let mut is_anchor = false;
                for quadrant in properties.iter() {
                    if group.kind.is_anchor_property(quadrant[usize::from(source)]) {
                        is_anchor = true;
                        break;
                    }
                }
                if !is_anchor {
                    continue;
                }
                result[group.kind.get_index()].push(DynamicTileEntry {
                    source,
                    x_offset: -i8::try_from(index % width)?,
                    y_offset: -i8::try_from(index / width)?,
                    width: u8::try_from(width)?,
                    height: u8::try_from(height)?,
                    before: before.clone(),
                    after_frames: after_frames.clone(),
                });
            }
        }
    }
    Ok(result)
}

fn build_tiling(
    tiling: &DynamicTiling,
    palettes: &BTreeMap<u8, Palette>,
    palette_slots: &BTreeMap<u8, usize>,
    character_slots: &CharacterSlots,
    definitions: &mut Vec<[u16; 4]>,
    properties: &mut [Vec<u8>; 4],
    definition_ids: &mut BTreeMap<[Placement; 4], u16>,
) -> Result<Vec<u16>> {
    let mut result = Vec::new();
    for y in (0..tiling.tiles.len()).step_by(2) {
        for x in (0..tiling.tiles[0].len()).step_by(2) {
            result.push(intern_map16(
                [
                    tiling.tiles[y][x],
                    tiling.tiles[y][x + 1],
                    tiling.tiles[y + 1][x],
                    tiling.tiles[y + 1][x + 1],
                ],
                palettes,
                palette_slots,
                character_slots,
                definitions,
                properties,
                definition_ids,
            )?);
        }
    }
    Ok(result)
}

fn intern_map16(
    placements: [Placement; 4],
    palettes: &BTreeMap<u8, Palette>,
    palette_slots: &BTreeMap<u8, usize>,
    character_slots: &CharacterSlots,
    definitions: &mut Vec<[u16; 4]>,
    properties: &mut [Vec<u8>; 4],
    definition_ids: &mut BTreeMap<[Placement; 4], u16>,
) -> Result<u16> {
    let mut words = [0; 4];
    let mut props = [0; 4];
    for (quadrant, placement) in placements.iter().copied().enumerate() {
        let palette = &palettes[&placement.palette];
        let tile = &palette.tiles[placement.tile];
        words[quadrant] = u16::try_from(character_slots[&(placement.palette, placement.tile)])?
            | u16::try_from(2 + palette_slots[&placement.palette] / 2)? << 10
            | if tile.priority { 1 << 13 } else { 0 }
            | u16::from(placement.flip) << 14;
        props[quadrant] = if (0x10..0x1c).contains(&tile.collision) {
            tile.collision ^ placement.flip
        } else {
            tile.collision
        };
    }
    if let Some(&id) = definition_ids.get(&placements) {
        return Ok(id);
    }

    ensure!(
        definitions.len() < MAP16_CAPACITY,
        "Desert Map16 definitions exceed {MAP16_CAPACITY}"
    );
    let id = u16::try_from(definitions.len())?;
    definitions.push(words);
    for quadrant in 0..4 {
        properties[quadrant][usize::from(id)] = props[quadrant];
    }
    definition_ids.insert(placements, id);
    Ok(id)
}

fn build_map(
    screen: &ThemeScreen,
    palettes: &BTreeMap<u8, Palette>,
    palette_slots: &BTreeMap<u8, usize>,
    character_slots: &CharacterSlots,
    definitions: &mut Vec<[u16; 4]>,
    properties: &mut [Vec<u8>; 4],
    definition_ids: &mut BTreeMap<[Placement; 4], u16>,
) -> Result<Vec<u8>> {
    let mut map = Vec::with_capacity(0x800);
    for map_y in 0..32 {
        for map_x in 0..32 {
            let x = map_x * 2;
            let y = map_y * 2;
            let id = intern_map16(
                [
                    screen.placements[y * 64 + x],
                    screen.placements[y * 64 + x + 1],
                    screen.placements[(y + 1) * 64 + x],
                    screen.placements[(y + 1) * 64 + x + 1],
                ],
                palettes,
                palette_slots,
                character_slots,
                definitions,
                properties,
                definition_ids,
            )?;
            map.extend_from_slice(&id.to_le_bytes());
        }
    }
    Ok(map)
}

fn build_bg1_maps(
    placements: &[Option<Placement>],
    area: usize,
    width: usize,
    height: usize,
    palettes: &BTreeMap<u8, Palette>,
    palette_slots: &BTreeMap<u8, usize>,
    character_slots: &CharacterSlots,
    definitions: &mut Vec<[u16; 4]>,
    definition_ids: &mut BTreeMap<[u16; 4], u16>,
) -> Result<BTreeMap<usize, Vec<u8>>> {
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
                        let tile = &palettes[&placement.palette].tiles[placement.tile];
                        words[index] =
                            u16::try_from(character_slots[&(placement.palette, placement.tile)])?
                                | u16::try_from(2 + palette_slots[&placement.palette] / 2)? << 10
                                | if tile.priority { 1 << 13 } else { 0 }
                                | u16::from(placement.flip) << 14;
                    }
                    let id = if let Some(&id) = definition_ids.get(&words) {
                        id
                    } else {
                        let id = u16::try_from(definitions.len())?;
                        definitions.push(words);
                        definition_ids.insert(words, id);
                        id
                    };
                    output.extend_from_slice(&id.to_le_bytes());
                }
            }
            maps.insert(area + area_x + area_y * 8, output);
        }
    }
    Ok(maps)
}

fn build_graphic(
    palette_id: u8,
    pixels: &Pixels,
    palettes: &BTreeMap<u8, Palette>,
    slots: &BTreeMap<u8, usize>,
) -> Vec<u8> {
    let palette = &palettes[&palette_id];
    let offset = if palette.uses_upper_half {
        0
    } else if slots[&palette_id] % 2 == 1 {
        8
    } else {
        0
    };
    let mut graphic = Vec::with_capacity(64);
    for row in pixels {
        for &pixel in row {
            graphic.push(if pixel == 0 { 0 } else { pixel + offset });
        }
    }
    graphic
}

fn encode_eighths(value: f32) -> Result<i8> {
    let scaled = value * 8.0;
    ensure!(
        scaled.fract() == 0.0 && scaled >= f32::from(i8::MIN) && scaled <= f32::from(i8::MAX),
        "background camera value {value} is not representable in signed eighths"
    );
    Ok(scaled as i8)
}

fn encode_bgr555([red, green, blue]: [u8; 3]) -> u16 {
    u16::from(red) | u16::from(green) << 5 | u16::from(blue) << 10
}

fn encode_4bpp(pixels: &[u8]) -> [u8; 32] {
    let mut output = [0; 32];
    for y in 0..8 {
        for x in 0..8 {
            let pixel = pixels[y * 8 + x];
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

fn find_json_paths(directory: &Path) -> Result<Vec<PathBuf>> {
    let entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    serde_json::from_slice(
        &fs::read(path).with_context(|| format!("failed to read {}", path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", path.display()))
}

fn is_numbered_area(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() > 2
        && bytes[0].is_ascii_hexdigit()
        && bytes[1].is_ascii_hexdigit()
        && bytes[2] == b' '
}
