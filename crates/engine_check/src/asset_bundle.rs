use anyhow::{Result, ensure};
use patcher::import::{OverworldAreaAssets, OverworldSpriteVariant};
use std::collections::BTreeMap;

const BANK_SIZE: usize = 0x8000;
const METADATA_START: u32 = 0xa78000;
const PAYLOAD_START: u32 = 0xaa8000;
const METADATA_SIZE: usize = 3 * BANK_SIZE;
const POINTER_TABLE_SIZE: usize = 256 * 3;
pub const CREDITS_COOL_BACKGROUND_KEY: usize = 0xff;
pub const CREDITS_FIRST_KEY: usize = 0xa0;
pub const SPRITE_SEED_KEY: usize = 0xfe;
const PAYLOAD_SIZE: usize = 14 * BANK_SIZE;
// One unit is one 32-byte 4bpp tile; double palette bytes for parsing overhead.
const GRAPHICS_ROW_UNITS: usize = 16;
const PALETTE_ROW_UNITS: usize = 2;
const TRANSITION_BATCH_UNITS: usize = 8 * GRAPHICS_ROW_UNITS;
type InternedSequence = (u32, Vec<u32>, Vec<(u8, u32)>);

pub struct AssetBundle {
    pub metadata_start: u32,
    pub payload_start: u32,
    pub metadata: Vec<u8>,
    pub payload: Vec<u8>,
    pub unique_payloads: usize,
}

#[derive(Clone, Copy)]
pub struct AssetLayout {
    pub payload_start: u32,
    pub payload_size: usize,
}

impl Default for AssetLayout {
    fn default() -> Self {
        Self {
            payload_start: PAYLOAD_START,
            payload_size: PAYLOAD_SIZE,
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
    fn new(base_bank: u8, limit: usize, reserved: usize) -> Self {
        Self {
            base_bank,
            limit,
            next: reserved,
            bytes: vec![0; reserved],
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
    transition_phase: TransitionAssetPhase,
    layout: AssetLayout,
) -> Result<AssetBundle> {
    let mut metadata = Region::new(
        (METADATA_START >> 16) as u8,
        METADATA_SIZE,
        POINTER_TABLE_SIZE,
    );
    let mut payload = Region::new((layout.payload_start >> 16) as u8, layout.payload_size, 0);
    let empty_record = metadata.intern(&[0; 18])?;
    let empty_variants = intern_variant_list(&mut metadata, &[(0xff, empty_record)])?;
    let mut keys = vec![empty_variants; 256];

    for (area_key, area) in areas.iter().enumerate() {
        let mut variants = Vec::with_capacity(area.sprite_variants.len());
        for variant in &area.sprite_variants {
            variants.push((
                variant.max_game_state,
                intern_record(
                    &mut metadata,
                    &mut payload,
                    area,
                    variant,
                    true,
                    transition_phase,
                )?,
            ));
        }
        keys[area_key] = intern_variant_list(&mut metadata, &variants)?;
    }

    for (scene, assets) in credits_overworld {
        let variant = &assets.sprite_variants[0];
        let record = intern_record(
            &mut metadata,
            &mut payload,
            assets,
            variant,
            false,
            transition_phase,
        )?;
        keys[CREDITS_FIRST_KEY + usize::from(*scene)] =
            intern_variant_list(&mut metadata, &[(0xff, record)])?;
    }

    let cool_variant = &credits_cool_background.sprite_variants[0];
    let cool_record = intern_record(
        &mut metadata,
        &mut payload,
        credits_cool_background,
        cool_variant,
        false,
        transition_phase,
    )?;
    keys[CREDITS_COOL_BACKGROUND_KEY] = intern_variant_list(&mut metadata, &[(0xff, cool_record)])?;

    let seed_sequence =
        intern_character_sequence(&mut metadata, &mut payload, &sprite_seed.character_rows)?;
    let mut seed_record = [0; 18];
    seed_record[..3].copy_from_slice(&little_endian_pointer(seed_sequence));
    let seed_record = metadata.intern(&seed_record)?;
    keys[SPRITE_SEED_KEY] = intern_variant_list(&mut metadata, &[(0xff, seed_record)])?;

    for (key, variants) in keys.into_iter().enumerate() {
        let offset = key * 3;
        metadata.bytes[offset..offset + 3].copy_from_slice(&little_endian_pointer(variants));
    }

    Ok(AssetBundle {
        metadata_start: METADATA_START,
        payload_start: layout.payload_start,
        unique_payloads: payload.offsets.len(),
        metadata: metadata.bytes,
        payload: payload.bytes,
    })
}

fn intern_variant_list(metadata: &mut Region, variants: &[(u8, u32)]) -> Result<u32> {
    let mut bytes = Vec::with_capacity(variants.len() * 4);
    for &(max_state, record) in variants {
        bytes.push(max_state);
        bytes.extend_from_slice(&little_endian_pointer(record));
    }
    metadata.intern(&bytes)
}

fn intern_record(
    metadata: &mut Region,
    payload: &mut Region,
    assets: &OverworldAreaAssets,
    sprite_variant: &OverworldSpriteVariant,
    include_transitions: bool,
    transition_phase: TransitionAssetPhase,
) -> Result<u32> {
    let (sequence, palette_sources, character_sources) =
        intern_full_sequence(metadata, payload, assets, sprite_variant)?;
    let mut record = vec![0; 18];
    record[..3].copy_from_slice(&little_endian_pointer(sequence));

    if include_transitions {
        let mut transition_palette_rows = Vec::new();
        for (row, &load) in assets.transition_palette_rows.iter().enumerate() {
            if load {
                transition_palette_rows.push(row);
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
        if !transition_palette_rows.is_empty() || !transition_characters.is_empty() {
            let character_count = ((TRANSITION_BATCH_UNITS
                - transition_palette_rows.len() * PALETTE_ROW_UNITS)
                / GRAPHICS_ROW_UNITS)
                .min(transition_characters.len());
            let mut batch = Vec::new();
            for &row in &transition_palette_rows {
                descriptor(&mut batch, palette_sources[row], u8::try_from(row + 2)?);
            }
            batch.push(0);
            for &(destination, source) in &transition_characters[..character_count] {
                descriptor(&mut batch, source, destination);
            }
            batch.push(0);
            transition_batches.push(metadata.intern(&batch)?);
            character_offset = character_count;
        }
        let rows_per_batch = TRANSITION_BATCH_UNITS / GRAPHICS_ROW_UNITS;
        for rows in transition_characters[character_offset..].chunks(rows_per_batch) {
            let mut batch = vec![0];
            for &(destination, source) in rows {
                descriptor(&mut batch, source, destination);
            }
            batch.push(0);
            transition_batches.push(metadata.intern(&batch)?);
        }
        let schedule = intern_schedule(metadata, &transition_batches, transition_phase)?;
        for offset in [3, 6, 9, 12] {
            record[offset..offset + 3].copy_from_slice(&little_endian_pointer(schedule));
        }
    }
    metadata.intern(&record)
}

fn intern_schedule(
    metadata: &mut Region,
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
    metadata.intern(&schedule)
}

fn intern_full_sequence(
    metadata: &mut Region,
    payload: &mut Region,
    assets: &OverworldAreaAssets,
    sprite_variant: &OverworldSpriteVariant,
) -> Result<InternedSequence> {
    let palette_sources = assets
        .palette_rows
        .iter()
        .map(|row| payload.intern(row))
        .collect::<Result<Vec<_>>>()?;
    let mut character_sources = assets
        .character_rows
        .iter()
        .enumerate()
        .map(|(destination, row)| Ok((u8::try_from(destination)?, payload.intern(row)?)))
        .collect::<Result<Vec<_>>>()?;
    character_sources.extend(
        sprite_variant
            .character_rows
            .iter()
            .map(|(destination, row)| Ok((*destination, payload.intern(row)?)))
            .collect::<Result<Vec<_>>>()?,
    );

    let mut sequence = Vec::new();
    for (batch_index, rows) in character_sources.chunks(8).enumerate() {
        let mut batch = Vec::new();
        if batch_index == 0 {
            for (row, &source) in palette_sources.iter().enumerate() {
                descriptor(&mut batch, source, u8::try_from(row + 2)?);
            }
        }
        batch.push(0);
        for &(destination, source) in rows {
            descriptor(&mut batch, source, destination);
        }
        batch.push(0);
        bank_first_pointer(&mut sequence, metadata.intern(&batch)?);
    }
    sequence.push(0);
    Ok((
        metadata.intern(&sequence)?,
        palette_sources,
        character_sources,
    ))
}

fn intern_character_sequence(
    metadata: &mut Region,
    payload: &mut Region,
    rows: &[(u8, [u8; 512])],
) -> Result<u32> {
    let sources = rows
        .iter()
        .map(|(destination, row)| Ok((*destination, payload.intern(row)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut sequence = Vec::new();
    for rows in sources.chunks(8) {
        let mut batch = vec![0];
        for &(destination, source) in rows {
            descriptor(&mut batch, source, destination);
        }
        batch.push(0);
        bank_first_pointer(&mut sequence, metadata.intern(&batch)?);
    }
    sequence.push(0);
    metadata.intern(&sequence)
}

fn descriptor(output: &mut Vec<u8>, source: u32, destination: u8) {
    bank_first_pointer(output, source);
    output.push(destination);
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
