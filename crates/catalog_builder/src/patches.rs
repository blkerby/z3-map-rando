use crate::z3_json_data::{self, collect_source_files, read_source};
use anyhow::{Context, Result, bail};
use patch_catalog::{
    ItemEncoding, ItemLocation, ItemLocationId, PatchCatalog, PatchIps, PatchSymbols, SourceRoomId,
};
use serde::{
    Deserialize,
    de::value::{Error, MapDeserializer, SeqDeserializer},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
};

const ASSEMBLY_ARGUMENTS: &[&str] = &["--fix-checksum=off", "--no-title-check", "--disable-read"];

pub struct BuiltPatches {
    pub patches: PatchIps,
    pub symbols: PatchSymbols,
    patch_count: usize,
}

/// Build the patched Asar and update independently cached IPS patches.
/// Also export and import the shared ASM interface into its typed manifest.
/// Dependency tracking supports only literal `incsrc "path"` directives.
pub fn build_patches(
    asm_directory: &Path,
    asar_source: &Path,
    build_directory: &Path,
    asar_executable: Option<&Path>,
) -> Result<BuiltPatches> {
    let asm_directory = asm_directory.canonicalize()?;
    fs::create_dir_all(build_directory)?;
    let build_directory = build_directory.canonicalize()?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(build_directory.join("cache.lock"))?;
    lock.lock()?;

    let asar = if let Some(asar) = asar_executable {
        asar.canonicalize()?
    } else {
        build_asar(asar_source, &build_directory)?
    };

    let mut assembler_fingerprint = Sha256::new();
    assembler_fingerprint.update(include_bytes!("patches.rs"));
    assembler_fingerprint.update(fs::read(&asar)?);

    let output_directory = build_directory.join("patches");
    fs::create_dir_all(&output_directory)?;
    let temporary = tempfile::tempdir_in(&output_directory)?;
    let temporary_rom = temporary.path().join("dummy.smc");
    let mut roots = Vec::new();
    for entry in fs::read_dir(&asm_directory)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "asm") {
            roots.push(path);
        }
    }
    roots.sort();

    let mut outputs = BTreeMap::new();
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
        let is_symbols = filename == "symbols.asm";
        let extension = if is_symbols { "sym" } else { "ips" };
        let output = output_directory.join(filename).with_extension(extension);
        let metadata = output.with_extension(format!("{extension}.sha256"));
        if output.exists() && fs::read(&metadata).unwrap_or_default() == fingerprint.as_slice() {
            reused += 1;
        } else {
            eprintln!("Assembling {}", root.display());
            let temporary_output = temporary.path().join(filename).with_extension(extension);
            File::create(&temporary_rom)?;
            let mut command = Command::new(&asar);
            command.args(ASSEMBLY_ARGUMENTS);
            if is_symbols {
                command
                    .arg("--symbols=nocash")
                    .arg(format!("--symbols-path={}", temporary_output.display()));
            } else {
                command.arg("--ips").arg(&temporary_output);
            }
            command.arg(&root).arg(&temporary_rom);
            run_command(&mut command)?;
            let temporary_metadata = temporary_output.with_extension(format!("{extension}.sha256"));
            fs::write(&temporary_metadata, fingerprint)?;
            // Invalidate the old fingerprint before replacing its output. An interruption
            // between publishing the two files must leave a cache miss.
            fs::write(&metadata, [])?;
            fs::rename(&temporary_output, &output)?;
            fs::rename(&temporary_metadata, &metadata)?;
            rebuilt += 1;
        }
        if !is_symbols {
            outputs.insert(
                root.file_stem().unwrap().to_string_lossy().into_owned(),
                fs::read(&output)?,
            );
        }
    }

    let mut symbols = BTreeMap::new();
    let symbol_file = fs::read_to_string(output_directory.join("symbols.sym"))?;
    for line in symbol_file.lines() {
        let Some((value, name)) = line.split_once(' ') else {
            continue;
        };
        if let Some(name) = name.trim().strip_prefix("export_") {
            symbols.insert(name.to_owned(), u32::from_str_radix(value, 16)?);
        }
    }
    let symbols = PatchSymbols::deserialize(MapDeserializer::<_, Error>::new(symbols.into_iter()))?;
    let patch_count = outputs.len();
    let mut entries = Vec::new();
    for (name, bytes) in outputs {
        entries.push((name, SeqDeserializer::<_, Error>::new(bytes.into_iter())));
    }
    let patches = PatchIps::deserialize(MapDeserializer::<_, Error>::new(entries.into_iter()))?;
    eprintln!(
        "Updated ASM artifacts in {}\n  {rebuilt} assembled, {reused} reused",
        output_directory.display()
    );
    Ok(BuiltPatches {
        patches,
        symbols,
        patch_count,
    })
}

/// Assemble patches and package their matching symbols and z3-json-data encodings.
pub fn build_catalog(
    asm_directory: &Path,
    asar_source: &Path,
    source_directory: &Path,
    output_directory: &Path,
    asar_executable: Option<&Path>,
    compression_level: i32,
) -> Result<()> {
    let built = build_patches(
        asm_directory,
        asar_source,
        output_directory,
        asar_executable,
    )?;
    let mut catalog = PatchCatalog {
        patches: built.patches,
        symbols: built.symbols,
        item_locations: BTreeMap::new(),
        items: BTreeMap::new(),
    };
    let items: z3_json_data::Items = read_source(&source_directory.join("items.json"))?;
    for category in [
        items.inventory,
        items.refills,
        items.bottle_contents,
        items.currency,
        items.dungeon_items,
        items.dungeon_prizes,
        items.goal_items,
        items.expansions,
    ] {
        for item in category {
            let receipt_id = u8::from_str_radix(item.item_receipt_id.trim_start_matches("0x"), 16)?;
            let prize_patch_bytes = if let Some(source_bytes) = item.prize_patch_bytes {
                let mut bytes = [0; 6];
                for (i, byte) in source_bytes.iter().enumerate() {
                    bytes[i] = u8::from_str_radix(byte.trim_start_matches("0x"), 16)?;
                }
                Some(bytes)
            } else {
                None
            };
            catalog.items.insert(
                item.name,
                ItemEncoding {
                    receipt_id,
                    prize_patch_bytes,
                },
            );
        }
    }
    let mut room_paths = Vec::new();
    collect_source_files(&source_directory.join("rooms"), &mut room_paths)?;
    room_paths.sort();
    for path in room_paths {
        let room: z3_json_data::Room = read_source(&path)?;
        let room_id = if room.room_type == logic_catalog::RoomKind::Overworld {
            SourceRoomId::Overworld(room.id)
        } else {
            SourceRoomId::Underworld(room.id)
        };
        for item in room.items {
            // Flute activation is a fixed event, not an item placement location.
            if item.item == "OcarinaActive" {
                continue;
            }
            let addresses = match &item.item_address {
                z3_json_data::Addresses::One(address) => std::slice::from_ref(address),
                z3_json_data::Addresses::Many(addresses) => addresses.as_slice(),
            };
            let mut rom_addresses = Vec::new();
            for address in addresses {
                if address != "unknown" {
                    rom_addresses.push(u32::from_str_radix(address.trim_start_matches("0x"), 16)?);
                }
            }
            catalog.item_locations.insert(
                ItemLocationId {
                    room_id,
                    item_id: item.id,
                },
                ItemLocation { rom_addresses },
            );
        }
    }
    let output_catalog = output_directory.join("patch_catalog.bin");
    let mut temporary = tempfile::NamedTempFile::new_in(output_directory)?;
    {
        let mut output = BufWriter::new(temporary.as_file_mut());
        patch_catalog::encode_catalog(&catalog, &mut output, compression_level)?;
        output.flush()?;
    }
    let bytes_len = temporary.as_file().metadata()?.len();
    temporary.persist(&output_catalog)?;
    eprintln!(
        "Wrote patch catalog to {} ({} bytes)\n  {} IPS patches, {} item locations, {} item encodings",
        output_catalog.display(),
        bytes_len,
        built.patch_count,
        catalog.item_locations.len(),
        catalog.items.len(),
    );
    Ok(())
}

fn build_asar(asar_source: &Path, build_directory: &Path) -> Result<PathBuf> {
    let asar_build = build_directory.join("asar");
    run_command(
        Command::new("cmake")
            .arg("-S")
            .arg(asar_source.join("src"))
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
