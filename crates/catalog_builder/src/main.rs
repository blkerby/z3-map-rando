use anyhow::Result;
use catalog_builder::{cache::CatalogCache, config::BuildConfig};
use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(about = "Build randomizer catalogs from source data")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build the logic catalog from z3-json-data.
    Logic,
    /// Build a retiling catalog from ALTTPRetiling.
    Retiling,
    /// Build the patch catalog from ASM and z3-json-data.
    Patches,
    /// Update all three catalogs, reusing unchanged outputs.
    All,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut config = BuildConfig::read_config(root)?;
    config.resolve_paths(root);
    let cache = CatalogCache::open_cache(config)?;
    match args.command {
        Command::Logic => cache.update_logic()?,
        Command::Retiling => cache.update_retiling()?,
        Command::Patches => cache.update_patches()?,
        Command::All => cache.update_all()?,
    }
    Ok(())
}
