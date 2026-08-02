use anyhow::{Result, ensure};
use patcher::import::{OverworldAreaAssets, OverworldSpriteVariant};
use std::collections::BTreeMap;

const BANK_SIZE: usize = 0x8000;
pub const METADATA_START: u32 = 0xa78000;
pub const PAYLOAD_START: u32 = 0xaa8000;
const METADATA_BANK: u8 = (METADATA_START >> 16) as u8;
const METADATA_SIZE: usize = 3 * BANK_SIZE;
const POINTER_TABLE_SIZE: usize = 256 * 3;
pub const CREDITS_COOL_BACKGROUND_KEY: usize = 0xff;
pub const CREDITS_FIRST_KEY: usize = 0xa0;
pub const SPRITE_SEED_KEY: usize = 0xfe;
const PAYLOAD_BANK: u8 = (PAYLOAD_START >> 16) as u8;
const PAYLOAD_SIZE: usize = 14 * BANK_SIZE;

pub struct AssetBundle {
    pub metadata: Vec<u8>,
    pub payload: Vec<u8>,
    pub unique_payloads: usize,
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

    fn intern(&mut self, bytes: &[u8], alignment: usize) -> Result<u32> {
        if let Some(&address) = self.offsets.get(bytes) {
            return Ok(address);
        }

        let mut offset = self.next.next_multiple_of(alignment);
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
) -> Result<AssetBundle> {
    ensure!(areas.len() == 0xa0, "expected 160 overworld asset sets");

    let mut metadata = Region::new(METADATA_BANK, METADATA_SIZE, POINTER_TABLE_SIZE);
    let mut payload = Region::new(PAYLOAD_BANK, PAYLOAD_SIZE, 0);
    let empty_record = metadata.intern(&[0; 18], 1)?;
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
        ensure!(assets.sprite_variants.len() == 1);
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

    ensure!(credits_cool_background.sprite_variants.len() == 1);
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
    let seed_record = metadata.intern(&seed_record, 1)?;
    keys[SPRITE_SEED_KEY] = intern_variant_list(&mut metadata, &[(0xff, seed_record)])?;

    for (key, variants) in keys.into_iter().enumerate() {
        let offset = key * 3;
        metadata.bytes[offset..offset + 3].copy_from_slice(&little_endian_pointer(variants));
    }

    let bundle = AssetBundle {
        unique_payloads: payload.offsets.len(),
        metadata: metadata.bytes,
        payload: payload.bytes,
    };
    validate(
        &bundle,
        areas,
        credits_overworld,
        credits_cool_background,
        sprite_seed,
    )?;
    Ok(bundle)
}

fn intern_variant_list(metadata: &mut Region, variants: &[(u8, u32)]) -> Result<u32> {
    ensure!(
        variants
            .last()
            .is_some_and(|(max_state, _)| *max_state == 0xff),
        "asset variants must end at game state FF"
    );
    ensure!(
        variants.windows(2).all(|pair| pair[0].0 < pair[1].0),
        "asset variant bounds must be strictly increasing"
    );
    let mut bytes = Vec::with_capacity(variants.len() * 4);
    for &(max_state, record) in variants {
        bytes.push(max_state);
        bytes.extend_from_slice(&little_endian_pointer(record));
    }
    metadata.intern(&bytes, 1)
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
        let transition_palette_rows = assets
            .transition_palette_groups
            .iter()
            .zip([0..3, 3..6])
            .filter(|(load, _)| **load)
            .flat_map(|(_, rows)| rows)
            .collect::<Vec<_>>();
        let palette_batch = if transition_palette_rows.is_empty() {
            None
        } else {
            let mut batch = Vec::with_capacity(transition_palette_rows.len() * 4 + 2);
            for &row in &transition_palette_rows {
                descriptor(&mut batch, palette_sources[row], u8::try_from(row + 2)?);
            }
            batch.extend_from_slice(&[0, 0]);
            Some(metadata.intern(&batch, 1)?)
        };

        let mut transition_characters = assets
            .transition_sheets
            .iter()
            .enumerate()
            .filter(|(_, load)| **load)
            .flat_map(|(sheet, _)| sheet * 4..sheet * 4 + 4)
            .map(|row| (u8::try_from(row).unwrap(), character_sources[row].1))
            .collect::<Vec<_>>();
        transition_characters.extend_from_slice(&character_sources[assets.character_rows.len()..]);

        let mut transition_batches = Vec::new();
        for rows in transition_characters.chunks(8) {
            let mut batch = vec![0];
            for &(destination, source) in rows {
                descriptor(&mut batch, source, destination);
            }
            batch.push(0);
            transition_batches.push(metadata.intern(&batch, 1)?);
        }
        let batches = palette_batch
            .into_iter()
            .chain(transition_batches)
            .collect::<Vec<_>>();
        let schedule = intern_schedule(metadata, &batches, transition_phase)?;
        for offset in [3, 6, 9, 12] {
            record[offset..offset + 3].copy_from_slice(&little_endian_pointer(schedule));
        }
    }
    metadata.intern(&record, 1)
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
    metadata.intern(&schedule, 1)
}

fn intern_full_sequence(
    metadata: &mut Region,
    payload: &mut Region,
    assets: &OverworldAreaAssets,
    sprite_variant: &OverworldSpriteVariant,
) -> Result<(u32, Vec<u32>, Vec<(u8, u32)>)> {
    ensure!(
        assets.palette_rows.len() == 6,
        "expected six BG palette rows"
    );
    ensure!(
        assets.character_rows.len() == 32,
        "expected 32 BG character rows"
    );
    validate_sprite_rows(&sprite_variant.character_rows)?;

    let palette_sources = assets
        .palette_rows
        .iter()
        .map(|row| payload.intern(row, 32))
        .collect::<Result<Vec<_>>>()?;
    let mut character_sources = assets
        .character_rows
        .iter()
        .enumerate()
        .map(|(destination, row)| Ok((u8::try_from(destination)?, payload.intern(row, 512)?)))
        .collect::<Result<Vec<_>>>()?;
    character_sources.extend(
        sprite_variant
            .character_rows
            .iter()
            .map(|(destination, row)| Ok((*destination, payload.intern(row, 512)?)))
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
        bank_first_pointer(&mut sequence, metadata.intern(&batch, 1)?);
    }
    sequence.push(0);
    Ok((
        metadata.intern(&sequence, 1)?,
        palette_sources,
        character_sources,
    ))
}

fn intern_character_sequence(
    metadata: &mut Region,
    payload: &mut Region,
    rows: &[(u8, [u8; 512])],
) -> Result<u32> {
    validate_sprite_rows(rows)?;
    let sources = rows
        .iter()
        .map(|(destination, row)| Ok((*destination, payload.intern(row, 512)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut sequence = Vec::new();
    for rows in sources.chunks(8) {
        let mut batch = vec![0];
        for &(destination, source) in rows {
            descriptor(&mut batch, source, destination);
        }
        batch.push(0);
        bank_first_pointer(&mut sequence, metadata.intern(&batch, 1)?);
    }
    sequence.push(0);
    metadata.intern(&sequence, 1)
}

fn validate_sprite_rows(rows: &[(u8, [u8; 512])]) -> Result<()> {
    let mut destinations = [false; 16];
    for (destination, _) in rows {
        ensure!((0x50..=0x5f).contains(destination));
        let destination = usize::from(*destination - 0x50);
        ensure!(!destinations[destination], "duplicate OBJ character row");
        destinations[destination] = true;
    }
    Ok(())
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

fn region_offset(address: u32, base_bank: u8, limit: usize) -> Result<usize> {
    let bank = (address >> 16) as u8;
    let within_bank = address as u16;
    ensure!(
        bank >= base_bank && within_bank >= 0x8000,
        "invalid asset pointer"
    );
    let offset = usize::from(bank - base_bank) * BANK_SIZE + usize::from(within_bank - 0x8000);
    ensure!(offset < limit, "asset pointer outside generated region");
    Ok(offset)
}

fn read_little_endian_pointer(bytes: &[u8], offset: usize) -> Result<u32> {
    ensure!(offset + 3 <= bytes.len(), "truncated asset pointer");
    Ok(u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        0,
    ]))
}

fn read_bank_first_pointer(bytes: &[u8], offset: &mut usize) -> Result<Option<u32>> {
    ensure!(*offset < bytes.len(), "unterminated asset list");
    let bank = bytes[*offset];
    *offset += 1;
    if bank == 0 {
        return Ok(None);
    }
    ensure!(*offset + 2 <= bytes.len(), "truncated asset pointer");
    let address = u16::from_le_bytes([bytes[*offset], bytes[*offset + 1]]);
    *offset += 2;
    Ok(Some(u32::from(bank) << 16 | u32::from(address)))
}

fn validate(
    bundle: &AssetBundle,
    areas: &[OverworldAreaAssets],
    credits_overworld: &[(u8, OverworldAreaAssets)],
    credits_cool_background: &OverworldAreaAssets,
    sprite_seed: &OverworldSpriteVariant,
) -> Result<()> {
    for (screen, expected) in areas.iter().enumerate() {
        for state in variant_test_states(&expected.sprite_variants) {
            validate_full_record(bundle, screen, state, expected)?;
        }
    }
    for (scene, expected) in credits_overworld {
        validate_full_record(bundle, CREDITS_FIRST_KEY + usize::from(*scene), 0, expected)?;
    }
    validate_full_record(
        bundle,
        CREDITS_COOL_BACKGROUND_KEY,
        0,
        credits_cool_background,
    )?;
    validate_sprite_seed(bundle, sprite_seed)?;

    // Validate every ordinary adjacency in both worlds. Large-area child
    // screens already resolve to their parent's assets in the importer.
    for world in [0, 0x40] {
        for y in 0..8 {
            for x in 0..8 {
                let destination = world + y * 8 + x;
                if x > 0 {
                    validate_transition(bundle, areas, destination - 1, destination, 3, 0x1f)?;
                }
                if x < 7 {
                    validate_transition(bundle, areas, destination + 1, destination, 6, 0x1f)?;
                }
                if y > 0 {
                    validate_transition(bundle, areas, destination - 8, destination, 9, 0x1b)?;
                }
                if y < 7 {
                    validate_transition(bundle, areas, destination + 8, destination, 12, 0x1b)?;
                }
            }
        }
    }
    Ok(())
}

fn validate_full_record(
    bundle: &AssetBundle,
    key: usize,
    game_state: u8,
    expected: &OverworldAreaAssets,
) -> Result<()> {
    let variant = select_expected_variant(expected, game_state);
    let record = resolve_record(bundle, key, game_state)?;
    let sequence = read_little_endian_pointer(&bundle.metadata, record)?;
    let mut sequence = region_offset(sequence, METADATA_BANK, bundle.metadata.len())?;
    let mut palettes = vec![[0; 32]; 6];
    let mut characters = vec![[0; 512]; 48];
    let mut palette_written = [false; 6];
    let mut character_written = [false; 48];

    while let Some(batch) = read_bank_first_pointer(&bundle.metadata, &mut sequence)? {
        apply_batch(
            bundle,
            batch,
            &mut palettes,
            &mut characters,
            &mut palette_written,
            &mut character_written,
        )?;
    }

    ensure!(palette_written.into_iter().all(|written| written));
    ensure!(character_written[..32].iter().all(|written| *written));
    let mut expected_characters = vec![[0; 512]; 48];
    expected_characters[..32].copy_from_slice(&expected.character_rows);
    let mut expected_written = [false; 48];
    expected_written[..32].fill(true);
    apply_expected_sprite_variant(
        variant,
        &mut expected_characters,
        Some(&mut expected_written),
    );
    ensure!(character_written == expected_written);
    ensure!(
        palettes == expected.palette_rows,
        "palette descriptor mismatch"
    );
    ensure!(
        characters == expected_characters,
        "character descriptor mismatch"
    );
    Ok(())
}

fn validate_transition(
    bundle: &AssetBundle,
    areas: &[OverworldAreaAssets],
    source: usize,
    destination: usize,
    record_offset: usize,
    last_frame: u8,
) -> Result<()> {
    for state in variant_test_states(&areas[destination].sprite_variants) {
        validate_transition_for_state(
            bundle,
            areas,
            source,
            destination,
            record_offset,
            last_frame,
            state,
        )?;
    }
    Ok(())
}

fn validate_transition_for_state(
    bundle: &AssetBundle,
    areas: &[OverworldAreaAssets],
    source: usize,
    destination: usize,
    record_offset: usize,
    last_frame: u8,
    game_state: u8,
) -> Result<()> {
    let record = resolve_record(bundle, destination, game_state)?;
    let schedule = read_little_endian_pointer(&bundle.metadata, record + record_offset)?;
    let mut cursor = region_offset(schedule, METADATA_BANK, bundle.metadata.len())?;
    let mut palettes = areas[source].palette_rows.clone();
    let mut expected_palettes = palettes.clone();
    let mut expected_palette_written = [false; 6];
    for (&load, rows) in areas[destination]
        .transition_palette_groups
        .iter()
        .zip([0..3, 3..6])
    {
        if load {
            expected_palettes[rows.clone()]
                .copy_from_slice(&areas[destination].palette_rows[rows.clone()]);
            expected_palette_written[rows].fill(true);
        }
    }
    let mut characters = vec![[0; 512]; 48];
    characters[..32].copy_from_slice(&areas[source].character_rows);
    apply_expected_sprite_variant(
        select_expected_variant(&areas[source], game_state),
        &mut characters,
        None,
    );
    let mut expected_characters = characters.clone();
    for (sheet, &load) in areas[destination].transition_sheets.iter().enumerate() {
        if load {
            let rows = sheet * 4..sheet * 4 + 4;
            expected_characters[rows.clone()]
                .copy_from_slice(&areas[destination].character_rows[rows]);
        }
    }
    let destination_variant = select_expected_variant(&areas[destination], game_state);
    apply_expected_sprite_variant(destination_variant, &mut expected_characters, None);
    let mut palette_written = [false; 6];
    let mut character_written = [false; 48];

    while let Some(batch) = read_bank_first_pointer(&bundle.metadata, &mut cursor)? {
        apply_batch(
            bundle,
            batch,
            &mut palettes,
            &mut characters,
            &mut palette_written,
            &mut character_written,
        )?;
    }

    let mut previous_frame = None;
    loop {
        ensure!(
            cursor < bundle.metadata.len(),
            "unterminated scroll schedule"
        );
        let frame = bundle.metadata[cursor];
        cursor += 1;
        if frame == 0xff {
            break;
        }
        ensure!(frame <= last_frame, "scroll frame outside transition");
        ensure!(previous_frame.is_none_or(|previous| frame > previous));
        previous_frame = Some(frame);
        let batch = read_bank_first_pointer(&bundle.metadata, &mut cursor)?
            .ok_or_else(|| anyhow::anyhow!("null scroll batch pointer"))?;
        apply_batch(
            bundle,
            batch,
            &mut palettes,
            &mut characters,
            &mut palette_written,
            &mut character_written,
        )?;
    }

    while let Some(batch) = read_bank_first_pointer(&bundle.metadata, &mut cursor)? {
        apply_batch(
            bundle,
            batch,
            &mut palettes,
            &mut characters,
            &mut palette_written,
            &mut character_written,
        )?;
    }

    ensure!(palette_written == expected_palette_written);
    for (sheet, &load) in areas[destination].transition_sheets.iter().enumerate() {
        ensure!(
            character_written[sheet * 4..sheet * 4 + 4]
                .iter()
                .all(|&written| written == load)
        );
    }
    let mut expected_sprite_written = [false; 48];
    apply_expected_sprite_variant(
        destination_variant,
        &mut vec![[0; 512]; 48],
        Some(&mut expected_sprite_written),
    );
    ensure!(character_written[32..] == expected_sprite_written[32..]);
    ensure!(palettes == expected_palettes);
    ensure!(characters == expected_characters);
    Ok(())
}

fn variant_test_states(variants: &[OverworldSpriteVariant]) -> Vec<u8> {
    let mut state = 0;
    variants
        .iter()
        .map(|variant| {
            let result = state;
            state = variant.max_game_state.saturating_add(1);
            result
        })
        .collect()
}

fn select_expected_variant(
    assets: &OverworldAreaAssets,
    game_state: u8,
) -> &OverworldSpriteVariant {
    assets
        .sprite_variants
        .iter()
        .find(|variant| game_state <= variant.max_game_state)
        .expect("sprite variants end at FF")
}

fn resolve_record(bundle: &AssetBundle, key: usize, game_state: u8) -> Result<usize> {
    let variants = read_little_endian_pointer(&bundle.metadata, key * 3)?;
    let mut cursor = region_offset(variants, METADATA_BANK, bundle.metadata.len())?;
    let mut previous = None;
    loop {
        ensure!(
            cursor + 4 <= bundle.metadata.len(),
            "truncated asset variant list"
        );
        let max_state = bundle.metadata[cursor];
        ensure!(previous.is_none_or(|value| max_state > value));
        let record = read_little_endian_pointer(&bundle.metadata, cursor + 1)?;
        if game_state <= max_state {
            return region_offset(record, METADATA_BANK, bundle.metadata.len());
        }
        ensure!(
            max_state != 0xff,
            "asset variant list does not cover game state"
        );
        previous = Some(max_state);
        cursor += 4;
    }
}

fn apply_expected_sprite_variant(
    variant: &OverworldSpriteVariant,
    characters: &mut [[u8; 512]],
    mut written: Option<&mut [bool; 48]>,
) {
    for (destination, row) in &variant.character_rows {
        let index = 32 + usize::from(*destination - 0x50);
        characters[index] = *row;
        if let Some(written) = written.as_deref_mut() {
            written[index] = true;
        }
    }
}

fn validate_sprite_seed(bundle: &AssetBundle, expected: &OverworldSpriteVariant) -> Result<()> {
    let record = resolve_record(bundle, SPRITE_SEED_KEY, 0)?;
    let sequence = read_little_endian_pointer(&bundle.metadata, record)?;
    let mut sequence = region_offset(sequence, METADATA_BANK, bundle.metadata.len())?;
    let mut palettes = vec![[0; 32]; 6];
    let mut characters = vec![[0; 512]; 48];
    let mut palette_written = [false; 6];
    let mut character_written = [false; 48];
    while let Some(batch) = read_bank_first_pointer(&bundle.metadata, &mut sequence)? {
        apply_batch(
            bundle,
            batch,
            &mut palettes,
            &mut characters,
            &mut palette_written,
            &mut character_written,
        )?;
    }
    let mut expected_characters = vec![[0; 512]; 48];
    let mut expected_written = [false; 48];
    apply_expected_sprite_variant(
        expected,
        &mut expected_characters,
        Some(&mut expected_written),
    );
    ensure!(!palette_written.into_iter().any(|written| written));
    ensure!(character_written == expected_written);
    ensure!(characters == expected_characters);
    Ok(())
}

fn apply_batch(
    bundle: &AssetBundle,
    batch: u32,
    palettes: &mut [[u8; 32]],
    characters: &mut [[u8; 512]],
    palette_written: &mut [bool; 6],
    character_written: &mut [bool; 48],
) -> Result<()> {
    let mut cursor = region_offset(batch, METADATA_BANK, bundle.metadata.len())?;
    let mut palette_count = 0;
    while let Some(source) = read_bank_first_pointer(&bundle.metadata, &mut cursor)? {
        palette_count += 1;
        ensure!(palette_count <= 6, "too many palette rows in one batch");
        ensure!(
            cursor < bundle.metadata.len(),
            "missing palette destination"
        );
        let destination = usize::from(bundle.metadata[cursor]);
        cursor += 1;
        ensure!((2..8).contains(&destination));
        let destination = destination - 2;
        ensure!(
            !palette_written[destination],
            "duplicate palette destination"
        );
        let source = region_offset(source, PAYLOAD_BANK, bundle.payload.len())?;
        ensure!(
            source + 32 <= bundle.payload.len(),
            "truncated palette payload"
        );
        palettes[destination].copy_from_slice(&bundle.payload[source..source + 32]);
        palette_written[destination] = true;
    }
    let mut character_count = 0;
    while let Some(source) = read_bank_first_pointer(&bundle.metadata, &mut cursor)? {
        character_count += 1;
        ensure!(
            character_count <= 8,
            "more than 4 KiB of characters in one batch"
        );
        ensure!(
            cursor < bundle.metadata.len(),
            "missing character destination"
        );
        let destination = usize::from(bundle.metadata[cursor]);
        cursor += 1;
        let destination = match destination {
            0..=31 => destination,
            0x50..=0x5f => 32 + destination - 0x50,
            _ => anyhow::bail!("character destination outside generated regions"),
        };
        ensure!(
            !character_written[destination],
            "duplicate character destination"
        );
        let source = region_offset(source, PAYLOAD_BANK, bundle.payload.len())?;
        ensure!(
            source + 512 <= bundle.payload.len(),
            "truncated character payload"
        );
        characters[destination].copy_from_slice(&bundle.payload[source..source + 512]);
        character_written[destination] = true;
    }
    Ok(())
}
