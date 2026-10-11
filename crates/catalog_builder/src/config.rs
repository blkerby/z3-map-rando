use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
pub struct BuildConfig {
    pub logic_source: PathBuf,
    pub retiling_source: PathBuf,
    pub asm: PathBuf,
    pub asar_source: PathBuf,
    pub asar_executable: Option<PathBuf>,
    pub output_directory: PathBuf,
    pub tile_fingerprints: PathBuf,
    pub compression_level: i32,
}

impl BuildConfig {
    pub fn read_config(root: &Path) -> Result<Self> {
        let path = root.join("catalog-build.toml");
        if !path.exists() {
            fs::copy(root.join("catalog-build.default.toml"), &path)?;
        }
        let text = fs::read_to_string(&path)?;
        let config =
            toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(config)
    }

    /// Resolve configuration paths from the repository root.
    pub fn resolve_paths(&mut self, root: &Path) {
        for path in [
            &mut self.logic_source,
            &mut self.retiling_source,
            &mut self.asm,
            &mut self.asar_source,
            &mut self.output_directory,
            &mut self.tile_fingerprints,
        ] {
            *path = root.join(&*path);
        }
        if let Some(path) = &mut self.asar_executable {
            *path = root.join(&*path);
        }
    }
}
