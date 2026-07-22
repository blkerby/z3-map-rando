use anyhow::{Context, Result};
use clap::Parser;
use patcher::{Patcher, PcAddr, SnesAddr, import::Importer};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const VANILLA_ROM_SHA256: &str = "794e040b02c7591b59ad8843b51e7c619b88f87cddc6083a8e7a4027b96a2271";
const FLAT_MAPS_START: SnesAddr = SnesAddr(0x200000);
const FLAT_MAP_POINTERS_START: SnesAddr = SnesAddr(0x270000);

#[derive(Parser)]
struct Args {
    input_rom: PathBuf,
    output_rom: PathBuf,
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
    let flat_map16 = Importer::new(rom.clone())?.flat_map16()?;
    rom.resize(2 * 1024 * 1024, 0);

    let mut patcher = Patcher::default();
    let mut context = patcher.context("flat Map16 data");
    context.write(FLAT_MAPS_START.into(), flat_map16.maps)?;
    context.write(FLAT_MAP_POINTERS_START.into(), flat_map16.screen_pointers)?;

    let mut context = patcher.context("zero obsolete Map32 data");
    for range in [
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
    patcher.apply(&mut rom)?;

    fs::write(&args.output_rom, rom)
        .with_context(|| format!("failed to write {}", args.output_rom.display()))
}
