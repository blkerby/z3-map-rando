#[cfg(not(feature = "bundle-catalogs"))]
fn main() {}

#[cfg(feature = "bundle-catalogs")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use catalog_builder::{cache::CatalogCache, config::BuildConfig};
    use std::path::Path;

    // This path is deliberately never created. Let the existing catalog cache
    // check for changes on every bundled build instead of duplicating tracking.
    println!("cargo:rerun-if-changed=always-update-bundled-catalogs");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut config = BuildConfig::read_config(root)?;
    config.resolve_paths(root);
    let output_directory = config.output_directory.clone();
    let cache = CatalogCache::open_cache(config)?;
    cache.update_all()?;
    for (name, filename) in [
        ("Z3_LOGIC_CATALOG", "logic_catalog.bin"),
        ("Z3_RETILING_CATALOG", "retiling_catalog.bin"),
        ("Z3_PATCH_CATALOG", "patch_catalog.bin"),
    ] {
        println!(
            "cargo:rustc-env={name}={}",
            output_directory.join(filename).display()
        );
    }
    Ok(())
}
