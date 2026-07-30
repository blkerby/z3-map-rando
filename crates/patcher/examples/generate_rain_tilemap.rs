use anyhow::{Context, Result, ensure};
use patcher::{
    PcAddr, SnesAddr,
    import::{FlatMap16, Importer},
};
use std::{collections::BTreeMap, env, fs};

const FLAT_MAPS_START: SnesAddr = SnesAddr(0x200000);
const RAIN_OVERLAY: usize = 0x9f;
const BLANK: u16 = 0x026f;
// Index bits identify the left and right 8x8 columns within a Map16 tile.
const FALLING_RAIN_TILE_BY_COLUMNS: [u16; 4] = [BLANK, 0x0c62, 0x0c63, 0x0c64];
const SPLASHES: [u16; 3] = [0x0c65, 0x0c66, 0x0c67];
const SPLASH_WITH_DROP: u16 = 0x0c68;
const SPLASH_ANCHORS: [(usize, usize); 8] = [
    (9, 4),
    (1, 6),
    (10, 6),
    (12, 8),
    (4, 9),
    (5, 11),
    (9, 12),
    (7, 14),
];

fn load_rain_map(flat: &FlatMap16) -> Vec<u16> {
    let pointer = &flat.screen_pointers[RAIN_OVERLAY * 3..RAIN_OVERLAY * 3 + 3];
    let pointer = u32::from_le_bytes([pointer[0], pointer[1], pointer[2], 0]);
    let rain_start: PcAddr = SnesAddr(pointer).into();
    let flat_start: PcAddr = FLAT_MAPS_START.into();
    let offset = (rain_start.0 - flat_start.0) as usize;
    flat.maps[offset..offset + 32 * 16 * 2]
        .chunks_exact(2)
        .map(|bytes| u16::from_le_bytes(bytes.try_into().unwrap()))
        .collect()
}

fn align_splashes(map: &mut [u16]) -> Result<()> {
    let phases: Vec<_> = SPLASH_ANCHORS
        .iter()
        .map(|&(x, y)| match map[y * 32 + x] {
            SPLASH_WITH_DROP => Ok(0),
            tile => SPLASHES
                .iter()
                .position(|&splash| splash == tile)
                .context("rain splash anchor is not a splash tile"),
        })
        .collect::<Result<_>>()?;

    for tile in &mut *map {
        if SPLASHES.contains(tile) || *tile == SPLASH_WITH_DROP {
            *tile = BLANK;
        }
    }
    for ((x, y), phase) in SPLASH_ANCHORS.into_iter().zip(phases) {
        for step in 0..4 {
            let index = ((y + step * 4) % 16) * 32 + (x + step * 8) % 32;
            map[index] = SPLASHES.get((phase + step) % 4).copied().unwrap_or(BLANK);
        }
    }
    Ok(())
}

fn adjacent_sequences_differ(left: u8, right: u8) -> bool {
    left == 0 || right == 0 || left != right
}

fn no_three_side_by_side(left: u8, middle: u8, right: u8) -> bool {
    left == 0 || middle == 0 || right == 0
}

fn choices(original: u8, fixed: u8, available: u8) -> Vec<(u8, u32)> {
    (0u8..16)
        .filter(|mask| mask.count_ones() <= 2 && mask & fixed == fixed && mask & !available == 0)
        .map(|mask| (mask, (original ^ mask).count_ones()))
        .collect()
}

fn optimize_row(original: &[u8; 64], fixed: &[u8; 64], available: &[u8; 64]) -> [u8; 64] {
    // Start at an already-empty position. Keeping it empty breaks the
    // horizontal ring, so the rest can be optimized as one linear row.
    let separator = (0..64)
        .find(|&x| original[x] == 0 && fixed[x] == 0)
        .expect("rain row has no empty position");
    let original: [u8; 64] = std::array::from_fn(|x| original[(x + separator) % 64]);
    let fixed: [u8; 64] = std::array::from_fn(|x| fixed[(x + separator) % 64]);
    let available: [u8; 64] = std::array::from_fn(|x| available[(x + separator) % 64]);
    let choices: [Vec<_>; 64] =
        std::array::from_fn(|x| choices(original[x], fixed[x], available[x]));
    let original_density: u32 = original.iter().map(|mask| mask.count_ones()).sum();
    let mut states = BTreeMap::from([((0, 0, 0), (0, vec![0]))]);

    for options in &choices[1..] {
        let mut next_states = BTreeMap::new();
        for (&(two_back, previous, density), (cost, path)) in &states {
            for &(current, current_cost) in options {
                if !adjacent_sequences_differ(previous, current)
                    || !no_three_side_by_side(two_back, previous, current)
                {
                    continue;
                }
                let new_cost = cost + current_cost;
                let entry = next_states
                    .entry((previous, current, density + current.count_ones()))
                    .or_insert_with(|| (u32::MAX, Vec::new()));
                if new_cost < entry.0 {
                    let mut new_path = path.clone();
                    new_path.push(current);
                    *entry = (new_cost, new_path);
                }
            }
        }
        states = next_states;
    }

    let (_, path) = states
        .into_iter()
        .min_by_key(|((_, _, density), (cost, _))| (density.abs_diff(original_density), *cost))
        .expect("no valid rain row");
    let mut result = [0; 64];
    for (x, mask) in path.1.into_iter().enumerate() {
        result[(x + separator) % 64] = mask;
    }
    result
}

fn falling_rain_sequences(map: &[u16], y: usize) -> ([u8; 64], [u8; 64], [u8; 64]) {
    // Each output byte describes one visible 8-pixel column; bit N says that
    // falling rain occupies the column during animation step N.
    let mut original = [0; 64];
    let mut fixed = [0; 64];
    let mut available = [0; 64];

    for x in 0..64 {
        let column_bit = 1 << (x & 1);
        for step in 0..4 {
            let index = ((y + step * 4) % 16) * 32 + ((x / 2 + step * 8) % 32);
            let frame_bit = 1 << step;
            if let Some(columns) = FALLING_RAIN_TILE_BY_COLUMNS
                .iter()
                .position(|&tile| tile == map[index])
            {
                available[x] |= frame_bit;
                if columns & column_bit != 0 {
                    original[x] |= frame_bit;
                }
            } else if column_bit == 2 && map[index] == SPLASHES[2] {
                original[x] |= frame_bit;
                fixed[x] |= frame_bit;
                available[x] |= frame_bit;
            }
        }
    }
    (original, fixed, available)
}

fn adjust_falling_rain(map: &mut [u16]) {
    let targets: [[u8; 64]; 4] = std::array::from_fn(|y| {
        let (original, fixed, available) = falling_rain_sequences(map, y);
        optimize_row(&original, &fixed, &available)
    });

    for tile in &mut *map {
        if FALLING_RAIN_TILE_BY_COLUMNS.contains(tile) {
            *tile = BLANK;
        }
    }
    for (y, row) in targets.into_iter().enumerate() {
        for (x, sequence) in row.into_iter().enumerate() {
            let column_bit = 1 << (x & 1);
            for step in 0..4 {
                if sequence & 1 << step == 0 {
                    continue;
                }
                let index = ((y + step * 4) % 16) * 32 + ((x / 2 + step * 8) % 32);
                if let Some(columns) = FALLING_RAIN_TILE_BY_COLUMNS
                    .iter()
                    .position(|&tile| tile == map[index])
                {
                    map[index] = FALLING_RAIN_TILE_BY_COLUMNS[columns | column_bit];
                }
            }
        }
    }
}

fn validate(map: &[u16]) {
    for y in 0..4 {
        let (sequences, _, _) = falling_rain_sequences(map, y);
        for x in 0..64 {
            assert!(sequences[x].count_ones() <= 2);
            let next = sequences[(x + 1) % 64];
            let next_two = sequences[(x + 2) % 64];
            assert!(adjacent_sequences_differ(sequences[x], next));
            assert!(no_three_side_by_side(sequences[x], next, next_two));
        }
    }
}

fn falling_rain_count(map: &[u16]) -> u32 {
    map.iter()
        .map(|tile| {
            FALLING_RAIN_TILE_BY_COLUMNS
                .iter()
                .position(|rain| rain == tile)
                .map_or(
                    u32::from(SPLASHES[2] == *tile || SPLASH_WITH_DROP == *tile),
                    |columns| columns.count_ones(),
                )
        })
        .sum()
}

fn main() -> Result<()> {
    let path = env::args_os()
        .nth(1)
        .context("usage: generate_rain_tilemap VANILLA_ROM")?;
    let mut importer = Importer::new(fs::read(path)?)?;
    let mut map = load_rain_map(&importer.flat_map16()?);
    let vanilla_falling_rain = falling_rain_count(&map);
    align_splashes(&mut map)?;
    adjust_falling_rain(&mut map);
    validate(&map);
    ensure!(map.len() == 32 * 16);
    eprintln!(
        "falling-rain animation steps: {} generated, {vanilla_falling_rain} vanilla",
        falling_rain_count(&map)
    );

    println!("pub const RAIN_TILEMAP: [[u16; 32]; 16] = [");
    for row in map.chunks_exact(32) {
        print!("    [");
        for tile in row {
            print!("0x{tile:04x}, ");
        }
        println!("],");
    }
    println!("];");
    Ok(())
}
