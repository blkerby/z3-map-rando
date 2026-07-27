use anyhow::{Context, Result};
use clap::Parser;
use patcher::{
    Patcher, PcAddr, SnesAddr,
    import::{Importer, Tile16},
};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const VANILLA_ROM_SHA256: &str = "794e040b02c7591b59ad8843b51e7c619b88f87cddc6083a8e7a4027b96a2271";
const FLAT_MAPS_START: SnesAddr = SnesAddr(0x200000);
const FLAT_MAP_POINTERS_START: SnesAddr = SnesAddr(0x27e000);
const MAP16_DEFINITION_STARTS: [SnesAddr; 4] = [
    SnesAddr(0x288000),
    SnesAddr(0x298000),
    SnesAddr(0x2a8000),
    SnesAddr(0x2b8000),
];

#[derive(Parser)]
struct Args {
    input_rom: PathBuf,
    output_rom: PathBuf,
}

fn split_map16_definitions(tiles16: &[Tile16]) -> [Vec<u8>; 4] {
    std::array::from_fn(|quadrant| {
        tiles16
            .iter()
            .flat_map(|tile| tile[quadrant].to_vram_tilemap_word().to_le_bytes())
            .collect()
    })
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut rom = fs::read(&args.input_rom)
        .with_context(|| format!("failed to read {}", args.input_rom.display()))?;
    anyhow::ensure!(
        rom.len() == 1024 * 1024,
        "expected a 1 MiB vanilla ROM, got {} bytes",
        rom.len()
    );
    let digest = Sha256::digest(&rom)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    anyhow::ensure!(
        digest == VANILLA_ROM_SHA256,
        "input ROM SHA-256 mismatch: expected {VANILLA_ROM_SHA256}, got {digest}"
    );
    let mut importer = Importer::new(rom.clone())?;
    let flat_map16 = importer.flat_map16()?;
    let map16_definitions = split_map16_definitions(importer.tiles16()?);
    rom.resize(2 * 1024 * 1024, 0);

    let mut patcher = Patcher::default();
    let mut context = patcher.context("flat Map16 data");
    context.write(FLAT_MAPS_START.into(), flat_map16.maps)?;
    context.write(FLAT_MAP_POINTERS_START.into(), flat_map16.screen_pointers)?;

    let mut context = patcher.context("expanded Map16 definitions");
    for (start, definitions) in MAP16_DEFINITION_STARTS.into_iter().zip(map16_definitions) {
        context.write(start.into(), definitions)?;
    }

    // Zero out the obsolete Map16 definitions and Map32 data. This is only
    // temporary, to prove that they are no longer used.
    let mut context = patcher.context("zero obsolete map data");
    for range in [
        SnesAddr(0x0f8000)..SnesAddr(0x0ff4f0), // Map16 definitions
        SnesAddr(0x02f6b1)..SnesAddr(0x02fa71), // map pointer tables
        SnesAddr(0x038000)..SnesAddr(0x03e780), // top Tile32 definitions
        SnesAddr(0x048000)..SnesAddr(0x04e780), // bottom Tile32 definitions
        SnesAddr(0x0b8000)..SnesAddr(0x0bfe49), // compressed maps
        SnesAddr(0x0c8000)..SnesAddr(0x0cc0ab), // compressed maps
    ] {
        let start: PcAddr = range.start.into();
        let end: PcAddr = range.end.into();
        context.write(start, vec![0; (end.0 - start.0) as usize])?;
    }

    let patches = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../patches/ips");
    patcher.use_ips(&patches.join("expand.ips"))?;
    patcher.use_ips(&patches.join("flat_map16.ips"))?;
    patcher.use_ips(&patches.join("expand_map16.ips"))?;
    patcher.apply(&mut rom)?;

    fs::write(&args.output_rom, rom)
        .with_context(|| format!("failed to write {}", args.output_rom.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use patcher::import::Tile8;

    #[test]
    fn splits_map16_definitions_by_quadrant() {
        let tiles16 = [
            [0x0201, 0x0403, 0x0605, 0x0807].map(Tile8::from_vram_tilemap_word),
            [0x0A09, 0x0C0B, 0x0E0D, 0x100F].map(Tile8::from_vram_tilemap_word),
        ];

        assert_eq!(
            split_map16_definitions(&tiles16),
            [
                vec![1, 2, 9, 10],
                vec![3, 4, 11, 12],
                vec![5, 6, 13, 14],
                vec![7, 8, 15, 16],
            ]
        );
    }
}
