use crate::{NotableSettings, Settings, SkillSettings};
use anyhow::{Context, Result, anyhow};
use common_types::SourceRoomId;
use serde::Deserialize;
use std::collections::BTreeMap;

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

/// The bundled default root preset (`data/presets/generation/root/default.json`), resolved into
/// concrete settings.
///
/// Its skill preset is derived from the difficulty data, as in the frontend's
/// `frontend/src/lib/generation/data.ts`: the preset for a difficulty (ID `hard` for "Hard") enables
/// every technique and notable ranked at or below it, and sets each proficiency to its highest level
/// ranked at or below it (its lowest level if none is). Unranked members are disabled.
pub fn default_settings() -> Result<Settings> {
    let mut preset: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/presets/generation/root/default.json"
    ))?;
    let settings = &mut preset["settings"];
    let skill_id = settings["skill"]["preset"]
        .as_str()
        .context("default preset has no skill preset")?
        .to_owned();
    let max_rank = DIFFICULTIES
        .iter()
        .position(|d| d.to_lowercase().replace(' ', "_") == skill_id)
        .ok_or_else(|| anyhow!("unknown skill preset: {skill_id}"))?;
    let reached = |difficulty: &str| matches!(DIFFICULTIES.iter().position(|d| *d == difficulty), Some(rank) if rank <= max_rank);

    let techs: Vec<TechRecord> =
        serde_json::from_str(include_str!("../../../data/tech_data.json"))?;
    let notables: Vec<NotableRecord> =
        serde_json::from_str(include_str!("../../../data/notable_data.json"))?;
    let mut skill = SkillSettings {
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
                let current = skill.proficiencies.entry(tech.tech_id).or_insert(level);
                if reached(&tech.difficulty) {
                    *current = level;
                }
            }
            None => {
                skill.tech.insert(tech.tech_id, reached(&tech.difficulty));
            }
        }
    }
    for notable in &notables {
        let (rooms, room) = match notable.room_id {
            SourceRoomId::Overworld(room) => (&mut skill.notables.overworld, room),
            SourceRoomId::Underworld(room) => (&mut skill.notables.underworld, room),
        };
        rooms
            .entry(room)
            .or_default()
            .insert(notable.notable_id, reached(&notable.difficulty));
    }

    settings["skill"] = serde_json::to_value(skill)?;
    Ok(serde_json::from_value(settings.take())?)
}
