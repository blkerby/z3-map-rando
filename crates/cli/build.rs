#[cfg(not(feature = "bundle-catalogs"))]
fn main() {}

#[cfg(feature = "bundle-catalogs")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use catalog_builder::{cache::CatalogCache, config::BuildConfig};
    use std::path::Path;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut config = BuildConfig::read_config(root)?;
    config.resolve_paths(root);
    let output_directory = config.output_directory.clone();
    let cache = CatalogCache::open_cache(config)?;
    println!(
        "cargo:rerun-if-changed={}",
        root.join("catalog-build.toml").display()
    );
    for path in cache.collect_input_paths()? {
        println!("cargo:rerun-if-changed={}", path.display());
    }
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
