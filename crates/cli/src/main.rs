use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use clap::Parser;
use rand::{Rng, SeedableRng, rngs::StdRng};
use seed_data::{SeedData, SeedMetadata, Settings};
use sha2::{Digest, Sha256};
use std::{
    fmt::Write as _,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DEFAULT_SETTINGS: &str = include_str!("../../../data/settings.json");

#[derive(Parser)]
#[command(
    name = "z3-map-rando",
    version = generator::VERSION,
    about = "Generate a randomized seed"
)]
struct Args {
    /// Settings JSON; uses bundled defaults when omitted.
    #[arg(long)]
    settings: Option<PathBuf>,
    /// Whole-game theme.
    #[arg(long, default_value = "Base")]
    theme: String,
    /// Numeric RNG seed; chooses randomly when omitted.
    #[arg(long)]
    rng: Option<u64>,
    /// Output seed file; defaults to z3-map-rando-<SEED-NAME>.json.zst.
    #[arg(long)]
    output_seed: Option<PathBuf>,
    /// Zstd compression level for the output seed.
    #[arg(long, default_value_t = 3, allow_negative_numbers = true)]
    compression_level: i32,
    /// Logic catalog used for generation.
    #[arg(long, default_value = "build/logic_catalog.bin")]
    logic_catalog: PathBuf,
    /// Retiling catalog used for theme selection.
    #[arg(long, default_value = "build/retiling_catalog.bin")]
    retiling_catalog: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let seed = generate_seed(&args)?;
    let output_path = args.output_seed.unwrap_or_else(|| {
        PathBuf::from(format!("z3-map-rando-{}.json.zst", seed.metadata.seed_name))
    });
    write_seed(&seed, &output_path, args.compression_level)
}

fn generate_seed(args: &Args) -> Result<SeedData> {
    let settings = load_settings(args.settings.as_deref())?;
    let logic_bytes = fs::read(&args.logic_catalog)
        .with_context(|| format!("reading logic catalog {}", args.logic_catalog.display()))?;
    let logic_catalog = logic_catalog::decode_catalog(logic_bytes.as_slice())?;
    let retiling_bytes = fs::read(&args.retiling_catalog).with_context(|| {
        format!(
            "reading retiling catalog {}",
            args.retiling_catalog.display()
        )
    })?;
    let retiling_catalog = retiling_catalog::decode_catalog(retiling_bytes.as_slice())?;
    let numeric_seed: u64 = args.rng.unwrap_or_else(rand::random);
    let metadata = create_metadata(numeric_seed, &logic_bytes, &retiling_bytes)?;
    let mut rng = StdRng::seed_from_u64(numeric_seed);
    generator::generate(
        settings,
        &logic_catalog,
        &retiling_catalog,
        &args.theme,
        metadata,
        &mut rng,
    )
}

fn load_settings(path: Option<&Path>) -> Result<Settings> {
    if let Some(path) = path {
        let file =
            File::open(path).with_context(|| format!("opening settings {}", path.display()))?;
        serde_json::from_reader(BufReader::new(file))
            .with_context(|| format!("reading settings {}", path.display()))
    } else {
        Ok(serde_json::from_str(DEFAULT_SETTINGS)?)
    }
}

fn create_metadata(
    numeric_seed: u64,
    logic_bytes: &[u8],
    retiling_bytes: &[u8],
) -> Result<SeedMetadata> {
    let now = SystemTime::now();
    let timestamp = now.duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(SeedMetadata {
        seed_name: generate_seed_name(numeric_seed, timestamp),
        generator_version: generator::VERSION.to_owned(),
        generator_commit: None,
        logic_catalog_hash: compute_catalog_hash(logic_bytes),
        retiling_catalog_hash: compute_catalog_hash(retiling_bytes),
        rng_seed: numeric_seed.to_string(),
        generated_at: DateTime::<Utc>::from(now).to_rfc3339_opts(SecondsFormat::Nanos, true),
    })
}

fn generate_seed_name(numeric_seed: u64, timestamp: u128) -> String {
    let mut naming_seed = [0; 32];
    naming_seed[..8].copy_from_slice(&numeric_seed.to_le_bytes());
    naming_seed[8..24].copy_from_slice(&timestamp.to_le_bytes());
    let mut naming_rng = StdRng::from_seed(naming_seed);
    // Omit vowels and characters resembling them to reduce the chance of words.
    let alphabet = b"256789BCDFGHJKLMNPQRSTVWXYZbcdfghjkmnpqrstvwxyz";
    let mut seed_name = String::new();
    for _ in 0..9 {
        seed_name.push(alphabet[naming_rng.random_range(0..alphabet.len())] as char);
    }
    seed_name
}

fn write_seed(seed: &SeedData, output_path: &Path, compression_level: i32) -> Result<()> {
    let file = File::create(output_path)
        .with_context(|| format!("creating seed {}", output_path.display()))?;
    seed.write(file, compression_level)
        .with_context(|| format!("writing seed {}", output_path.display()))?;
    eprintln!(
        "Wrote seed {} to {} (RNG {}, {} item placements, {} retiled areas)",
        seed.metadata.seed_name,
        output_path.display(),
        seed.metadata.rng_seed,
        seed.item_placements.len(),
        seed.world_layout.areas.len(),
    );
    Ok(())
}

fn compute_catalog_hash(bytes: &[u8]) -> String {
    let mut hash = String::new();
    for byte in Sha256::digest(bytes) {
        write!(&mut hash, "{byte:02x}").unwrap();
    }
    hash
}
