use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};

const ASSEMBLY_ARGUMENTS: &[&str] = &[
    "--fix-checksum=off",
    "--no-title-check",
    "--disable-read",
    "--ips",
];

/// Build the patched Asar and update independently cached IPS patches.
/// Dependency tracking supports only literal `incsrc "path"` directives.
pub fn build_patches(
    repository: &Path,
    build_directory: &Path,
    asar_override: Option<&Path>,
) -> Result<Vec<PathBuf>> {
    let repository = repository.canonicalize()?;
    fs::create_dir_all(build_directory)?;
    let build_directory = build_directory.canonicalize()?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(build_directory.join("cache.lock"))?;
    lock.lock()?;

    let asar = if let Some(asar) = asar_override {
        asar.canonicalize()?
    } else {
        build_asar(&repository, &build_directory)?
    };

    let mut assembler_fingerprint = Sha256::new();
    assembler_fingerprint.update(include_bytes!("patches.rs"));
    assembler_fingerprint.update(fs::read(&asar)?);

    let output_directory = build_directory.join("patches");
    fs::create_dir_all(&output_directory)?;
    let temporary = tempfile::tempdir_in(&output_directory)?;
    let temporary_rom = temporary.path().join("dummy.smc");
    let mut roots = Vec::new();
    for entry in fs::read_dir(repository.join("patches/src"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "asm") {
            roots.push(path);
        }
    }
    roots.sort();

    let mut outputs = Vec::new();
    let mut rebuilt = 0;
    let mut reused = 0;
    for root in roots {
        let mut dependencies = BTreeMap::new();
        collect_dependencies(&root, &mut dependencies)?;
        let mut fingerprint = assembler_fingerprint.clone();
        for (path, content) in dependencies {
            let path_bytes = path.as_os_str().as_encoded_bytes();
            fingerprint.update(path_bytes.len().to_le_bytes());
            fingerprint.update(path_bytes);
            fingerprint.update(content.len().to_le_bytes());
            fingerprint.update(content);
        }
        let fingerprint = fingerprint.finalize();
        let filename = root.file_name().unwrap();
        let output = output_directory.join(filename).with_extension("ips");
        let metadata = output.with_extension("ips.sha256");
        if output.exists() && fs::read(&metadata).unwrap_or_default() == fingerprint.as_slice() {
            reused += 1;
        } else {
            eprintln!("Assembling {}", root.display());
            let temporary_output = temporary.path().join(filename).with_extension("ips");
            File::create(&temporary_rom)?;
            run_command(
                Command::new(&asar)
                    .args(ASSEMBLY_ARGUMENTS)
                    .arg(&temporary_output)
                    .arg(&root)
                    .arg(&temporary_rom),
            )?;
            let temporary_metadata = temporary_output.with_extension("ips.sha256");
            fs::write(&temporary_metadata, fingerprint)?;
            // Invalidate the old fingerprint before replacing its output. An interruption
            // between publishing the two files must leave a cache miss.
            fs::write(&metadata, [])?;
            fs::rename(&temporary_output, &output)?;
            fs::rename(&temporary_metadata, &metadata)?;
            rebuilt += 1;
        }
        outputs.push(output);
    }
    eprintln!(
        "Updated IPS patches in {}\n  {rebuilt} assembled, {reused} reused",
        output_directory.display()
    );
    Ok(outputs)
}

fn build_asar(repository: &Path, build_directory: &Path) -> Result<PathBuf> {
    let asar_build = build_directory.join("asar");
    run_command(
        Command::new("cmake")
            .arg("-S")
            .arg(repository.join("asar/src"))
            .arg("-B")
            .arg(&asar_build)
            .args([
                "-DCMAKE_BUILD_TYPE=Release",
                "-DASAR_GEN_EXE=ON",
                "-DASAR_GEN_DLL=ON",
                "-DASAR_GEN_LIB=ON",
            ]),
    )?;
    run_command(Command::new("cmake").arg("--build").arg(&asar_build).args([
        "--config",
        "Release",
        "--target",
        "asar-standalone",
    ]))?;
    let executable = format!("asar{}", std::env::consts::EXE_SUFFIX);
    let bin_directory = asar_build.join("asar/bin");
    let release_executable = bin_directory.join("Release").join(&executable);
    if release_executable.exists() {
        Ok(release_executable)
    } else {
        Ok(bin_directory.join(executable))
    }
}

fn collect_dependencies(path: &Path, dependencies: &mut BTreeMap<PathBuf, Vec<u8>>) -> Result<()> {
    let path = path.canonicalize()?;
    if dependencies.contains_key(&path) {
        return Ok(());
    }
    let content = fs::read_to_string(&path)?;
    dependencies.insert(path.clone(), content.as_bytes().to_vec());
    for line in content.lines() {
        let Some((directive, argument)) = line.trim_start().split_once(char::is_whitespace) else {
            continue;
        };
        if !directive.eq_ignore_ascii_case("incsrc") {
            continue;
        }
        if let Some(argument) = argument.trim_start().strip_prefix('"') {
            if let Some((included_path, _)) = argument.split_once('"') {
                collect_dependencies(&path.parent().unwrap().join(included_path), dependencies)?;
            }
        }
    }
    Ok(())
}

fn run_command(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("Running {command:?}"))?;
    if !status.success() {
        bail!("Command failed ({status}): {command:?}");
    }
    Ok(())
}
