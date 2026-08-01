use anyhow::{Result, ensure};
use patcher::import::OverworldAreaAssets;
use std::collections::BTreeMap;

const BANK_SIZE: usize = 0x8000;
pub const METADATA_START: u32 = 0xa78000;
pub const PAYLOAD_START: u32 = 0xaa8000;
const METADATA_BANK: u8 = (METADATA_START >> 16) as u8;
const METADATA_SIZE: usize = 3 * BANK_SIZE;
const POINTER_TABLE_SIZE: usize = 256 * 3;
const PAYLOAD_BANK: u8 = (PAYLOAD_START >> 16) as u8;
const PAYLOAD_SIZE: usize = 14 * BANK_SIZE;

pub struct AssetBundle {
    pub metadata: Vec<u8>,
    pub payload: Vec<u8>,
    pub unique_payloads: usize,
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

pub fn build(areas: &[OverworldAreaAssets]) -> Result<AssetBundle> {
    ensure!(areas.len() == 0xa0, "expected 160 overworld asset sets");

    let mut metadata = Region::new(METADATA_BANK, METADATA_SIZE, POINTER_TABLE_SIZE);
    let mut payload = Region::new(PAYLOAD_BANK, PAYLOAD_SIZE, 0);
    let empty_record = metadata.intern(&[0; 18], 1)?;
    let mut area_records = Vec::with_capacity(areas.len());

    for area in areas {
        ensure!(area.palette_rows.len() == 6, "expected six BG palette rows");
        ensure!(
            area.character_rows.len() == 32,
            "expected 32 BG character rows"
        );

        let palette_sources = area
            .palette_rows
            .iter()
            .map(|row| payload.intern(row, 32))
            .collect::<Result<Vec<_>>>()?;
        let character_sources = area
            .character_rows
            .iter()
            .map(|row| payload.intern(row, 512))
            .collect::<Result<Vec<_>>>()?;

        let mut batch_sources = Vec::with_capacity(4);
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
            batch_sources.push(metadata.intern(&batch, 1)?);
        }

        let mut sequence = Vec::with_capacity(13);
        for source in batch_sources {
            bank_first_pointer(&mut sequence, source);
        }
        sequence.push(0);
        let sequence = metadata.intern(&sequence, 1)?;

        let mut record = vec![0; 18];
        record[..3].copy_from_slice(&little_endian_pointer(sequence));
        area_records.push(metadata.intern(&record, 1)?);
    }

    for screen in 0..256 {
        let record = area_records.get(screen).copied().unwrap_or(empty_record);
        let offset = screen * 3;
        metadata.bytes[offset..offset + 3].copy_from_slice(&little_endian_pointer(record));
    }

    let bundle = AssetBundle {
        unique_payloads: payload.offsets.len(),
        metadata: metadata.bytes,
        payload: payload.bytes,
    };
    validate(&bundle, areas)?;
    Ok(bundle)
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

fn validate(bundle: &AssetBundle, areas: &[OverworldAreaAssets]) -> Result<()> {
    for (screen, expected) in areas.iter().enumerate() {
        let record = read_little_endian_pointer(&bundle.metadata, screen * 3)?;
        let record = region_offset(record, METADATA_BANK, bundle.metadata.len())?;
        let sequence = read_little_endian_pointer(&bundle.metadata, record)?;
        let mut sequence = region_offset(sequence, METADATA_BANK, bundle.metadata.len())?;
        let mut palettes = vec![[0; 32]; 6];
        let mut characters = vec![[0; 512]; 32];
        let mut palette_written = [false; 6];
        let mut character_written = [false; 32];
        let mut batches = 0;

        while let Some(batch) = read_bank_first_pointer(&bundle.metadata, &mut sequence)? {
            batches += 1;
            let mut cursor = region_offset(batch, METADATA_BANK, bundle.metadata.len())?;
            while let Some(source) = read_bank_first_pointer(&bundle.metadata, &mut cursor)? {
                ensure!(
                    cursor < bundle.metadata.len(),
                    "missing palette destination"
                );
                let destination = usize::from(bundle.metadata[cursor]);
                cursor += 1;
                ensure!(
                    (2..8).contains(&destination),
                    "palette destination outside rows 2-7"
                );
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
            while let Some(source) = read_bank_first_pointer(&bundle.metadata, &mut cursor)? {
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
        }

        ensure!(batches == 4, "expected four full-reload batches");
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
    }
    Ok(())
}
