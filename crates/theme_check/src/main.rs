use anyhow::{Context, Result};
use clap::Parser;
use patcher::{PatchSettings, apply_retiling, finalize_rom, verify_vanilla_rom};
use seed_data::{AreaLayout, WorldLayout};
use std::{fs, path::PathBuf};

#[derive(Parser)]
struct Args {
    input_rom: PathBuf,
    output_rom: PathBuf,
    retiling_catalog: PathBuf,
    #[arg(long, default_value = "build/patch_catalog.bin")]
    patch_catalog: PathBuf,
    #[arg(long, default_value = "Base")]
    theme: String,
    #[arg(long, value_enum, default_value = "scroll")]
    transition_asset_phase: TransitionAssetPhase,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum TransitionAssetPhase {
    PreScroll,
    Scroll,
    PostScroll,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let catalog = retiling_catalog::decode_catalog(
        fs::File::open(&args.retiling_catalog)
            .with_context(|| format!("failed to read {}", args.retiling_catalog.display()))?,
    )?;
    let patches = patch_catalog::decode_catalog(
        fs::File::open(&args.patch_catalog)
            .with_context(|| format!("failed to read {}", args.patch_catalog.display()))?,
    )?;
    let mut rom = fs::read(&args.input_rom)
        .with_context(|| format!("failed to read {}", args.input_rom.display()))?;
    verify_vanilla_rom(&rom)?;
    let mut areas = Vec::new();
    for area in catalog.areas.values() {
        if let Some(vanilla_map_id) = area.vanilla_map_id {
            if area.themes.contains_key(&args.theme) {
                areas.push(AreaLayout {
                    vanilla_map_id,
                    theme_name: args.theme.clone(),
                });
            }
        }
    }
    let layout = WorldLayout { areas };
    let settings = PatchSettings {
        transition_asset_phase: match args.transition_asset_phase {
            TransitionAssetPhase::PreScroll => patcher::TransitionAssetPhase::PreScroll,
            TransitionAssetPhase::Scroll => patcher::TransitionAssetPhase::Scroll,
            TransitionAssetPhase::PostScroll => patcher::TransitionAssetPhase::PostScroll,
        },
    };
    let stats = apply_retiling(&mut rom, &layout, &catalog, &patches, &settings)?;
    finalize_rom(&mut rom);
    for warning in &stats.warnings {
        eprintln!("warning: {}: {warning}", args.theme);
    }
    eprintln!(
        "{}: {} screens, {} palettes, {} character slots, {} Map16 definitions",
        args.theme,
        stats.screen_count,
        stats.palette_count,
        stats.character_count,
        stats.map16_count,
    );
    eprintln!(
        "BG1: {} areas, {} variants ({} bytes: {}), {} background records; modes {:?}, {} parallax, {} drifting; fullest palette ${:02X} uses {}/12 half-slots",
        stats.bg1_area_count,
        stats.bg1_variant_names.len(),
        stats.bg1_bytes,
        stats.bg1_variant_names.join(", "),
        stats.background_record_count,
        stats.background_modes,
        stats.parallax_records,
        stats.drifting_records,
        stats.fullest_palette_screen,
        stats.fullest_palette_half_slots,
    );
    eprintln!(
        "overworld assets: {} bytes total ({} pointer table, {} data), {} unique blocks",
        stats.asset_pointer_table_bytes + stats.asset_data_bytes,
        stats.asset_pointer_table_bytes,
        stats.asset_data_bytes,
        stats.unique_asset_blocks,
    );

    fs::write(&args.output_rom, rom)
        .with_context(|| format!("failed to write {}", args.output_rom.display()))
}
