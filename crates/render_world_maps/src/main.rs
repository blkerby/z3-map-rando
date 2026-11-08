use anyhow::{Context, Result, ensure};
use clap::Parser;
use generator::map::{MapPool, select_borders};
use patcher::{import::Importer, retiling::compile_world_map};
use rand::{SeedableRng, rngs::StdRng};
use retiling_catalog::RetilingCatalog;
use seed_data::{AreaLayout, WorldLayout};
use std::{
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(about = "Render vanilla and pool-randomized Mode 7 maps without reachability checks")]
struct Args {
    /// Unheadered Japanese 1.0 vanilla ROM, used only to read graphics.
    #[arg(long)]
    rom: PathBuf,
    #[arg(long, default_value = "Desert")]
    theme: String,
    /// Number of random layouts, in addition to the vanilla layout; 0 renders only vanilla.
    #[arg(long, default_value_t = 10)]
    count: usize,
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
    ensure!(
        !areas.is_empty(),
        "theme {:?} has no overworld areas",
        args.theme
    );
    let mut layout = WorldLayout {
        areas,
        rooms: Vec::new(),
        borders: Vec::new(),
    };
    fs::create_dir_all(&args.output_dir)?;
    write_maps(&catalog, &importer, &layout, &args.output_dir, "vanilla")?;
    if args.count == 0 {
        return Ok(());
    }

    let logic = logic_catalog::decode_catalog(
        File::open(&args.logic_catalog)
            .with_context(|| format!("opening {}", args.logic_catalog.display()))?,
    )?;
    let pool_bytes = fs::read(&args.map_pool)
        .with_context(|| format!("reading map pool {}", args.map_pool.display()))?;
    let pool = MapPool::decode(&pool_bytes, &logic)?;
    let numeric_seed = args.rng.unwrap_or_else(rand::random);
    eprintln!("RNG seed: {numeric_seed}");
    let mut rng = StdRng::seed_from_u64(numeric_seed);
    for index in 1..=args.count {
        layout.rooms = pool.select_rooms(&logic, &mut rng)?;
        select_borders(&logic, &mut layout, &mut rng)?;
        write_maps(
            &catalog,
            &importer,
            &layout,
            &args.output_dir,
            &format!("random-{index:04}"),
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
) -> Result<()> {
    let map = compile_world_map(catalog, importer, layout)?;
    for (world, suffix) in ["light", "dark"].into_iter().enumerate() {
        let mut palette = Vec::new();
        for bytes in map.palettes[world * 256..(world + 1) * 256].chunks_exact(2) {
            let color = u16::from_le_bytes([bytes[0], bytes[1]]);
            for shift in [0, 5, 10] {
                let component = ((color >> shift) & 31) as u8;
                palette.push((component << 3) | (component >> 2));
            }
        }
        let mut pixels = vec![0; 512 * 512];
        for y in 0..512 {
            for x in 0..512 {
                let tx = x / 8;
                let ty = y / 8;
                // Compiled tilemaps store four 32x32 quadrants, not linear rows.
                let tile = map.tilemaps
                    [world * 4096 + (ty / 32 * 2 + tx / 32) * 1024 + ty % 32 * 32 + tx % 32];
                pixels[y * 512 + x] =
                    map.graphics[world * 0x4000 + usize::from(tile) * 64 + y % 8 * 8 + x % 8];
            }
        }
        let path = output_dir.join(format!("{name}-{suffix}.png"));
        let mut encoder = png::Encoder::new(BufWriter::new(File::create(&path)?), 512, 512);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_palette(palette);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&pixels)?;
        writer.finish()?;
        eprintln!("Wrote {}", path.display());
    }
    Ok(())
}
