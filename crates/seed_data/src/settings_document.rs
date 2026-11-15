use crate::{ItemPoolSettings, Settings, SkillSettings, StartingState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const SETTINGS_FORMAT_VERSION: u32 = 1;

/// Sparse, public settings: built-in selections and explicit tweaks, without user preset metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsDocument {
    pub format_version: u32,
    pub revision: String,
    pub preset: String,
    #[serde(flatten)]
    pub tweaks: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RootPreset {
    pub id: String,
    pub label: String,
    pub skill_preset: String,
    pub item_pool_preset: String,
    pub starting_state_preset: String,
    pub settings: Settings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillPreset {
    pub id: String,
    pub label: String,
    pub settings: SkillSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemPoolPreset {
    pub id: String,
    pub label: String,
    pub settings: ItemPoolSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartingStatePreset {
    pub id: String,
    pub label: String,
    pub settings: StartingState,
}

/// Complete captured baselines, including the root's inherited scope presets and any overrides.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresetBundle {
    pub root: RootPreset,
    pub skill: Vec<SkillPreset>,
    pub item_pool: Vec<ItemPoolPreset>,
    pub starting_state: Vec<StartingStatePreset>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsSource {
    pub document: SettingsDocument,
    pub source_revision: Option<String>,
    pub presets: PresetBundle,
}

pub struct ResolvedSettings {
    pub values: Settings,
    pub source: SettingsSource,
}
