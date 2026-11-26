use crate::config::BuildConfig;
use crate::z3_json_data::{self, collect_source_files, read_source};
use anyhow::{Context, Result, anyhow, bail};
use common_types::SourceRoomId;
use preset_catalog::{ItemPoolPreset, PresetCatalog, RootPreset, SkillPreset, StartingStatePreset};
use seed_data::{CustomizationPreset, SettingsErrors, SettingsMode, SkillSettings};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    fs::File,
    io::BufWriter,
    path::Path,
};

/// Difficulties ranked easiest to hardest. Others (e.g. "Ignored") never take part in skill presets.
const DIFFICULTIES: [&str; 5] = ["Basic", "Medium", "Hard", "Very Hard", "Expert"];

#[derive(Deserialize)]
struct TechRecord {
    tech_id: u32,
    name: String,
    /// Present on proficiencies, which have one record per level, lowest first.
    level: Option<u32>,
    difficulty: String,
}

#[derive(Deserialize)]
struct NotableRecord {
    room_id: SourceRoomId,
    room_name: String,
    notable_id: u32,
    name: String,
    difficulty: String,
}

/// A built-in preset file: its scope-local ID, display label, and settings.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PresetFile {
    id: String,
    label: String,
    settings: serde_json::Value,
}

/// Everything the frontend needs about generation and customization settings, written as
/// `settings_data.json` beside the preset catalog.
#[derive(Serialize)]
struct SettingsData {
    difficulties: [&'static str; 5],
    proficiencies: Vec<ProficiencyData>,
    techs: Vec<TechData>,
    notables: Vec<NotableData>,
    /// The item records, unchanged.
    items: serde_json::Value,
    /// The prepared presets of both contexts, with the settings revision.
    presets: PresetCatalog,
}

#[derive(Serialize)]
struct ProficiencyData {
    id: u32,
    name: String,
    levels: Vec<LevelData>,
}

#[derive(Serialize)]
struct LevelData {
    level: u32,
    difficulty: String,
    /// The upstream note of the tier with this value; absent for level 0 (no proficiency).
    note: Option<String>,
}

#[derive(Serialize)]
struct TechData {
    id: u32,
    name: String,
    difficulty: String,
    note: String,
}

#[derive(Serialize)]
struct NotableData {
    room: SourceRoomId,
    room_name: String,
    id: u32,
    name: String,
    difficulty: String,
    note: String,
    detail_note: Option<String>,
}

/// Prepare the built-in presets, writing the preset catalog and `settings_data.json`.
///
/// Skill presets are derived from the difficulty classifications: the preset for a difficulty (ID
/// `hard` for "Hard") enables every technique and notable ranked at or below it, and sets each
/// proficiency to its highest level ranked at or below it (its lowest level if none is). Unranked
/// members are disabled. Root presets come from `<context>/root/*.json` under the preset directory,
/// the default first; generation root presets are resolved into complete settings by applying the
/// skill, item pool, and starting state presets they select.
///
/// Each technique, proficiency level, and notable is described by its upstream note in
/// z3-json-data, matched by ID. A note's lines are joined into one paragraph.
///
/// `revision` identifies the processing code and prepared data; it is recorded in both outputs.
pub fn build_catalog(config: &BuildConfig, output: &Path, revision: String) -> Result<()> {
    let techs: Vec<TechRecord> = serde_json::from_slice(&fs::read(&config.tech_data)?)
        .with_context(|| format!("reading {}", config.tech_data.display()))?;
    let notables: Vec<NotableRecord> = serde_json::from_slice(&fs::read(&config.notable_data)?)
        .with_context(|| format!("reading {}", config.notable_data.display()))?;
    let items: serde_json::Value = serde_json::from_slice(&fs::read(&config.items)?)
        .with_context(|| format!("reading {}", config.items.display()))?;

    let source_techs: z3_json_data::Techs = read_source(&config.logic_source.join("tech.json"))?;
    let mut tech_sources = BTreeMap::new();
    let mut pending = Vec::new();
    for category in source_techs.tech_categories {
        pending.extend(category.techs);
    }
    while let Some(mut tech) = pending.pop() {
        pending.append(&mut tech.extension_techs);
        tech_sources.insert(tech.id, tech);
    }
    let mut room_paths = Vec::new();
    collect_source_files(&config.logic_source.join("rooms"), &mut room_paths)?;
    let mut notable_sources = BTreeMap::new();
    for path in room_paths {
        let room: z3_json_data::Room = read_source(&path)?;
        for notable in room.notables {
            notable_sources.insert((room.id, notable.id), (room.name.clone(), notable));
        }
    }

    let mut catalog = PresetCatalog {
        revision,
        root: Vec::new(),
        skill: Vec::new(),
        item_pool: Vec::new(),
        starting_state: Vec::new(),
        customization: Vec::new(),
        proficiency_levels: BTreeMap::new(),
        item_minimums: BTreeMap::new(),
        ordinary_item_count: logic_catalog::decode_catalog(File::open(
            config.output_directory.join("logic_catalog.bin"),
        )?)?
        .count_ordinary_item_locations(),
    };
    for item in items.as_array().unwrap() {
        if matches!(item["category"].as_str(), Some("dungeon" | "prize")) {
            continue;
        }
        catalog.item_minimums.insert(
            serde_json::from_value(item["name"].clone())?,
            serde_json::from_value(item["min"].clone())?,
        );
    }
    for tech in &techs {
        if let Some(level) = tech.level {
            catalog
                .proficiency_levels
                .entry(tech.tech_id)
                .or_default()
                .push(level);
        }
    }
    for (max_rank, label) in DIFFICULTIES.iter().enumerate() {
        let reached = |difficulty: &str| matches!(DIFFICULTIES.iter().position(|d| *d == difficulty), Some(rank) if rank <= max_rank);
        let mut settings = SkillSettings {
            proficiencies: BTreeMap::new(),
            tech: BTreeMap::new(),
            notables: BTreeMap::new(),
        };
        for tech in &techs {
            match tech.level {
                Some(level) => {
                    let current = settings.proficiencies.entry(tech.tech_id).or_insert(level);
                    if reached(&tech.difficulty) {
                        *current = level;
                    }
                }
                None => {
                    settings
                        .tech
                        .insert(tech.tech_id, reached(&tech.difficulty));
                }
            }
        }
        for notable in &notables {
            settings
                .notables
                .entry(notable.room_id)
                .or_default()
                .insert(notable.notable_id, reached(&notable.difficulty));
        }
        catalog.skill.push(SkillPreset {
            id: label.to_lowercase().replace(' ', "_"),
            label: label.to_string(),
            settings,
        });
    }

    for file in read_presets(&config.presets.join("generation/item_pool"))? {
        catalog.item_pool.push(ItemPoolPreset {
            settings: serde_json::from_value(file.settings)
                .with_context(|| format!("reading item pool preset {}", file.id))?,
            id: file.id,
            label: file.label,
        });
    }

    for file in read_presets(&config.presets.join("generation/starting_state"))? {
        catalog.starting_state.push(StartingStatePreset {
            settings: serde_json::from_value(file.settings)
                .with_context(|| format!("reading starting state preset {}", file.id))?,
            id: file.id,
            label: file.label,
        });
    }

    for file in read_presets(&config.presets.join("generation/root"))? {
        let mut settings = file.settings;
        let skill_preset = settings["skill"]["preset"]
            .as_str()
            .with_context(|| format!("preset {} selects no skill preset", file.id))?
            .to_owned();
        let skill = catalog
            .skill
            .iter()
            .find(|p| p.id == skill_preset)
            .ok_or_else(|| anyhow!("unknown skill preset {skill_preset} in preset {}", file.id))?;
        let mut skill_values = serde_json::to_value(&skill.settings)?;
        settings["skill"].as_object_mut().unwrap().remove("preset");
        merge_settings(&mut skill_values, &settings["skill"]);
        settings["skill"] = skill_values;
        let item_pool_preset = settings["item_pool"]["preset"]
            .as_str()
            .with_context(|| format!("preset {} selects no item pool preset", file.id))?
            .to_owned();
        let item_pool = catalog
            .item_pool
            .iter()
            .find(|p| p.id == item_pool_preset)
            .ok_or_else(|| {
                anyhow!(
                    "unknown item pool preset {item_pool_preset} in preset {}",
                    file.id
                )
            })?;
        let mut item_pool_values = serde_json::to_value(&item_pool.settings)?;
        settings["item_pool"]
            .as_object_mut()
            .unwrap()
            .remove("preset");
        merge_settings(&mut item_pool_values, &settings["item_pool"]);
        settings["item_pool"] = item_pool_values;
        let starting_state_preset = settings["starting_state"]["preset"]
            .as_str()
            .with_context(|| format!("preset {} selects no starting state preset", file.id))?
            .to_owned();
        let starting_state = catalog
            .starting_state
            .iter()
            .find(|p| p.id == starting_state_preset)
            .ok_or_else(|| {
                anyhow!(
                    "unknown starting state preset {starting_state_preset} in preset {}",
                    file.id
                )
            })?;
        let mut starting_state_values = serde_json::to_value(&starting_state.settings)?;
        settings["starting_state"]
            .as_object_mut()
            .unwrap()
            .remove("preset");
        merge_settings(&mut starting_state_values, &settings["starting_state"]);
        settings["starting_state"] = starting_state_values;
        catalog
            .validate_settings(&mut settings)
            .with_context(|| format!("validating preset {}", file.id))?;
        catalog.root.push(RootPreset {
            settings: serde_json::from_value(settings)
                .with_context(|| format!("resolving preset {}", file.id))?,
            id: file.id,
            label: file.label,
            skill_preset,
            item_pool_preset,
            starting_state_preset,
        });
    }

    for file in read_presets(&config.presets.join("customization/root"))? {
        catalog.customization.push(CustomizationPreset {
            settings: serde_json::from_value(file.settings)
                .with_context(|| format!("reading customization preset {}", file.id))?,
            id: file.id,
            label: file.label,
        });
    }
    // Processing a customization preset without tweaks validates it against the schema.
    for preset in &catalog.customization {
        let document = json!({"preset": preset.id});
        let report = catalog.process_customization(&document, SettingsMode::Strict)?;
        if !report.errors.is_empty() {
            bail!(
                "validating customization preset {}: {}",
                preset.id,
                SettingsErrors(report.errors)
            );
        }
    }

    let mut settings_data = SettingsData {
        difficulties: DIFFICULTIES,
        proficiencies: Vec::new(),
        techs: Vec::new(),
        notables: Vec::new(),
        items,
        presets: catalog,
    };
    let mut seen_techs = BTreeSet::new();
    let mut seen_proficiency_levels = BTreeSet::new();
    for tech in techs {
        let source = tech_sources
            .get(&tech.tech_id)
            .with_context(|| format!("tech {} is not in tech.json", tech.name))?;
        let Some(level) = tech.level else {
            if source.note.is_empty() {
                bail!("tech {} has no note", tech.name);
            }
            seen_techs.insert(tech.tech_id);
            settings_data.techs.push(TechData {
                id: tech.tech_id,
                name: tech.name,
                difficulty: tech.difficulty,
                note: source.note.join(" "),
            });
            continue;
        };
        seen_proficiency_levels.insert((tech.tech_id, level));
        let note = if level == 0 {
            None
        } else {
            let tier = source
                .tiers
                .iter()
                .find(|t| t.value == level)
                .with_context(|| format!("tech {} has no tier {level}", tech.name))?;
            Some(tier.note.join(" "))
        };
        let level = LevelData {
            level,
            difficulty: tech.difficulty,
            note,
        };
        match settings_data
            .proficiencies
            .iter_mut()
            .find(|p| p.id == tech.tech_id)
        {
            Some(proficiency) => proficiency.levels.push(level),
            None => settings_data.proficiencies.push(ProficiencyData {
                id: tech.tech_id,
                name: tech.name,
                levels: vec![level],
            }),
        }
    }
    let mut seen_notables = BTreeSet::new();
    for notable in notables {
        let (_, source) = notable_sources
            .get(&(notable.room_id, notable.notable_id))
            .with_context(|| format!("notable {} is not in the room data", notable.name))?;
        seen_notables.insert((notable.room_id, notable.notable_id));
        settings_data.notables.push(NotableData {
            room: notable.room_id,
            room_name: notable.room_name,
            id: notable.notable_id,
            name: notable.name,
            difficulty: notable.difficulty,
            note: source.note.join(" "),
            detail_note: if source.detail_note.is_empty() {
                None
            } else {
                Some(source.detail_note.join(" "))
            },
        });
    }

    let mut missing = Vec::new();
    for source in tech_sources.values() {
        if source.tiers.is_empty() {
            if !seen_techs.contains(&source.id) {
                missing.push(format!(
                    "tech {} (id {}) missing from {}",
                    source.name,
                    source.id,
                    config.tech_data.display()
                ));
            }
        } else {
            for tier in &source.tiers {
                if !seen_proficiency_levels.contains(&(source.id, tier.value)) {
                    missing.push(format!(
                        "proficiency {} (id {}, level {}) missing from {}",
                        source.name,
                        source.id,
                        tier.value,
                        config.tech_data.display()
                    ));
                }
            }
        }
    }
    for (&(room_id, notable_id), (room_name, source)) in &notable_sources {
        if !seen_notables.contains(&(room_id, notable_id)) {
            missing.push(format!(
                "notable {} (id {}) in {} (room {}) missing from {}",
                source.name,
                notable_id,
                room_name,
                room_id.0,
                config.notable_data.display()
            ));
        }
    }
    if !missing.is_empty() {
        bail!("unclassified upstream entries:\n  {}", missing.join("\n  "));
    }

    preset_catalog::encode_catalog(
        &settings_data.presets,
        BufWriter::new(File::create(output)?),
        config.compression_level,
    )?;
    serde_json::to_writer(
        BufWriter::new(File::create(
            config.output_directory.join("settings_data.json"),
        )?),
        &settings_data,
    )?;
    Ok(())
}

fn merge_settings(values: &mut serde_json::Value, tweaks: &serde_json::Value) {
    if let (Some(values), Some(tweaks)) = (values.as_object_mut(), tweaks.as_object()) {
        for (key, tweak) in tweaks {
            merge_settings(
                values.entry(key.clone()).or_insert(serde_json::Value::Null),
                tweak,
            );
        }
    } else {
        *values = tweaks.clone();
    }
}

/// The preset files in a directory, the default first.
fn read_presets(directory: &Path) -> Result<Vec<PresetFile>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        paths.push(entry?.path());
    }
    paths.sort();
    let mut presets = Vec::new();
    for path in paths {
        let file: PresetFile = serde_json::from_slice(&fs::read(&path)?)
            .with_context(|| format!("reading preset {}", path.display()))?;
        if file.id == "default" {
            presets.insert(0, file);
        } else {
            presets.push(file);
        }
    }
    Ok(presets)
}
