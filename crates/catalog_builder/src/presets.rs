use crate::config::BuildConfig;
use anyhow::{Context, Result, anyhow};
use common_types::SourceRoomId;
use preset_catalog::{PresetCatalog, RootPreset, SkillPreset};
use seed_data::{NotableSettings, SkillSettings};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, fs::File, io::BufWriter, path::Path};

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
    generation: PresetCatalog,
    customization: Vec<PresetFile>,
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
}

#[derive(Serialize)]
struct TechData {
    id: u32,
    name: String,
    difficulty: String,
}

#[derive(Serialize)]
struct NotableData {
    world: &'static str,
    room: u32,
    room_name: String,
    id: u32,
    name: String,
    difficulty: String,
}

/// Prepare the built-in presets, writing the preset catalog and `settings_data.json`.
///
/// Skill presets are derived from the difficulty classifications: the preset for a difficulty (ID
/// `hard` for "Hard") enables every technique and notable ranked at or below it, and sets each
/// proficiency to its highest level ranked at or below it (its lowest level if none is). Unranked
/// members are disabled. Root presets come from `<context>/root/*.json` under the preset directory,
/// the default first; generation root presets are resolved into complete settings by applying the
/// skill preset they select.
pub fn build_catalog(config: &BuildConfig, output: &Path) -> Result<()> {
    let techs: Vec<TechRecord> = serde_json::from_slice(&fs::read(&config.tech_data)?)
        .with_context(|| format!("reading {}", config.tech_data.display()))?;
    let notables: Vec<NotableRecord> = serde_json::from_slice(&fs::read(&config.notable_data)?)
        .with_context(|| format!("reading {}", config.notable_data.display()))?;
    let items: serde_json::Value = serde_json::from_slice(&fs::read(&config.items)?)
        .with_context(|| format!("reading {}", config.items.display()))?;

    let mut catalog = PresetCatalog {
        root: Vec::new(),
        skill: Vec::new(),
    };
    for (max_rank, label) in DIFFICULTIES.iter().enumerate() {
        let reached = |difficulty: &str| matches!(DIFFICULTIES.iter().position(|d| *d == difficulty), Some(rank) if rank <= max_rank);
        let mut settings = SkillSettings {
            proficiencies: BTreeMap::new(),
            tech: BTreeMap::new(),
            notables: NotableSettings {
                overworld: BTreeMap::new(),
                underworld: BTreeMap::new(),
            },
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
            let (rooms, room) = match notable.room_id {
                SourceRoomId::Overworld(room) => (&mut settings.notables.overworld, room),
                SourceRoomId::Underworld(room) => (&mut settings.notables.underworld, room),
            };
            rooms
                .entry(room)
                .or_default()
                .insert(notable.notable_id, reached(&notable.difficulty));
        }
        catalog.skill.push(SkillPreset {
            id: label.to_lowercase().replace(' ', "_"),
            label: label.to_string(),
            settings,
        });
    }

    for file in read_root_presets(&config.presets.join("generation/root"))? {
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
        settings["skill"] = serde_json::to_value(&skill.settings)?;
        catalog.root.push(RootPreset {
            settings: serde_json::from_value(settings)
                .with_context(|| format!("resolving preset {}", file.id))?,
            id: file.id,
            label: file.label,
            skill_preset,
        });
    }

    let mut settings_data = SettingsData {
        difficulties: DIFFICULTIES,
        proficiencies: Vec::new(),
        techs: Vec::new(),
        notables: Vec::new(),
        items,
        generation: catalog,
        customization: read_root_presets(&config.presets.join("customization/root"))?,
    };
    for tech in techs {
        let Some(level) = tech.level else {
            settings_data.techs.push(TechData {
                id: tech.tech_id,
                name: tech.name,
                difficulty: tech.difficulty,
            });
            continue;
        };
        let level = LevelData {
            level,
            difficulty: tech.difficulty,
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
    for notable in notables {
        let (world, room) = match notable.room_id {
            SourceRoomId::Overworld(room) => ("overworld", room),
            SourceRoomId::Underworld(room) => ("underworld", room),
        };
        settings_data.notables.push(NotableData {
            world,
            room,
            room_name: notable.room_name,
            id: notable.notable_id,
            name: notable.name,
            difficulty: notable.difficulty,
        });
    }

    preset_catalog::encode_catalog(
        &settings_data.generation,
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

/// The root preset files in a directory, the default first.
fn read_root_presets(directory: &Path) -> Result<Vec<PresetFile>> {
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
