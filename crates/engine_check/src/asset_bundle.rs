use anyhow::{Result, ensure};
use patcher::import::{OverworldAreaAssets, OverworldSpriteVariant};
use std::collections::BTreeMap;

const BANK_SIZE: usize = 0x8000;
const POINTER_TABLE_START: u32 = 0xa78000;
const DATA_START: u32 = 0xaa8000;
const AREA_POINTER_TABLE_SIZE: usize = 256 * 3;
pub const DYNAMIC_TILE_GROUP_COUNT: usize = 22;
const POINTER_TABLE_SIZE: usize = AREA_POINTER_TABLE_SIZE + DYNAMIC_TILE_GROUP_COUNT * 3;
pub const CREDITS_COOL_BACKGROUND_KEY: usize = 0xff;
pub const CREDITS_FIRST_KEY: usize = 0xa0;
pub const SPRITE_SEED_KEY: usize = 0xfe;
const DATA_SIZE: usize = 14 * BANK_SIZE;
// One unit is one 32-byte 4bpp tile; charge one unit per eight palette colors.
const GRAPHICS_ROW_UNITS: usize = 16;
const TRANSITION_BATCH_UNITS: usize = 8 * GRAPHICS_ROW_UNITS;
type InternedSequence = (u32, Vec<u32>, Vec<(u8, u32)>);

pub struct AssetBundle {
    pub pointer_table_start: u32,
    pub data_start: u32,
    pub pointer_table: Vec<u8>,
    pub data: Vec<u8>,
    pub unique_blocks: usize,
}

pub struct DynamicTileEntry {
    pub source: u16,
    pub x_offset: i8,
    pub y_offset: i8,
    pub width: u8,
    pub height: u8,
    pub before: Vec<u16>,
    pub after_frames: Vec<Vec<u16>>,
}

#[derive(Clone, Copy)]
pub struct AssetLayout {
    pub data_start: u32,
    pub data_size: usize,
}

impl Default for AssetLayout {
    fn default() -> Self {
        Self {
            data_start: DATA_START,
            data_size: DATA_SIZE,
        }
    }
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum TransitionAssetPhase {
    PreScroll,
    Scroll,
    PostScroll,
}

struct Region {
    base_bank: u8,
    limit: usize,
    next: usize,
    bytes: Vec<u8>,
    offsets: BTreeMap<Vec<u8>, u32>,
}

impl Region {
    fn new(base_bank: u8, limit: usize) -> Self {
        Self {
            base_bank,
            limit,
            next: 0,
            bytes: Vec::new(),
            offsets: BTreeMap::new(),
        }
    }

    fn intern(&mut self, bytes: &[u8]) -> Result<u32> {
        if let Some(&address) = self.offsets.get(bytes) {
            return Ok(address);
        }

        let mut offset = self.next;
        if offset % BANK_SIZE + bytes.len() > BANK_SIZE {
            offset = offset.next_multiple_of(BANK_SIZE);
        }
        ensure!(
            offset + bytes.len() <= self.limit,
            "generated asset region overflow"
        );
        self.bytes.resize(offset, 0);
        self.bytes.extend_from_slice(bytes);
        self.next = offset + bytes.len();

        let address = snes_address(self.base_bank, offset);
        self.offsets.insert(bytes.to_vec(), address);
        Ok(address)
    }
}

pub fn build(
    areas: &[OverworldAreaAssets],
    credits_overworld: &[(u8, OverworldAreaAssets)],
    credits_cool_background: &OverworldAreaAssets,
    sprite_seed: &OverworldSpriteVariant,
    dynamic_tile_groups: &[Vec<DynamicTileEntry>],
    transition_phase: TransitionAssetPhase,
    layout: AssetLayout,
) -> Result<AssetBundle> {
    let mut data = Region::new((layout.data_start >> 16) as u8, layout.data_size);
    let mut empty_record = [0; 32];
    empty_record[24] = 0xff;
    let empty_record = data.intern(&empty_record)?;
    let empty_variants = intern_variant_list(&mut data, &[(0xff, empty_record)])?;
    let mut keys = vec![empty_variants; 256];

    for (area_key, area) in areas.iter().enumerate() {
        let mut variants = Vec::with_capacity(area.sprite_variants.len());
        for variant in &area.sprite_variants {
            variants.push((
                variant.max_game_state,
                intern_record(&mut data, area, variant, true, transition_phase)?,
            ));
        }
        keys[area_key] = intern_variant_list(&mut data, &variants)?;
    }

    for (scene, assets) in credits_overworld {
        let variant = &assets.sprite_variants[0];
        let record = intern_record(&mut data, assets, variant, false, transition_phase)?;
        keys[CREDITS_FIRST_KEY + usize::from(*scene)] =
            intern_variant_list(&mut data, &[(0xff, record)])?;
    }

    let cool_variant = &credits_cool_background.sprite_variants[0];
    let cool_record = intern_record(
        &mut data,
        credits_cool_background,
        cool_variant,
        false,
        transition_phase,
    )?;
    keys[CREDITS_COOL_BACKGROUND_KEY] = intern_variant_list(&mut data, &[(0xff, cool_record)])?;

    let seed_sequence = intern_character_sequence(&mut data, &sprite_seed.character_rows)?;
    let mut seed_record = [0; 32];
    seed_record[..3].copy_from_slice(&little_endian_pointer(seed_sequence));
    seed_record[24] = 0xff;
    let seed_record = data.intern(&seed_record)?;
    keys[SPRITE_SEED_KEY] = intern_variant_list(&mut data, &[(0xff, seed_record)])?;

    let mut pointer_table = vec![0; POINTER_TABLE_SIZE];
    for (key, variants) in keys.into_iter().enumerate() {
        let offset = key * 3;
        pointer_table[offset..offset + 3].copy_from_slice(&little_endian_pointer(variants));
    }
    for (group_index, group) in dynamic_tile_groups.iter().enumerate() {
        if group.is_empty() {
            continue;
        }
        let mut bytes = Vec::with_capacity(1 + group.len() * 7);
        bytes.push(u8::try_from(group.len())?);
        for entry in group {
            let mut descriptor = Vec::new();
            descriptor.push(entry.width);
            descriptor.push(entry.height);
            descriptor.push(u8::try_from(entry.after_frames.len())?);
            for &id in &entry.before {
                descriptor.extend_from_slice(&id.to_le_bytes());
            }
            for frame in &entry.after_frames {
                for &id in frame {
                    descriptor.extend_from_slice(&id.to_le_bytes());
                }
            }

            bytes.extend_from_slice(&entry.source.to_le_bytes());
            bytes.push(entry.x_offset.to_le_bytes()[0]);
            bytes.push(entry.y_offset.to_le_bytes()[0]);
            bytes.extend_from_slice(&little_endian_pointer(data.intern(&descriptor)?));
        }
        let offset = AREA_POINTER_TABLE_SIZE + group_index * 3;
        pointer_table[offset..offset + 3]
            .copy_from_slice(&little_endian_pointer(data.intern(&bytes)?));
    }

    Ok(AssetBundle {
        pointer_table_start: POINTER_TABLE_START,
        data_start: layout.data_start,
        pointer_table,
        unique_blocks: data.offsets.len(),
        data: data.bytes,
    })
}

fn intern_variant_list(data: &mut Region, variants: &[(u8, u32)]) -> Result<u32> {
    let mut bytes = Vec::with_capacity(variants.len() * 4);
    for &(max_state, record) in variants {
        bytes.push(max_state);
        bytes.extend_from_slice(&little_endian_pointer(record));
    }
    data.intern(&bytes)
}

fn intern_record(
    data: &mut Region,
    assets: &OverworldAreaAssets,
    sprite_variant: &OverworldSpriteVariant,
    include_transitions: bool,
    transition_phase: TransitionAssetPhase,
) -> Result<u32> {
    let (sequence, palette_sources, character_sources) =
        intern_full_sequence(data, assets, sprite_variant)?;
    let mut record = vec![0; 32];
    record[..3].copy_from_slice(&little_endian_pointer(sequence));
    let animations = intern_animation_tracks(data, &assets.animation_tracks)?;
    if animations != 0 {
        record[15..18].copy_from_slice(&little_endian_pointer(animations));
    }
    if !assets.entrances.is_empty() {
        let mut entrances = Vec::with_capacity(1 + assets.entrances.len() * 4);
        entrances.push(u8::try_from(assets.entrances.len())?);
        for entrance in &assets.entrances {
            entrances.extend_from_slice(&entrance.map16_offset.to_le_bytes());
            entrances.push(entrance.entrance_id);
            entrances.push(if entrance.allows_frog_dwarf { 1 } else { 0 });
        }
        record[18..21].copy_from_slice(&little_endian_pointer(data.intern(&entrances)?));
    }
    if !assets.pit_entrances.is_empty() {
        let mut entrances = Vec::with_capacity(1 + assets.pit_entrances.len() * 3);
        entrances.push(u8::try_from(assets.pit_entrances.len())?);
        for entrance in &assets.pit_entrances {
            entrances.extend_from_slice(&entrance.map16_offset.to_le_bytes());
            entrances.push(entrance.entrance_id);
        }
        record[21..24].copy_from_slice(&little_endian_pointer(data.intern(&entrances)?));
    }
    if !assets.special_transitions.is_empty() {
        let mut transitions = Vec::with_capacity(1 + assets.special_transitions.len() * 5);
        transitions.push(u8::try_from(assets.special_transitions.len())?);
        for transition in &assets.special_transitions {
            transitions.extend_from_slice(&transition.map16_offset.to_le_bytes());
            transitions.extend_from_slice(&transition.special_area.to_le_bytes());
            transitions.push(transition.direction);
        }
        record[29..32].copy_from_slice(&little_endian_pointer(data.intern(&transitions)?));
    }
    record[24] = assets.background.layering;
    record[25] = assets.background.camera_follow_x.to_le_bytes()[0];
    record[26] = assets.background.camera_drift_x.to_le_bytes()[0];
    record[27] = assets.background.camera_follow_y.to_le_bytes()[0];
    record[28] = assets.background.camera_drift_y.to_le_bytes()[0];

    if include_transitions {
        let mut transition_palettes = Vec::new();
        for range in &assets.transition_palette_ranges {
            let mut start = usize::from(range.start_color);
            let end = start + usize::from(range.color_count);
            while start < end {
                let row = start / 16 - 2;
                let row_offset = start % 16;
                let color_count = (16 - row_offset).min(end - start);
                transition_palettes.push((
                    palette_sources[row] + u32::try_from(row_offset * 2)?,
                    u8::try_from(start)?,
                    u8::try_from(color_count)?,
                ));
                start += color_count;
            }
        }
        let mut transition_characters = Vec::new();
        for (row, &load) in assets.transition_character_rows.iter().enumerate() {
            if load {
                transition_characters.push((u8::try_from(row)?, character_sources[row].1));
            }
        }
        transition_characters.extend_from_slice(&character_sources[assets.character_rows.len()..]);

        let mut transition_batches = Vec::new();
        let mut character_offset = 0;
        if !transition_palettes.is_empty() || !transition_characters.is_empty() {
            let mut palette_units = 0;
            for &(_, _, color_count) in &transition_palettes {
                palette_units += usize::from(color_count).div_ceil(8);
            }
            let character_count = ((TRANSITION_BATCH_UNITS - palette_units) / GRAPHICS_ROW_UNITS)
                .min(transition_characters.len());
            let mut batch = Vec::new();
            for &(source, start_color, color_count) in &transition_palettes {
                palette_descriptor(&mut batch, source, start_color, color_count);
            }
            batch.push(0);
            for &(destination, source) in &transition_characters[..character_count] {
                descriptor(&mut batch, source, destination);
            }
            batch.push(0);
            transition_batches.push(data.intern(&batch)?);
            character_offset = character_count;
        }
        let rows_per_batch = TRANSITION_BATCH_UNITS / GRAPHICS_ROW_UNITS;
        for rows in transition_characters[character_offset..].chunks(rows_per_batch) {
            let mut batch = vec![0];
            for &(destination, source) in rows {
                descriptor(&mut batch, source, destination);
            }
            batch.push(0);
            transition_batches.push(data.intern(&batch)?);
        }
        let schedule = intern_schedule(data, &transition_batches, transition_phase)?;
        for offset in [3, 6, 9, 12] {
            record[offset..offset + 3].copy_from_slice(&little_endian_pointer(schedule));
        }
    }
    data.intern(&record)
}

fn intern_animation_tracks(
    data: &mut Region,
    tracks: &[patcher::import::OverworldAnimationTrack],
) -> Result<u32> {
    if tracks.is_empty() {
        return Ok(0);
    }

    let mut list = Vec::new();
    for track in tracks {
        let frame_count = u8::try_from(track.frames.len())?;
        let (initial_frame, initial_countdown) = get_initial_animation_state(track);
        let mut definition = vec![
            frame_count,
            track.frame_hold,
            u8::try_from(initial_frame)?,
            initial_countdown,
        ];
        for frame in &track.frames {
            let mut batch = vec![0];
            for (&destination, row) in track.destination_rows.iter().zip(frame) {
                descriptor(&mut batch, data.intern(row)?, destination);
            }
            batch.push(0);
            bank_first_pointer(&mut definition, data.intern(&batch)?);
        }
        bank_first_pointer(&mut list, data.intern(&definition)?);
    }
    list.push(0);
    data.intern(&list)
}

fn get_initial_animation_state(track: &patcher::import::OverworldAnimationTrack) -> (usize, u8) {
    let phase = track.phase_offset % (track.frames.len() * usize::from(track.frame_hold));
    (
        phase / usize::from(track.frame_hold),
        track.frame_hold - (phase % usize::from(track.frame_hold)) as u8,
    )
}

fn intern_schedule(
    data: &mut Region,
    batches: &[u32],
    transition_phase: TransitionAssetPhase,
) -> Result<u32> {
    let mut schedule = Vec::with_capacity(3 + batches.len() * 4);
    if matches!(transition_phase, TransitionAssetPhase::PreScroll) {
        for &source in batches {
            bank_first_pointer(&mut schedule, source);
        }
    }
    schedule.push(0);
    if matches!(transition_phase, TransitionAssetPhase::Scroll) {
        for (frame, &source) in batches.iter().enumerate() {
            schedule.push(u8::try_from(frame)?);
            bank_first_pointer(&mut schedule, source);
        }
    }
    schedule.push(0xff);
    if matches!(transition_phase, TransitionAssetPhase::PostScroll) {
        for &source in batches {
            bank_first_pointer(&mut schedule, source);
        }
    }
    schedule.push(0);
    data.intern(&schedule)
}

fn intern_full_sequence(
    data: &mut Region,
    assets: &OverworldAreaAssets,
    sprite_variant: &OverworldSpriteVariant,
) -> Result<InternedSequence> {
    let palette_sources = assets
        .palette_rows
        .iter()
        .map(|row| data.intern(row))
        .collect::<Result<Vec<_>>>()?;
    let mut character_rows = assets.character_rows.clone();
    for track in &assets.animation_tracks {
        let (frame, _) = get_initial_animation_state(track);
        for (&destination, row) in track.destination_rows.iter().zip(&track.frames[frame]) {
            character_rows[usize::from(destination)] = *row;
        }
    }
    let mut character_sources = character_rows
        .iter()
        .enumerate()
        .map(|(destination, row)| Ok((u8::try_from(destination)?, data.intern(row)?)))
        .collect::<Result<Vec<_>>>()?;
    character_sources.extend(
        sprite_variant
            .character_rows
            .iter()
            .map(|(destination, row)| Ok((*destination, data.intern(row)?)))
            .collect::<Result<Vec<_>>>()?,
    );

    let mut sequence = Vec::new();
    for (batch_index, rows) in character_sources.chunks(8).enumerate() {
        let mut batch = Vec::new();
        if batch_index == 0 {
            for (row, &source) in palette_sources.iter().enumerate() {
                palette_descriptor(&mut batch, source, u8::try_from((row + 2) * 16)?, 16);
            }
        }
        batch.push(0);
        for &(destination, source) in rows {
            descriptor(&mut batch, source, destination);
        }
        batch.push(0);
        bank_first_pointer(&mut sequence, data.intern(&batch)?);
    }
    sequence.push(0);
    Ok((data.intern(&sequence)?, palette_sources, character_sources))
}

fn intern_character_sequence(data: &mut Region, rows: &[(u8, [u8; 512])]) -> Result<u32> {
    let sources = rows
        .iter()
        .map(|(destination, row)| Ok((*destination, data.intern(row)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut sequence = Vec::new();
    for rows in sources.chunks(8) {
        let mut batch = vec![0];
        for &(destination, source) in rows {
            descriptor(&mut batch, source, destination);
        }
        batch.push(0);
        bank_first_pointer(&mut sequence, data.intern(&batch)?);
    }
    sequence.push(0);
    data.intern(&sequence)
}

fn descriptor(output: &mut Vec<u8>, source: u32, destination: u8) {
    bank_first_pointer(output, source);
    output.push(destination);
}

fn palette_descriptor(output: &mut Vec<u8>, source: u32, start_color: u8, color_count: u8) {
    descriptor(output, source, start_color);
    output.push(color_count);
}

fn bank_first_pointer(output: &mut Vec<u8>, address: u32) {
    output.push((address >> 16) as u8);
    output.extend_from_slice(&(address as u16).to_le_bytes());
}

fn little_endian_pointer(address: u32) -> [u8; 3] {
    [address as u8, (address >> 8) as u8, (address >> 16) as u8]
}

fn snes_address(base_bank: u8, offset: usize) -> u32 {
    (u32::from(base_bank) + u32::try_from(offset / BANK_SIZE).unwrap()) << 16
        | u32::try_from(0x8000 + offset % BANK_SIZE).unwrap()
}
