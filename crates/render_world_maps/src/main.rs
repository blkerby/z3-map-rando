use anyhow::{Context, Result};
use clap::Parser;
use generator::map::{MapPool, build_vanilla_rooms, select_borders};
use patcher::{import::Importer, retiling::compile_world_map};
use rand::{SeedableRng, rngs::StdRng};
use retiling_catalog::RetilingCatalog;
use seed_data::WorldLayout;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(about = "Render vanilla and pool-randomized Mode 7 maps without reachability checks")]
struct Args {
    /// Unheadered Japanese 1.0 vanilla ROM, used only to read graphics.
    #[arg(long)]
    rom: PathBuf,
    #[arg(long, default_value = "desert")]
    theme: String,
    /// Number of random layouts, in addition to the vanilla layout; 0 renders only vanilla.
    #[arg(long, default_value_t = 10)]
    count: usize,
    /// Render both Light and Dark World maps. By default, render only Light World maps.
    #[arg(long)]
    both_worlds: bool,
    /// Numeric RNG seed; chooses randomly when omitted.
    #[arg(long)]
    rng: Option<u64>,
    #[arg(long, default_value = "build/world-maps")]
    output_dir: PathBuf,
    #[arg(long, default_value = "build/retiling_catalog.bin")]
    retiling_catalog: PathBuf,
    #[arg(long, default_value = "build/logic_catalog.bin")]
    logic_catalog: PathBuf,
    #[arg(long, default_value = "maps/map_pool.bin")]
    map_pool: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let rom = fs::read(&args.rom).with_context(|| format!("reading ROM {}", args.rom.display()))?;
    patcher::verify_vanilla_rom(&rom)?;
    let importer = Importer::new(rom)?;
    let catalog = retiling_catalog::decode_catalog(
        File::open(&args.retiling_catalog)
            .with_context(|| format!("opening {}", args.retiling_catalog.display()))?,
    )?;
    let logic = logic_catalog::decode_catalog(
        File::open(&args.logic_catalog)
            .with_context(|| format!("opening {}", args.logic_catalog.display()))?,
    )?;
    let mut layout = WorldLayout {
        vanilla: true,
        rooms: build_vanilla_rooms(&logic, &args.theme),
        borders: Vec::new(),
    };
    fs::create_dir_all(&args.output_dir)?;
    write_maps(
        &catalog,
        &importer,
        &layout,
        &args.output_dir,
        "vanilla",
        args.both_worlds,
    )?;
    if args.count == 0 {
        return Ok(());
    }

    let pool_bytes = fs::read(&args.map_pool)
        .with_context(|| format!("reading map pool {}", args.map_pool.display()))?;
    let pool = MapPool::decode(&pool_bytes, &logic)?;
    let numeric_seed = args.rng.unwrap_or_else(rand::random);
    eprintln!("RNG seed: {numeric_seed}");
    let mut rng = StdRng::seed_from_u64(numeric_seed);
    layout.vanilla = false;
    for index in 1..=args.count {
        layout.rooms = pool.select_rooms(&logic, &args.theme, &mut rng)?;
        select_borders(&logic, &mut layout, &mut rng)?;
        write_maps(
            &catalog,
            &importer,
            &layout,
            &args.output_dir,
            &format!("random-{index:04}"),
            args.both_worlds,
        )?;
    }
    Ok(())
}

fn write_maps(
    catalog: &RetilingCatalog,
    importer: &Importer,
    layout: &WorldLayout,
    output_dir: &Path,
    name: &str,
    both_worlds: bool,
) -> Result<()> {
    let map = compile_world_map(catalog, importer, layout, both_worlds)?;
    for (png, suffix) in map.encode_pngs()?.iter().zip(["light", "dark"]) {
        let path = output_dir.join(format!("{name}-{suffix}.png"));
        fs::write(&path, png)?;
        eprintln!("Wrote {}", path.display());
    }
    Ok(())
}
