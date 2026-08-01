use anyhow::{Result, ensure};
use patcher::import::OverworldAreaAssets;
use std::collections::BTreeMap;

const BANK_SIZE: usize = 0x8000;
pub const METADATA_START: u32 = 0xa78000;
pub const PAYLOAD_START: u32 = 0xaa8000;
const METADATA_BANK: u8 = (METADATA_START >> 16) as u8;
const METADATA_SIZE: usize = 3 * BANK_SIZE;
const POINTER_TABLE_SIZE: usize = 256 * 3;
pub const CREDITS_COOL_BACKGROUND_KEY: usize = 0xff;
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
    credits_cool_background: &OverworldAreaAssets,
    transition_phase: TransitionAssetPhase,
) -> Result<AssetBundle> {
    ensure!(areas.len() == 0xa0, "expected 160 overworld asset sets");

    let mut metadata = Region::new(METADATA_BANK, METADATA_SIZE, POINTER_TABLE_SIZE);
    let mut payload = Region::new(PAYLOAD_BANK, PAYLOAD_SIZE, 0);
    let empty_record = metadata.intern(&[0; 18], 1)?;
    let mut area_records = Vec::with_capacity(areas.len());

    for area in areas {
        let (sequence, palette_sources, character_sources) =
            intern_full_sequence(&mut metadata, &mut payload, area)?;

        // Vanilla scrolling transitions retain palette groups selected by
        // $FF and character sheets selected by zero.
        let transition_palette_rows = area
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

        let transition_rows = area
            .transition_sheets
            .iter()
            .enumerate()
            .filter(|(_, load)| **load)
            .flat_map(|(sheet, _)| sheet * 4..sheet * 4 + 4)
            .collect::<Vec<_>>();

        let mut transition_batches = Vec::new();
        for rows in transition_rows.chunks(8) {
            let mut batch = vec![0];
            for &row in rows {
                descriptor(&mut batch, character_sources[row], u8::try_from(row)?);
            }
            batch.push(0);
            transition_batches.push(metadata.intern(&batch, 1)?);
        }

        let transition_batches = palette_batch
            .into_iter()
            .chain(transition_batches)
            .collect::<Vec<_>>();
        let mut schedule = Vec::with_capacity(3 + transition_batches.len() * 4);
        if matches!(transition_phase, TransitionAssetPhase::PreScroll) {
            for &source in &transition_batches {
                bank_first_pointer(&mut schedule, source);
            }
        }
        schedule.push(0);
        if matches!(transition_phase, TransitionAssetPhase::Scroll) {
            for (frame, &source) in transition_batches.iter().enumerate() {
                schedule.push(u8::try_from(frame)?);
                bank_first_pointer(&mut schedule, source);
            }
        }
        schedule.push(0xff);
        if matches!(transition_phase, TransitionAssetPhase::PostScroll) {
            for source in transition_batches {
                bank_first_pointer(&mut schedule, source);
            }
        }
        schedule.push(0);
        let schedule = metadata.intern(&schedule, 1)?;

        let mut record = vec![0; 18];
        record[..3].copy_from_slice(&little_endian_pointer(sequence));
        for offset in [3, 6, 9, 12] {
            record[offset..offset + 3].copy_from_slice(&little_endian_pointer(schedule));
        }
        area_records.push(metadata.intern(&record, 1)?);
    }

    let (credits_sequence, _, _) =
        intern_full_sequence(&mut metadata, &mut payload, credits_cool_background)?;
    let mut credits_record_bytes = vec![0; 18];
    credits_record_bytes[..3].copy_from_slice(&little_endian_pointer(credits_sequence));
    let credits_record = metadata.intern(&credits_record_bytes, 1)?;

    for screen in 0..256 {
        let record = if screen == CREDITS_COOL_BACKGROUND_KEY {
            credits_record
        } else {
            area_records.get(screen).copied().unwrap_or(empty_record)
        };
        let offset = screen * 3;
        metadata.bytes[offset..offset + 3].copy_from_slice(&little_endian_pointer(record));
    }

    let bundle = AssetBundle {
        unique_payloads: payload.offsets.len(),
        metadata: metadata.bytes,
        payload: payload.bytes,
    };
    validate(&bundle, areas, credits_cool_background)?;
    Ok(bundle)
}

fn intern_full_sequence(
    metadata: &mut Region,
    payload: &mut Region,
    assets: &OverworldAreaAssets,
) -> Result<(u32, Vec<u32>, Vec<u32>)> {
    ensure!(
        assets.palette_rows.len() == 6,
        "expected six BG palette rows"
    );
    ensure!(
        assets.character_rows.len() == 32,
        "expected 32 BG character rows"
    );

    let palette_sources = assets
        .palette_rows
        .iter()
        .map(|row| payload.intern(row, 32))
        .collect::<Result<Vec<_>>>()?;
    let character_sources = assets
        .character_rows
        .iter()
        .map(|row| payload.intern(row, 512))
        .collect::<Result<Vec<_>>>()?;

    let mut sequence = Vec::with_capacity(13);
    for batch_index in 0..4 {
        let mut batch = Vec::new();
        if batch_index == 0 {
            for (row, &source) in palette_sources.iter().enumerate() {
                descriptor(&mut batch, source, u8::try_from(row + 2)?);
            }
        }
        batch.push(0);
        for row in batch_index * 8..batch_index * 8 + 8 {
            descriptor(&mut batch, character_sources[row], u8::try_from(row)?);
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
    credits_cool_background: &OverworldAreaAssets,
) -> Result<()> {
    for (screen, expected) in areas.iter().enumerate() {
        validate_full_record(bundle, screen, expected)?;
    }
    validate_full_record(bundle, CREDITS_COOL_BACKGROUND_KEY, credits_cool_background)?;

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
    expected: &OverworldAreaAssets,
) -> Result<()> {
    let record = read_little_endian_pointer(&bundle.metadata, key * 3)?;
    let record = region_offset(record, METADATA_BANK, bundle.metadata.len())?;
    let sequence = read_little_endian_pointer(&bundle.metadata, record)?;
    let mut sequence = region_offset(sequence, METADATA_BANK, bundle.metadata.len())?;
    let mut palettes = vec![[0; 32]; 6];
    let mut characters = vec![[0; 512]; 32];
    let mut palette_written = [false; 6];
    let mut character_written = [false; 32];

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
    ensure!(character_written.into_iter().all(|written| written));
    ensure!(
        palettes == expected.palette_rows,
        "palette descriptor mismatch"
    );
    ensure!(
        characters == expected.character_rows,
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
    let record = read_little_endian_pointer(&bundle.metadata, destination * 3)?;
    let record = region_offset(record, METADATA_BANK, bundle.metadata.len())?;
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
    let mut characters = areas[source].character_rows.clone();
    let mut expected_characters = characters.clone();
    for (sheet, &load) in areas[destination].transition_sheets.iter().enumerate() {
        if load {
            let rows = sheet * 4..sheet * 4 + 4;
            expected_characters[rows.clone()]
                .copy_from_slice(&areas[destination].character_rows[rows]);
        }
    }
    let mut palette_written = [false; 6];
    let mut character_written = [false; 32];

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
    ensure!(palettes == expected_palettes);
    ensure!(characters == expected_characters);
    Ok(())
}

fn apply_batch(
    bundle: &AssetBundle,
    batch: u32,
    palettes: &mut [[u8; 32]],
    characters: &mut [[u8; 512]],
    palette_written: &mut [bool; 6],
    character_written: &mut [bool; 32],
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
        ensure!(destination < 32, "character destination outside rows 0-31");
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
