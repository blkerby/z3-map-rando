use catalog_builder::{cache::CatalogCache, config::BuildConfig};
use std::path::Path;

/// Update the catalogs the server loads at startup, from the same `catalog-build.toml` as the CLI.
fn main() -> anyhow::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut config = BuildConfig::read_config(root)?;
    config.resolve_paths(root);
    println!(
        "cargo:rerun-if-changed={}",
        root.join("catalog-build.toml").display()
    );
    let cache = CatalogCache::open_cache(config)?;
    for path in cache.collect_input_paths()? {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    cache.update_all()?;
    Ok(())
}
