use anyhow::{Context, Result, anyhow};
use common_types::SourceRoomId;
use preset_catalog::{PresetCatalog, RootPreset, SkillPreset};
use seed_data::{NotableSettings, SkillSettings};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, fs::File, io::BufWriter, path::Path};

/// Difficulties ranked easiest to hardest. Others (e.g. "Ignored") never take part in skill presets.
const DIFFICULTIES: [&str; 5] = ["Basic", "Medium", "Hard", "Very Hard", "Expert"];

#[derive(Deserialize)]
struct TechRecord {
    tech_id: u32,
    /// Present on proficiencies, which have one record per level, lowest first.
    level: Option<u32>,
    difficulty: String,
}

#[derive(Deserialize)]
struct NotableRecord {
    room_id: SourceRoomId,
    notable_id: u32,
    difficulty: String,
}

/// A built-in preset file: its scope-local ID, display label, and settings.
#[derive(Deserialize)]
struct PresetFile {
    id: String,
    label: String,
    settings: serde_json::Value,
}

/// Prepare the built-in generation presets.
///
/// Skill presets are derived from the difficulty classifications: the preset for a difficulty (ID
/// `hard` for "Hard") enables every technique and notable ranked at or below it, and sets each
/// proficiency to its highest level ranked at or below it (its lowest level if none is). Unranked
/// members are disabled. Root presets come from `generation/root/*.json` under the preset directory,
/// the default first, and are resolved into complete settings by applying the skill preset they
/// select.
pub fn build_catalog(
    preset_directory: &Path,
    tech_data: &Path,
    notable_data: &Path,
    output: &Path,
    compression_level: i32,
) -> Result<()> {
    let techs: Vec<TechRecord> = serde_json::from_slice(&fs::read(tech_data)?)
        .with_context(|| format!("reading {}", tech_data.display()))?;
    let notables: Vec<NotableRecord> = serde_json::from_slice(&fs::read(notable_data)?)
        .with_context(|| format!("reading {}", notable_data.display()))?;

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

    let mut paths = Vec::new();
    for entry in fs::read_dir(preset_directory.join("generation/root"))? {
        paths.push(entry?.path());
    }
    paths.sort();
    for path in paths {
        let file: PresetFile = serde_json::from_slice(&fs::read(&path)?)
            .with_context(|| format!("reading preset {}", path.display()))?;
        let mut settings = file.settings;
        let skill_preset = settings["skill"]["preset"]
            .as_str()
            .with_context(|| format!("preset {} selects no skill preset", path.display()))?
            .to_owned();
        let skill = catalog
            .skill
            .iter()
            .find(|p| p.id == skill_preset)
            .ok_or_else(|| anyhow!("unknown skill preset {skill_preset} in {}", path.display()))?;
        settings["skill"] = serde_json::to_value(&skill.settings)?;
        let preset = RootPreset {
            id: file.id,
            label: file.label,
            skill_preset,
            settings: serde_json::from_value(settings)
                .with_context(|| format!("resolving preset {}", path.display()))?,
        };
        if preset.id == "default" {
            catalog.root.insert(0, preset);
        } else {
            catalog.root.push(preset);
        }
    }

    preset_catalog::encode_catalog(
        &catalog,
        BufWriter::new(File::create(output)?),
        compression_level,
    )
}
