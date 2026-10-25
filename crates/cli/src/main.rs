use anyhow::{Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use clap::Parser;
use rand::{Rng, SeedableRng, rngs::StdRng};
use seed_data::{SeedData, SeedMetadata, Settings};
use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    fmt::Write as _,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DEFAULT_SETTINGS: &str = include_str!("../../../data/settings.json");

#[cfg(feature = "bundle-catalogs")]
const BUNDLED_LOGIC_CATALOG: Option<&[u8]> = Some(include_bytes!(env!("Z3_LOGIC_CATALOG")));
#[cfg(not(feature = "bundle-catalogs"))]
const BUNDLED_LOGIC_CATALOG: Option<&[u8]> = None;

#[cfg(feature = "bundle-catalogs")]
const BUNDLED_RETILING_CATALOG: Option<&[u8]> = Some(include_bytes!(env!("Z3_RETILING_CATALOG")));
#[cfg(not(feature = "bundle-catalogs"))]
const BUNDLED_RETILING_CATALOG: Option<&[u8]> = None;

#[cfg(feature = "bundle-catalogs")]
const BUNDLED_PATCH_CATALOG: Option<&[u8]> = Some(include_bytes!(env!("Z3_PATCH_CATALOG")));
#[cfg(not(feature = "bundle-catalogs"))]
const BUNDLED_PATCH_CATALOG: Option<&[u8]> = None;

#[derive(Parser)]
#[command(
    name = "z3-map-rando",
    version = common_types::VERSION,
    about = "Generate a randomized seed, patch a ROM, or both"
)]
struct Args {
    /// Settings JSON; uses bundled defaults when omitted.
    #[arg(long)]
    settings: Option<PathBuf>,
    /// Existing compressed seed to patch; reuses its saved seed name.
    #[arg(long, requires = "rom", conflicts_with = "settings")]
    seed: Option<PathBuf>,
    /// Vanilla ROM to patch; omit to generate a seed file only.
    #[arg(long)]
    rom: Option<PathBuf>,
    /// Whole-game theme.
    #[arg(long, default_value = "Base")]
    theme: String,
    /// Numeric RNG seed; chooses randomly when omitted.
    #[arg(long)]
    rng: Option<u64>,
    /// Directory for default output seed and ROM paths.
    #[arg(long)]
    output_dir: Option<PathBuf>,
    /// Output seed file; overrides --output-dir. Defaults to z3-map-rando-<SEED-NAME>.json.zst.
    #[arg(long)]
    output_seed: Option<PathBuf>,
    /// Output ROM file; overrides --output-dir. Defaults to z3-map-rando-<SEED-NAME>.sfc beside the input seed, or in the current directory.
    #[arg(long)]
    output_rom: Option<PathBuf>,
    /// When to load transition assets while patching.
    #[arg(long, value_enum, default_value = "scroll")]
    transition_asset_phase: TransitionAssetPhase,
    /// Zstd compression level for the output seed.
    #[arg(long, default_value_t = 3, allow_negative_numbers = true)]
    compression_level: i32,
    /// Logic catalog override; defaults to bundled data or build/logic_catalog.bin.
    #[arg(long)]
    logic_catalog: Option<PathBuf>,
    /// Retiling catalog override; defaults to bundled data or build/retiling_catalog.bin.
    #[arg(long)]
    retiling_catalog: Option<PathBuf>,
    /// Patch catalog override; defaults to bundled data or build/patch_catalog.bin.
    #[arg(long)]
    patch_catalog: Option<PathBuf>,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum TransitionAssetPhase {
    PreScroll,
    Scroll,
    PostScroll,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let rom = if let Some(path) = &args.rom {
        let rom = fs::read(path).with_context(|| format!("reading ROM {}", path.display()))?;
        patcher::verify_vanilla_rom(&rom)
            .with_context(|| format!("verifying ROM {}", path.display()))?;
        Some(rom)
    } else {
        None
    };
    let retiling_bytes = load_catalog(
        args.retiling_catalog.as_deref(),
        "retiling",
        "build/retiling_catalog.bin",
        BUNDLED_RETILING_CATALOG,
    )?;
    let retiling_catalog = retiling_catalog::decode_catalog(retiling_bytes.as_ref())?;
    let seed = if let Some(path) = &args.seed {
        let file = File::open(path).with_context(|| format!("opening seed {}", path.display()))?;
        SeedData::read(file).with_context(|| format!("reading seed {}", path.display()))?
    } else {
        let seed = generate_seed(&args, &retiling_catalog, &retiling_bytes)?;
        let output_path = args.output_seed.clone().unwrap_or_else(|| {
            args.output_dir
                .as_deref()
                .unwrap_or_else(|| Path::new(""))
                .join(format!("z3-map-rando-{}.json.zst", seed.metadata.seed_name))
        });
        write_seed(&seed, &output_path, args.compression_level)?;
        if args.settings.is_none() && args.rom.is_none() {
            eprintln!(
                concat!(
                    "\nTip: To patch this seed, run:\n\n",
                    "  z3-map-rando --seed {} --rom path/to/vanilla.sfc\n\n",
                    "You can also add --rom path/to/vanilla.sfc to the initial command ",
                    "to generate and patch in one step.",
                ),
                output_path.display(),
            );
        }
        seed
    };
    if let Some(rom) = &rom {
        patch_seed(&args, &seed, &retiling_catalog, rom)?;
    }
    Ok(())
}

fn generate_seed(
    args: &Args,
    retiling_catalog: &retiling_catalog::RetilingCatalog,
    retiling_bytes: &[u8],
) -> Result<SeedData> {
    let settings = load_settings(args.settings.as_deref())?;
    let logic_bytes = load_catalog(
        args.logic_catalog.as_deref(),
        "logic",
        "build/logic_catalog.bin",
        BUNDLED_LOGIC_CATALOG,
    )?;
    let logic_catalog = logic_catalog::decode_catalog(logic_bytes.as_ref())?;
    let numeric_seed: u64 = args.rng.unwrap_or_else(rand::random);
    let metadata = create_metadata(numeric_seed, &logic_bytes, retiling_bytes)?;
    let mut rng = StdRng::seed_from_u64(numeric_seed);
    generator::generate(
        settings,
        &logic_catalog,
        retiling_catalog,
        &args.theme,
        metadata,
        &mut rng,
    )
}

fn patch_seed(
    args: &Args,
    seed: &SeedData,
    retiling_catalog: &retiling_catalog::RetilingCatalog,
    rom: &[u8],
) -> Result<()> {
    let patch_bytes = load_catalog(
        args.patch_catalog.as_deref(),
        "patch",
        "build/patch_catalog.bin",
        BUNDLED_PATCH_CATALOG,
    )?;
    let patch_catalog = patch_catalog::decode_catalog(patch_bytes.as_ref())?;
    let settings = patcher::PatchSettings {
        transition_asset_phase: match args.transition_asset_phase {
            TransitionAssetPhase::PreScroll => patcher::TransitionAssetPhase::PreScroll,
            TransitionAssetPhase::Scroll => patcher::TransitionAssetPhase::Scroll,
            TransitionAssetPhase::PostScroll => patcher::TransitionAssetPhase::PostScroll,
        },
    };
    let output = patcher::patch_rom(rom, seed, retiling_catalog, &patch_catalog, &settings)?;
    let output_path = args.output_rom.clone().unwrap_or_else(|| {
        let filename = format!("z3-map-rando-{}.sfc", seed.metadata.seed_name);
        if let Some(dir) = &args.output_dir {
            dir.join(filename)
        } else if let Some(path) = &args.seed {
            path.with_file_name(filename)
        } else {
            PathBuf::from(filename)
        }
    });
    fs::write(&output_path, output)
        .with_context(|| format!("writing ROM {}", output_path.display()))?;
    eprintln!(
        "Wrote ROM {} to {}",
        seed.metadata.seed_name,
        output_path.display()
    );
    Ok(())
}

fn load_catalog(
    path: Option<&Path>,
    kind: &str,
    default_path: &str,
    bundled: Option<&'static [u8]>,
) -> Result<Cow<'static, [u8]>> {
    let path = if let Some(path) = path {
        path
    } else if let Some(bytes) = bundled {
        return Ok(Cow::Borrowed(bytes));
    } else {
        Path::new(default_path)
    };
    let bytes =
        fs::read(path).with_context(|| format!("reading {kind} catalog {}", path.display()))?;
    Ok(Cow::Owned(bytes))
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
        generator_version: common_types::VERSION.to_owned(),
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
        "Wrote seed {} to {} (RNG {})",
        seed.metadata.seed_name,
        output_path.display(),
        seed.metadata.rng_seed,
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
