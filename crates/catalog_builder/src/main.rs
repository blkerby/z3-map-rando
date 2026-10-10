use anyhow::Result;
use catalog_builder::{logic, retiling};
use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[command(about = "Build randomizer catalogs from source data")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build the logic catalog from z3-json-data.
    Logic {
        source_directory: PathBuf,
        output_catalog: PathBuf,
    },
    /// Build a ROM-free retiling catalog from ALTTPRetiling.
    Retiling {
        retiling_project: PathBuf,
        output_catalog: PathBuf,
        #[arg(long, default_value = "data/tile_fingerprints.json")]
        tile_fingerprints: PathBuf,
    },
    /// Build both catalogs using sibling source repositories by default.
    All {
        #[arg(long, default_value = "../z3-json-data")]
        logic_source: PathBuf,
        #[arg(long, default_value = "../ALTTPRetiling")]
        retiling_source: PathBuf,
        #[arg(long, default_value = "build")]
        output_directory: PathBuf,
        #[arg(long, default_value = "data/tile_fingerprints.json")]
        tile_fingerprints: PathBuf,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::Logic {
            source_directory,
            output_catalog,
        } => logic::build_catalog(&source_directory, &output_catalog)?,
        Command::Retiling {
            retiling_project,
            output_catalog,
            tile_fingerprints,
        } => retiling::build_catalog(&retiling_project, &output_catalog, &tile_fingerprints)?,
        Command::All {
            logic_source,
            retiling_source,
            output_directory,
            tile_fingerprints,
        } => {
            fs::create_dir_all(&output_directory)?;
            logic::build_catalog(&logic_source, &output_directory.join("logic_catalog.bin"))?;
            retiling::build_catalog(
                &retiling_source,
                &output_directory.join("retiling_catalog.bin"),
                &tile_fingerprints,
            )?;
        }
    }
    Ok(())
}
