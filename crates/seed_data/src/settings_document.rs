use crate::{Settings, SkillSettings};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use type_hash::TypeHash;

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

#[derive(Clone, Debug, Serialize, Deserialize, TypeHash)]
pub struct RootPreset {
    pub id: String,
    pub label: String,
    pub skill_preset: String,
    pub settings: Settings,
}

#[derive(Clone, Debug, Serialize, Deserialize, TypeHash)]
pub struct SkillPreset {
    pub id: String,
    pub label: String,
    pub settings: SkillSettings,
}

/// Complete captured baselines, including the root's inherited skill preset and any override.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresetBundle {
    pub root: RootPreset,
    pub skill: Vec<SkillPreset>,
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
    pub notices: Vec<String>,
}
