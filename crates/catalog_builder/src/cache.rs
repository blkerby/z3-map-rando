use crate::{config::BuildConfig, logic, patches, retiling};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};

include!(concat!(env!("OUT_DIR"), "/code_inputs.rs"));

/// Hold the shared artifact lock throughout catalog updates.
pub struct CatalogCache {
    config: BuildConfig,
    shared_fingerprint: Sha256,
    _lock: File,
}

impl CatalogCache {
    pub fn open_cache(config: BuildConfig) -> Result<Self> {
        fs::create_dir_all(&config.output_directory)?;
        let lock = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(config.output_directory.join("cache.lock"))?;
        lock.lock()?;
        let mut shared_fingerprint = Sha256::new();
        for &(path, bytes) in SHARED_CODE_INPUTS {
            hash_input(&mut shared_fingerprint, path.as_bytes());
            hash_input(&mut shared_fingerprint, bytes);
        }
        hash_input(
            &mut shared_fingerprint,
            &config.compression_level.to_le_bytes(),
        );
        Ok(Self {
            config,
            shared_fingerprint,
            _lock: lock,
        })
    }

    fn create_fingerprint(&self, inputs: &[(&str, &[u8])]) -> Sha256 {
        let mut fingerprint = self.shared_fingerprint.clone();
        for &(path, bytes) in inputs {
            hash_input(&mut fingerprint, path.as_bytes());
            hash_input(&mut fingerprint, bytes);
        }
        fingerprint
    }

    /// List external inputs for Cargo; fingerprints still decide which catalogs rebuild.
    // shortcut: after manually deleting catalog outputs, clean the CLI package;
    // watch outputs too only if their generation can avoid timestamp-driven rebuilds.
    pub fn collect_input_paths(&self) -> Result<Vec<PathBuf>> {
        let mut paths = vec![
            self.config.logic_source.clone(),
            self.config.retiling_source.clone(),
            self.config.asm.clone(),
            self.config.tile_fingerprints.clone(),
        ];
        if let Some(path) = &self.config.asar_executable {
            paths.push(path.clone());
        } else {
            paths.push(self.config.asar_source.join("src"));
        }
        for source in [&self.config.logic_source, &self.config.retiling_source] {
            if source.join(".git").exists() {
                // Submodule Git metadata lives outside the watched source directory.
                let output = Command::new("git")
                    .arg("-C")
                    .arg(source)
                    .args([
                        "rev-parse",
                        "--path-format=absolute",
                        "--git-path",
                        "HEAD",
                        "--git-path",
                        "refs",
                        "--git-path",
                        "packed-refs",
                    ])
                    .output()?;
                for path in String::from_utf8_lossy(&output.stdout).lines() {
                    // Missing optional Git files would force Cargo to rerun every time.
                    if Path::new(path).exists() {
                        paths.push(PathBuf::from(path));
                    }
                }
            }
        }
        Ok(paths)
    }

    pub fn update_logic(&self) -> Result<()> {
        let source = &self.config.logic_source;
        let output = self.config.output_directory.join("logic_catalog.bin");
        let mut fingerprint = self.create_fingerprint(LOGIC_CODE_INPUTS);
        let clean = hash_repository(&mut fingerprint, source)?;
        update_catalog(&output, fingerprint, clean, || {
            logic::build_catalog(source, &output, self.config.compression_level)
        })
    }

    pub fn update_retiling(&self) -> Result<()> {
        let source = &self.config.retiling_source;
        let output = self.config.output_directory.join("retiling_catalog.bin");
        let mut fingerprint = self.create_fingerprint(RETILING_CODE_INPUTS);
        let clean = hash_repository(&mut fingerprint, source)?;
        hash_input(&mut fingerprint, &fs::read(&self.config.tile_fingerprints)?);
        update_catalog(&output, fingerprint, clean, || {
            retiling::build_catalog(
                source,
                &output,
                &self.config.tile_fingerprints,
                self.config.compression_level,
            )
        })
    }

    pub fn update_patches(&self) -> Result<()> {
        let built = patches::assemble_patches(
            &self.config.asm,
            &self.config.asar_source,
            &self.config.output_directory,
            self.config.asar_executable.as_deref(),
        )?;
        let mut fingerprint = self.create_fingerprint(PATCH_CODE_INPUTS);
        let clean = hash_repository(&mut fingerprint, &self.config.logic_source)?;
        let mut artifacts = Vec::new();
        for entry in fs::read_dir(self.config.output_directory.join("patches"))? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "ips" || extension == "sym")
            {
                artifacts.push(path);
            }
        }
        artifacts.sort();
        for path in artifacts {
            hash_input(
                &mut fingerprint,
                path.file_name().unwrap().as_encoded_bytes(),
            );
            hash_input(&mut fingerprint, &fs::read(path)?);
        }
        let output = self.config.output_directory.join("patch_catalog.bin");
        update_catalog(&output, fingerprint, clean, || {
            patches::package_catalog(
                built,
                &self.config.logic_source,
                &self.config.output_directory,
                self.config.compression_level,
            )
        })
    }

    pub fn update_all(&self) -> Result<()> {
        self.update_logic()?;
        self.update_retiling()?;
        self.update_patches()
    }
}

fn hash_input(fingerprint: &mut Sha256, bytes: &[u8]) {
    fingerprint.update(bytes.len().to_le_bytes());
    fingerprint.update(bytes);
}

/// Non-Git overrides and dirty checkouts rebuild without reusable cache metadata.
fn hash_repository(fingerprint: &mut Sha256, source: &Path) -> Result<bool> {
    if !source.join(".git").exists() {
        return Ok(false);
    }
    let head = Command::new("git")
        .arg("-C")
        .arg(source)
        .args(["rev-parse", "HEAD"])
        .output()?;
    let status = Command::new("git")
        .arg("-C")
        .arg(source)
        .args([
            "status",
            "--porcelain",
            "--untracked-files=all",
            "--ignore-submodules=none",
        ])
        .output()?;
    hash_input(
        fingerprint,
        source.canonicalize()?.as_os_str().as_encoded_bytes(),
    );
    hash_input(fingerprint, &head.stdout);
    Ok(head.status.success() && status.status.success() && status.stdout.is_empty())
}

fn update_catalog(
    output: &Path,
    fingerprint: Sha256,
    clean: bool,
    build: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let fingerprint = fingerprint.finalize();
    let mut metadata = output.as_os_str().to_owned();
    metadata.push(".sha256");
    let metadata = PathBuf::from(metadata);
    if clean && output.exists() && fs::read(&metadata).unwrap_or_default() == fingerprint.as_slice()
    {
        eprintln!("Reused catalog at {}", output.display());
        return Ok(());
    }
    let directory = output.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(directory)?;
    // A failed or interrupted build must never reuse the old fingerprint.
    fs::write(&metadata, [])?;
    build()?;
    if clean {
        fs::write(metadata, fingerprint)?;
    }
    Ok(())
}
