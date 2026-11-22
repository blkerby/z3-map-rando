use anyhow::{Context, Result, ensure};
use clap::Parser;
use generator::metadata::{compute_catalog_hash, create_metadata};
use rand::{SeedableRng, rngs::StdRng};
use seed_data::{MapLayoutSetting, ResolvedSettings, SETTINGS_FORMAT_VERSION, SeedData};
use std::{
    borrow::Cow,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    sync::LazyLock,
};

#[cfg(feature = "bundle-catalogs")]
const BUNDLED_MAP_POOL: Option<&[u8]> = Some(include_bytes!(env!("Z3_MAP_POOL")));
#[cfg(not(feature = "bundle-catalogs"))]
const BUNDLED_MAP_POOL: Option<&[u8]> = None;

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

#[cfg(feature = "bundle-catalogs")]
const BUNDLED_PRESET_CATALOG: Option<&[u8]> = Some(include_bytes!(env!("Z3_PRESET_CATALOG")));
#[cfg(not(feature = "bundle-catalogs"))]
const BUNDLED_PRESET_CATALOG: Option<&[u8]> = None;

/// Project version, from the catalog builder's `version.txt`: bundled with the catalogs, or read
/// from `build/` like them. Unknown if the file has not been written yet.
static VERSION: LazyLock<String> = LazyLock::new(|| {
    #[cfg(feature = "bundle-catalogs")]
    return include_str!(env!("Z3_VERSION")).to_owned();
    #[cfg(not(feature = "bundle-catalogs"))]
    return fs::read_to_string("build/version.txt").unwrap_or_else(|_| "unknown".to_owned());
});

#[derive(Parser)]
#[command(
    name = "z3-map-rando",
    version = VERSION.as_str(),
    about = "Generate a randomized seed, patch a ROM, or both"
)]
struct Args {
    /// Settings JSON; uses the default root preset when neither --settings nor --preset is given.
    #[arg(long)]
    settings: Option<PathBuf>,
    /// Recover invalid submitted settings using the selected preset baseline.
    #[arg(long, requires = "settings")]
    recover_settings: bool,
    /// Root preset ID from the preset catalog.
    #[arg(long, conflicts_with_all = ["settings", "seed"])]
    preset: Option<String>,
    /// Existing seed JSON to patch; reuses its saved seed name.
    #[arg(long, requires = "rom", conflicts_with = "settings")]
    seed: Option<PathBuf>,
    /// Vanilla ROM to patch; omit to generate a seed file only.
    #[arg(long)]
    rom: Option<PathBuf>,
    /// Whole-game theme.
    #[arg(long, default_value = "desert")]
    theme: String,
    /// Map pool override; otherwise use bundled data or catalog-build.toml.
    #[arg(long)]
    map_pool: Option<PathBuf>,
    /// Numeric RNG seed; chooses randomly when omitted.
    #[arg(long)]
    rng: Option<u64>,
    /// Directory for default output seed and ROM paths.
    #[arg(long)]
    output_dir: Option<PathBuf>,
    /// Output seed file; overrides --output-dir. Defaults to z3-map-rando-<SEED-NAME>.json.
    #[arg(long)]
    output_seed: Option<PathBuf>,
    /// Output ROM file; overrides --output-dir. Defaults to z3-map-rando-<SEED-NAME>.sfc beside the input seed, or in the current directory.
    #[arg(long)]
    output_rom: Option<PathBuf>,
    /// Customization settings JSON (schema/settings/customization.json); uses the default customization preset when omitted.
    #[arg(long, requires = "rom")]
    customization_settings: Option<PathBuf>,
    /// When to load transition assets while patching.
    #[arg(long, value_enum, default_value = "scroll")]
    transition_asset_phase: TransitionAssetPhase,
    /// Logic catalog override; defaults to bundled data or build/logic_catalog.bin.
    #[arg(long)]
    logic_catalog: Option<PathBuf>,
    /// Full catalog or, when patching a saved seed, its filtered catalog. Defaults to bundled data or build/retiling_catalog.bin.
    #[arg(long)]
    retiling_catalog: Option<PathBuf>,
    /// Patch catalog override; defaults to bundled data or build/patch_catalog.bin.
    #[arg(long)]
    patch_catalog: Option<PathBuf>,
    /// Preset catalog override; defaults to bundled data or build/preset_catalog.bin.
    #[arg(long)]
    preset_catalog: Option<PathBuf>,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum TransitionAssetPhase {
    PreScroll,
    Scroll,
    PostScroll,
}

fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
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
        ensure!(
            retiling_catalog.resolved.is_none(),
            "generating a seed requires the full retiling catalog"
        );
        let seed = generate_seed(&args, &retiling_bytes, &retiling_catalog.palette_profiles)?;
        let output_path = args.output_seed.clone().unwrap_or_else(|| {
            args.output_dir
                .as_deref()
                .unwrap_or_else(|| Path::new(""))
                .join(format!("z3-map-rando-{}.json", seed.metadata.seed_name))
        });
        write_seed(&seed, &output_path)?;
        if args.rom.is_none() {
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
    retiling_bytes: &[u8],
    palette_profiles: &retiling_catalog::palette::PaletteProfiles,
) -> Result<SeedData> {
    let settings = load_settings(args)?;
    let logic_bytes = load_catalog(
        args.logic_catalog.as_deref(),
        "logic",
        "build/logic_catalog.bin",
        BUNDLED_LOGIC_CATALOG,
    )?;
    let logic_catalog = logic_catalog::decode_catalog(logic_bytes.as_ref())?;
    let map_pool = if settings.values.map_layout == MapLayoutSetting::Random {
        let bytes = if let Some(path) = &args.map_pool {
            Cow::Owned(
                fs::read(path).with_context(|| format!("reading map pool {}", path.display()))?,
            )
        } else if let Some(bytes) = BUNDLED_MAP_POOL {
            Cow::Borrowed(bytes)
        } else {
            let path = Path::new("catalog-build.toml");
            let pool_path = if path.exists() {
                let config: toml::Table = toml::from_str(&fs::read_to_string(path)?)?;
                PathBuf::from(
                    config
                        .get("map_pool")
                        .and_then(toml::Value::as_str)
                        .context("catalog-build.toml requires map_pool")?,
                )
            } else {
                PathBuf::from("maps/map_pool.bin")
            };
            Cow::Owned(
                fs::read(&pool_path)
                    .with_context(|| format!("reading map pool {}", pool_path.display()))?,
            )
        };
        Some(bytes)
    } else {
        None
    };
    let numeric_seed: u64 = args.rng.unwrap_or_else(rand::random);
    let metadata = create_metadata(
        &VERSION,
        numeric_seed,
        &compute_catalog_hash(&logic_bytes),
        &compute_catalog_hash(retiling_bytes),
    )?;
    let mut rng = StdRng::seed_from_u64(numeric_seed);
    generator::generate(
        settings,
        &logic_catalog,
        palette_profiles,
        &args.theme,
        map_pool.as_deref(),
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
    let customization = match &args.customization_settings {
        Some(path) => serde_json::from_slice(
            &fs::read(path)
                .with_context(|| format!("reading customization settings {}", path.display()))?,
        )
        .with_context(|| format!("parsing customization settings {}", path.display()))?,
        None => {
            let bytes = load_catalog(
                args.preset_catalog.as_deref(),
                "preset",
                "build/preset_catalog.bin",
                BUNDLED_PRESET_CATALOG,
            )?;
            let mut catalog = preset_catalog::decode_catalog(bytes.as_ref())?;
            catalog.customization.swap_remove(0).settings
        }
    };
    let settings = patcher::PatchSettings {
        customization,
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

fn load_settings(args: &Args) -> Result<ResolvedSettings> {
    let bytes = load_catalog(
        args.preset_catalog.as_deref(),
        "preset",
        "build/preset_catalog.bin",
        BUNDLED_PRESET_CATALOG,
    )?;
    let catalog = preset_catalog::decode_catalog(bytes.as_ref())?;
    let document = if let Some(path) = &args.settings {
        let file =
            File::open(path).with_context(|| format!("opening settings {}", path.display()))?;
        serde_json::from_reader(BufReader::new(file))
            .with_context(|| format!("reading settings {}", path.display()))?
    } else {
        let id = args.preset.as_deref().unwrap_or(&catalog.root[0].id);
        anyhow::ensure!(
            catalog.root.iter().any(|preset| preset.id == id),
            "unknown root preset {id}"
        );
        serde_json::json!({
            "format_version": SETTINGS_FORMAT_VERSION,
            "preset": id,
        })
    };
    let mode = if args.recover_settings {
        seed_data::SettingsMode::Recover
    } else {
        seed_data::SettingsMode::Strict
    };
    let report = catalog.process_settings(&document, mode)?;
    for notice in &report.notices {
        eprintln!("Settings: {notice}");
    }
    Ok(report.finish_processing()?)
}

fn write_seed(seed: &SeedData, output_path: &Path) -> Result<()> {
    let file = File::create(output_path)
        .with_context(|| format!("creating seed {}", output_path.display()))?;
    seed.write(file)
        .with_context(|| format!("writing seed {}", output_path.display()))?;
    eprintln!(
        "Wrote seed {} to {} (RNG {})",
        seed.metadata.seed_name,
        output_path.display(),
        seed.metadata.rng_seed,
    );
    Ok(())
}
