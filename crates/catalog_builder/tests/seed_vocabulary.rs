//! Check the identifiers that saved seeds use against `data/seed_vocabulary.json`.
//!
//! Seeds are interpreted against current catalogs, so every identifier a seed
//! saves is permanent (see `docs/data-formats/seed-data.md`). This test fails
//! whenever the catalogs and the committed vocabulary disagree, so that each
//! change to a saved identifier is a deliberate edit of the vocabulary file.
//!
//! RULES FOR UPDATING `data/seed_vocabulary.json`:
//!
//! - A new identifier is added to the vocabulary with its current name.
//! - A changed name is updated only when the identifier keeps its meaning, such
//!   as a corrected spelling. If the identifier now refers to a different
//!   location, door, or variant, the source data is wrong: the old identifier
//!   must be retired and the new one given a fresh identifier.
//! - Before the first release, an identifier may be removed from the vocabulary.
//! - AFTER THE FIRST RELEASE, AN IDENTIFIER IS NEVER REMOVED. It moves to a
//!   retired list together with a migration to a current equivalent that
//!   preserves connectivity and gameplay meaning, and it is never reused.
//!   The retired list and migration form are to be designed when first needed.

use anyhow::{Context, Result};
use catalog_builder::config::BuildConfig;
use common_types::{Item, SUPPORTED_THEMES, SourceRoomId};
use logic_catalog::LockKind;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::Path,
};

/// Keys are formatted identifiers and values are their names. Variant names,
/// theme IDs, and item names are their own names.
#[derive(Deserialize)]
struct SeedVocabulary {
    /// Every authored room, keyed by `<overworld|underworld>:<room_id>`.
    rooms: BTreeMap<String, String>,
    /// Keyed by `<room key>:<item_id>`.
    item_locations: BTreeMap<String, String>,
    /// Doors that a Small Key can be assigned to, keyed by `<room key>:<door_id>`.
    small_key_doors: BTreeMap<String, String>,
    /// Edge variants, including the implicit `closed`.
    border_variants: BTreeSet<String>,
    /// Retiling theme IDs in `SUPPORTED_THEMES`.
    themes: BTreeSet<String>,
    /// Serialized `Item` names, as saved in item placements.
    items: BTreeSet<String>,
}

#[test]
fn check_seed_vocabulary() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let vocabulary: SeedVocabulary =
        serde_json::from_slice(&fs::read(root.join("data/seed_vocabulary.json"))?)?;
    let mut config = BuildConfig::read_config(&root)?;
    config.resolve_paths(&root);
    // The catalogs are prerequisites, built by `catalog_builder` before the tests run.
    let logic_path = config.output_directory.join("logic_catalog.bin");
    let logic_catalog = logic_catalog::decode_catalog(
        File::open(&logic_path)
            .with_context(|| format!("run catalog_builder to build {}", logic_path.display()))?,
    )?;
    let retiling_path = config.output_directory.join("retiling_catalog.bin");
    let retiling_catalog =
        retiling_catalog::decode_catalog(File::open(&retiling_path).with_context(|| {
            format!("run catalog_builder to build {}", retiling_path.display())
        })?)?;

    let mut errors = Vec::new();

    let mut rooms = BTreeMap::new();
    for room in &logic_catalog.rooms {
        if let Some(room_id) = room.room_id {
            rooms.insert(format_room_id(room_id), room.name.clone());
        }
    }
    compare_entries("rooms", &vocabulary.rooms, &rooms, &mut errors);

    let mut item_locations = BTreeMap::new();
    for location in &logic_catalog.item_locations {
        item_locations.insert(
            format!(
                "{}:{}",
                format_room_id(location.id.room_id),
                location.id.item_id
            ),
            location.name.clone(),
        );
    }
    compare_entries(
        "item_locations",
        &vocabulary.item_locations,
        &item_locations,
        &mut errors,
    );

    let mut small_key_doors = BTreeMap::new();
    for door in &logic_catalog.doors {
        if door.lock == LockKind::SmallKey {
            let room = &logic_catalog.rooms[door.room_idx.0 as usize];
            small_key_doors.insert(
                format!("{}:{}", format_room_id(room.room_id.unwrap()), door.door_id),
                format!("{} - {}", room.name, door.name),
            );
        }
    }
    compare_entries(
        "small_key_doors",
        &vocabulary.small_key_doors,
        &small_key_doors,
        &mut errors,
    );

    let mut committed_variants = BTreeMap::new();
    for variant in &vocabulary.border_variants {
        committed_variants.insert(variant.clone(), variant.clone());
    }
    let mut logic_variants = BTreeMap::new();
    for room in &logic_catalog.rooms {
        for side in &room.edge_variants {
            for segment in side {
                for variant in segment.keys() {
                    logic_variants.insert(variant.clone(), variant.clone());
                }
            }
        }
    }
    compare_entries(
        "border_variants (logic catalog)",
        &committed_variants,
        &logic_variants,
        &mut errors,
    );
    let mut retiling_variants = BTreeMap::new();
    for variant in &retiling_catalog.conditions.edge_variants {
        retiling_variants.insert(variant.clone(), variant.clone());
    }
    compare_entries(
        "border_variants (retiling catalog)",
        &committed_variants,
        &retiling_variants,
        &mut errors,
    );

    let mut committed_themes = BTreeMap::new();
    for theme in &vocabulary.themes {
        committed_themes.insert(theme.clone(), theme.clone());
    }
    let mut themes = BTreeMap::new();
    for theme in SUPPORTED_THEMES {
        themes.insert(theme.to_owned(), theme.to_owned());
        for (area_name, area) in &retiling_catalog.areas {
            if area.vanilla_map_id.is_some() && !area.themes.contains_key(theme) {
                errors.push(format!(
                    "themes: supported theme {theme} is missing from retiling area {area_name}"
                ));
            }
        }
    }
    compare_entries("themes", &committed_themes, &themes, &mut errors);

    // `Item` variants cannot be enumerated, so check committed names against the
    // enum, and the names that settings can place against the vocabulary. The
    // vocabulary also lists `nothing`, which an empty slot saves.
    for name in &vocabulary.items {
        if serde_json::from_value::<Item>(json!(name)).is_err() && name != "small_key" {
            errors.push(format!("items: {name} is no longer an item"));
        }
    }
    let mut items = BTreeSet::new();
    let records: Vec<Value> = serde_json::from_slice(&fs::read(&config.items)?)?;
    for record in records {
        items.insert(record["name"].as_str().unwrap().to_owned());
    }
    for name in &items {
        if !vocabulary.items.contains(name) {
            errors.push(format!("items: {name} is missing from the vocabulary"));
        }
    }

    assert!(
        errors.is_empty(),
        "data/seed_vocabulary.json disagrees with the catalogs. Follow the rules at the top of \
         crates/catalog_builder/tests/seed_vocabulary.rs to update it:\n{}",
        errors.join("\n")
    );
    Ok(())
}

fn format_room_id(room_id: SourceRoomId) -> String {
    match room_id {
        SourceRoomId::Overworld(id) => format!("overworld:{id}"),
        SourceRoomId::Underworld(id) => format!("underworld:{id}"),
    }
}

fn compare_entries(
    kind: &str,
    committed: &BTreeMap<String, String>,
    current: &BTreeMap<String, String>,
    errors: &mut Vec<String>,
) {
    for (key, name) in current {
        match committed.get(key) {
            None => errors.push(format!("{kind}: {key} ({name}) is missing from the vocabulary")),
            Some(committed_name) if committed_name != name => errors.push(format!(
                "{kind}: {key} is named {committed_name:?} in the vocabulary but {name:?} in the catalog"
            )),
            Some(_) => {}
        }
    }
    for (key, name) in committed {
        if !current.contains_key(key) {
            errors.push(format!(
                "{kind}: {key} ({name}) is no longer in the catalog"
            ));
        }
    }
}
