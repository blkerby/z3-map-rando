use anyhow::{Context, Result, ensure};
use patcher::import::{OverworldAreaAssets, Tile8, Tile16};
use serde::Deserialize;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const MAP16_CAPACITY: usize = 0x4000;
const CHARACTER_CAPACITY: usize = 512;
type Graphic = Vec<u8>;
type CharacterSlots = BTreeMap<Graphic, usize>;
type ScreenGraphics = Vec<BTreeSet<Graphic>>;

#[derive(Deserialize)]
struct Palette {
    id: u8,
    colors: Vec<[u8; 3]>,
    tiles: Vec<Tile>,
    #[serde(skip)]
    uses_upper_half: bool,
}

#[derive(Deserialize)]
struct Tile {
    priority: bool,
    collision: u8,
    pixels: Vec<Vec<u8>>,
}

#[derive(Deserialize)]
struct Area {
    vanilla_map_id: usize,
    bg_color: [u8; 3],
    size: [usize; 2],
    screens: Vec<SourceScreen>,
}

#[derive(Deserialize)]
struct SourceScreen {
    position: [usize; 2],
    palettes: Vec<Vec<u8>>,
    tiles: Vec<Vec<usize>>,
    flips: Vec<Vec<u8>>,
}

#[derive(Clone, Copy)]
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
}

pub struct CompiledTheme {
    pub screen_maps: BTreeMap<usize, Vec<u8>>,
    pub map16_definitions: [Vec<u8>; 4],
    pub map16_properties: [Vec<u8>; 4],
    pub background_colors: [u16; 0xa0],
    pub area_assets: Vec<OverworldAreaAssets>,
    pub screen_count: usize,
    pub palette_count: usize,
    pub character_count: usize,
    pub map16_count: usize,
}

pub fn compile(
    root: &Path,
    vanilla_tiles: &[Tile16],
    vanilla_tile_types: &[u8],
    mut area_assets: Vec<OverworldAreaAssets>,
) -> Result<CompiledTheme> {
    let palettes = load_palettes(root)?;
    let (screens, background_colors) = load_screens(root)?;
    ensure!(area_assets.len() == 0xa0);

    let palette_slots = allocate_palettes(&screens, &palettes)?;
    let (character_slots, screen_graphics) =
        allocate_characters(&screens, &palettes, &palette_slots)?;
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
    for (id, words) in definitions.iter().enumerate() {
        let props = std::array::from_fn(|quadrant| properties[quadrant][id]);
        definition_ids.insert((*words, props), u16::try_from(id).unwrap());
    }

    let mut screen_maps = BTreeMap::new();
    for (screen, graphics) in screens.iter().zip(&screen_graphics) {
        let palette_rows = build_palette_rows(screen, &palettes, &palette_slots)?;
        let character_rows = build_character_rows(graphics, &character_slots)?;
        validate_assets(
            screen,
            &palettes,
            &palette_slots,
            &character_slots,
            &palette_rows,
            &character_rows,
        )?;

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
        area_assets[screen.id] = OverworldAreaAssets {
            palette_rows: palette_rows.to_vec(),
            character_rows: character_rows.to_vec(),
            transition_palette_groups: [true; 2],
            transition_sheets: [true; 8],
            sprite_variants: sprites,
        };
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
        screen_maps,
        map16_definitions,
        map16_properties: properties,
        background_colors,
        area_assets,
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
        palettes.insert(palette.id, palette);
    }
    Ok(palettes)
}

fn load_screens(root: &Path) -> Result<(Vec<ThemeScreen>, [u16; 0xa0])> {
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
    for path in paths {
        let area: Area = read_json(&path)?;
        let first_screen = result.len();
        let mut area_palettes = BTreeSet::new();
        let mut components = BTreeMap::new();
        for screen in area.screens {
            components.insert((screen.position[0], screen.position[1]), screen);
        }

        for group_y in 0..area.size[1] / 2 {
            for group_x in 0..area.size[0] / 2 {
                let id = area.vanilla_map_id + group_x + group_y * 8;
                background_colors[id] = encode_bgr555(area.bg_color);
                let mut placements = Vec::with_capacity(64 * 64);
                for component_y in 0..2 {
                    for y in 0..32 {
                        for component_x in 0..2 {
                            let screen = &components
                                [&(group_x * 2 + component_x, group_y * 2 + component_y)];
                            for x in 0..32 {
                                let placement = Placement {
                                    palette: screen.palettes[y][x],
                                    tile: screen.tiles[y][x],
                                    flip: screen.flips[y][x],
                                };
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
                });
            }
        }
        for screen in &mut result[first_screen..] {
            screen.palettes = area_palettes.clone();
        }
    }
    result.sort_by_key(|screen| screen.id);
    Ok((result, background_colors))
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

    // Populate conflicts: two palettes that exist in the same area
    // cannot be assigned to the same slot.
    for screen in screens {
        for &left in &screen.palettes {
            for &right in &screen.palettes {
                if right != left {
                    conflicts.get_mut(&left).unwrap().insert(right);
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
    palette_slots: &BTreeMap<u8, usize>,
) -> Result<(CharacterSlots, ScreenGraphics)> {
    let mut conflicts = BTreeMap::<Vec<u8>, BTreeSet<Vec<u8>>>::new();
    let mut group_graphics = BTreeMap::<usize, BTreeSet<Vec<u8>>>::new();
    for screen in screens {
        let graphics = group_graphics.entry(screen.asset_group).or_default();
        for placement in &screen.placements {
            graphics.insert(build_graphic(placement, palettes, palette_slots));
        }
    }
    for graphics in group_graphics.values() {
        for left in graphics {
            let entry = conflicts.entry(left.clone()).or_default();
            for right in graphics {
                if right != left {
                    entry.insert(right.clone());
                }
            }
        }
    }

    let mut order = Vec::with_capacity(conflicts.len());
    for graphic in conflicts.keys() {
        order.push(graphic.clone());
    }
    order.sort_by_key(|graphic| (Reverse(conflicts[graphic].len()), graphic.clone()));
    let mut slots = BTreeMap::new();
    // Greedy assignment; replace with backtracking or other algorithm later.
    for graphic in order {
        let mut blocked = BTreeSet::new();
        for other in &conflicts[&graphic] {
            if let Some(&slot) = slots.get(other) {
                blocked.insert(slot);
            }
        }
        let slot = (0..CHARACTER_CAPACITY)
            .find(|slot| !blocked.contains(slot))
            .context("Graphics exceed the existing stable character slots")?;
        slots.insert(graphic, slot);
    }

    let mut screen_graphics = Vec::with_capacity(screens.len());
    for screen in screens {
        screen_graphics.push(group_graphics[&screen.asset_group].clone());
    }
    Ok((slots, screen_graphics))
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
    graphics: &BTreeSet<Vec<u8>>,
    slots: &BTreeMap<Vec<u8>, usize>,
) -> Result<[[u8; 512]; 32]> {
    let mut rows = [[0; 512]; 32];
    for graphic in graphics {
        let slot = slots[graphic];
        ensure!(slot < CHARACTER_CAPACITY);
        let encoded = encode_4bpp(graphic);
        let offset = slot % 16 * 32;
        rows[slot / 16][offset..offset + 32].copy_from_slice(&encoded);
    }
    Ok(rows)
}

fn build_map(
    screen: &ThemeScreen,
    palettes: &BTreeMap<u8, Palette>,
    palette_slots: &BTreeMap<u8, usize>,
    character_slots: &BTreeMap<Vec<u8>, usize>,
    definitions: &mut Vec<[u16; 4]>,
    properties: &mut [Vec<u8>; 4],
    definition_ids: &mut BTreeMap<([u16; 4], [u8; 4]), u16>,
) -> Result<Vec<u8>> {
    let mut map = Vec::with_capacity(0x800);
    for map_y in 0..32 {
        for map_x in 0..32 {
            let mut words = [0; 4];
            let mut props = [0; 4];
            for (quadrant, (x, y)) in [
                (map_x * 2, map_y * 2),
                (map_x * 2 + 1, map_y * 2),
                (map_x * 2, map_y * 2 + 1),
                (map_x * 2 + 1, map_y * 2 + 1),
            ]
            .into_iter()
            .enumerate()
            {
                let placement = screen.placements[y * 64 + x];
                let palette = &palettes[&placement.palette];
                let tile = &palette.tiles[placement.tile];
                let graphic = build_graphic(&placement, palettes, palette_slots);
                words[quadrant] = u16::try_from(character_slots[&graphic])?
                    | u16::try_from(2 + palette_slots[&placement.palette] / 2)? << 10
                    | if tile.priority { 1 << 13 } else { 0 }
                    | u16::from(placement.flip) << 14;
                props[quadrant] = if (0x10..0x1c).contains(&tile.collision) {
                    tile.collision ^ placement.flip
                } else {
                    tile.collision
                };
            }
            let id = if let Some(&id) = definition_ids.get(&(words, props)) {
                id
            } else {
                ensure!(
                    definitions.len() < MAP16_CAPACITY,
                    "Desert Map16 definitions exceed {MAP16_CAPACITY}"
                );
                let id = u16::try_from(definitions.len())?;
                definitions.push(words);
                for quadrant in 0..4 {
                    properties[quadrant][usize::from(id)] = props[quadrant];
                }
                definition_ids.insert((words, props), id);
                id
            };
            map.extend_from_slice(&id.to_le_bytes());
        }
    }
    Ok(map)
}

fn validate_assets(
    screen: &ThemeScreen,
    palettes: &BTreeMap<u8, Palette>,
    palette_slots: &BTreeMap<u8, usize>,
    character_slots: &BTreeMap<Vec<u8>, usize>,
    palette_rows: &[[u8; 32]; 6],
    character_rows: &[[u8; 512]; 32],
) -> Result<()> {
    for placement in &screen.placements {
        let palette = &palettes[&placement.palette];
        let source = &palette.tiles[placement.tile];
        let graphic = build_graphic(placement, palettes, palette_slots);
        let slot = character_slots[&graphic];
        let offset = slot % 16 * 32;
        ensure!(
            character_rows[slot / 16][offset..offset + 32] == encode_4bpp(&graphic),
            "screen {:02X}: character encoding mismatch",
            screen.id
        );
        let palette_half = palette_slots[&placement.palette];
        let color_offset = if palette.uses_upper_half {
            0
        } else {
            palette_half % 2 * 8
        };
        for index in 1..palette.colors.len() {
            if source
                .pixels
                .iter()
                .flatten()
                .any(|&pixel| usize::from(pixel) == index)
            {
                let output = (index + color_offset) * 2;
                let actual = u16::from_le_bytes([
                    palette_rows[palette_half / 2][output],
                    palette_rows[palette_half / 2][output + 1],
                ]);
                ensure!(actual == encode_bgr555(palette.colors[index]));
            }
        }
    }
    Ok(())
}

fn build_graphic(
    placement: &Placement,
    palettes: &BTreeMap<u8, Palette>,
    slots: &BTreeMap<u8, usize>,
) -> Vec<u8> {
    let palette = &palettes[&placement.palette];
    let tile = &palette.tiles[placement.tile];
    let offset = if palette.uses_upper_half {
        0
    } else if slots[&placement.palette] % 2 == 1 {
        8
    } else {
        0
    };
    let mut pixels = Vec::with_capacity(64);
    for row in &tile.pixels {
        for &pixel in row {
            pixels.push(if pixel == 0 { 0 } else { pixel + offset });
        }
    }
    pixels
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
