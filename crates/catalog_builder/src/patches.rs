use crate::z3_json_data::{self, collect_source_files, read_source};
use anyhow::{Context, Result, bail};
use patch_catalog::{
    Ips, ItemEncoding, ItemLocation, ItemLocationId, PatchCatalog, PatchIps, PatchSymbols,
    SourceRoomId,
};
use serde::{
    Deserialize,
    de::value::{Error, MapDeserializer},
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
    fs::create_dir_all(build_directory)?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(build_directory.join("cache.lock"))?;
    lock.lock()?;
    assemble_patches(asm_directory, asar_source, build_directory, asar_executable)
}

pub(crate) fn assemble_patches(
    asm_directory: &Path,
    asar_source: &Path,
    build_directory: &Path,
    asar_executable: Option<&Path>,
) -> Result<BuiltPatches> {
    let asm_directory = asm_directory.canonicalize()?;
    let build_directory = build_directory.canonicalize()?;
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
                Ips {
                    name: output.file_name().unwrap().to_string_lossy().into_owned(),
                    bytes: fs::read(&output)?,
                },
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
    let patches = PatchIps::deserialize(serde_json::to_value(outputs)?)?;
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
    package_catalog(built, source_directory, output_directory, compression_level)
}

pub(crate) fn package_catalog(
    built: BuiltPatches,
    source_directory: &Path,
    output_directory: &Path,
    compression_level: i32,
) -> Result<()> {
    let items = read_item_encodings(source_directory)?;
    let item_locations = read_item_locations(source_directory, &built.symbols, &items)?;
    let catalog = PatchCatalog {
        patches: built.patches,
        symbols: built.symbols,
        item_locations,
        items,
    };
    write_catalog(
        &catalog,
        built.patch_count,
        output_directory,
        compression_level,
    )
}

fn read_item_encodings(source_directory: &Path) -> Result<BTreeMap<String, ItemEncoding>> {
    let mut encodings = BTreeMap::new();
    let items: z3_json_data::Items = read_source(&source_directory.join("items.json"))?;
    for category in [
        items.inventory,
        items.refills,
        items.bottle_contents,
        items.currency,
        items.dungeon_items,
        items.dungeon_prizes,
        items.health_upgrades,
        items.non_vanilla,
    ] {
        for item in category.items {
            let receipt_id = u8::from_str_radix(item.item_receipt_id.trim_start_matches("0x"), 16)?;
            // Mask, map properties/tile, prize type, number tile, falling-prize ID.
            // See plans/patches.md for the vanilla encodings and custom table layout.
            let prize_patch_bytes = match item.name.as_str() {
                "PendantOfCourage" => Some([0x04, 0x38, 0x60, 0x00, 0x69, 0x01]),
                "PendantOfWisdom" => Some([0x01, 0x32, 0x60, 0x00, 0x69, 0x03]),
                "PendantOfPower" => Some([0x02, 0x34, 0x60, 0x00, 0x69, 0x02]),
                "Crystal1" => Some([0x02, 0x34, 0x64, 0x40, 0x7F, 0x06]),
                "Crystal2" => Some([0x10, 0x34, 0x64, 0x40, 0x79, 0x06]),
                "Crystal3" => Some([0x40, 0x34, 0x64, 0x40, 0x6C, 0x06]),
                "Crystal4" => Some([0x20, 0x34, 0x64, 0x40, 0x6D, 0x06]),
                "Crystal5" => Some([0x04, 0x34, 0x64, 0x40, 0x6E, 0x06]),
                "Crystal6" => Some([0x01, 0x34, 0x64, 0x40, 0x6F, 0x06]),
                "Crystal7" => Some([0x08, 0x34, 0x64, 0x40, 0x7C, 0x06]),
                _ => None,
            };
            encodings.insert(
                item.name,
                ItemEncoding {
                    receipt_id,
                    prize_patch_bytes,
                },
            );
        }
    }
    // These specific randomizer receipts are not individual shared-data records.
    for (name, receipt_id) in [
        ("ProgressiveBoomerang", 0x62),
        ("BombUpgrade5", 0x51),
        ("BombUpgrade10", 0x52),
        ("BombUpgrade50", 0x4C),
        ("ArrowUpgrade5", 0x53),
        ("ArrowUpgrade10", 0x54),
        ("ArrowUpgrade70", 0x4D),
        ("Triforce", 0x6A),
    ] {
        encodings.insert(
            name.to_owned(),
            ItemEncoding {
                receipt_id,
                prize_patch_bytes: None,
            },
        );
    }
    Ok(encodings)
}

fn read_item_locations(
    source_directory: &Path,
    symbols: &PatchSymbols,
    encodings: &BTreeMap<String, ItemEncoding>,
) -> Result<BTreeMap<ItemLocationId, ItemLocation>> {
    let mut locations = BTreeMap::new();
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
            let mut rom_addresses = Vec::new();
            if let Some(address) = &item.item_address {
                rom_addresses.push(u32::from_str_radix(address.trim_start_matches("0x"), 16)?);
            }
            if encodings[&item.item].prize_patch_bytes.is_some() {
                let dungeon_id = room.dungeon_id.context("prize dungeon has no dungeonId")?;
                let index = dungeon_id / 2;
                let (slot, phase, world) = match dungeon_id {
                    0x04 => (0, 3, 0), // Eastern Palace
                    0x06 => (2, 3, 0), // Desert Palace
                    0x14 => (1, 3, 0), // Tower of Hera
                    0x0C => (0, 7, 1), // Palace of Darkness
                    0x10 => (1, 7, 1), // Skull Woods
                    0x18 => (2, 7, 1), // Turtle Rock
                    0x16 => (3, 7, 1), // Thieves' Town
                    0x0E => (4, 7, 1), // Misery Mire
                    0x12 => (5, 7, 1), // Ice Palace
                    0x0A => (6, 7, 1), // Swamp Palace
                    _ => bail!("unsupported prize dungeonId: {dungeon_id:#04X}"),
                };
                let map_properties = 0x53EF2 + 0x12 * slot + 2 * phase;
                rom_addresses = vec![
                    0x1209B + index,
                    map_properties,
                    map_properties + 1,
                    0x180050 + index,
                    0x18007C - 2 * slot + world,
                    0xC6FC + index,
                ];
            }
            if item.item == "SmallKey"
                || item.item == "BigKey"
                || matches!(
                    (room_id, item.id),
                    (SourceRoomId::Underworld(12 | 69), 1 | 2)
                )
            {
                rom_addresses.clear();
            }
            // These gifts keep their vanilla sprite subtype as a placeholder.
            // Their item IDs are read from separate data instead of that subtype.
            let item_address = match (room_id, item.id) {
                (SourceRoomId::Overworld(7), 1) => Some(symbols.catfish_item),
                (SourceRoomId::Underworld(104), 1) => Some(symbols.king_zora_item),
                _ => None,
            };
            if let Some(address) = item_address {
                rom_addresses = vec![((address & 0x7f0000) >> 1) | (address & 0x7fff)];
            }
            locations.insert(
                ItemLocationId {
                    room_id,
                    item_id: item.id,
                },
                ItemLocation { rom_addresses },
            );
        }
    }
    Ok(locations)
}

fn write_catalog(
    catalog: &PatchCatalog,
    patch_count: usize,
    output_directory: &Path,
    compression_level: i32,
) -> Result<()> {
    let output_catalog = output_directory.join("patch_catalog.bin");
    let mut output = BufWriter::new(File::create(&output_catalog)?);
    patch_catalog::encode_catalog(catalog, &mut output, compression_level)?;
    output.flush()?;
    let bytes_len = output.get_ref().metadata()?.len();
    eprintln!(
        "Wrote patch catalog to {} ({} bytes)\n  {} IPS patches, {} item locations, {} item encodings",
        output_catalog.display(),
        bytes_len,
        patch_count,
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
