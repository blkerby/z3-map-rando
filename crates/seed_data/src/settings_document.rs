use crate::{CustomizationSettings, ItemPoolSettings, Settings, SkillSettings, StartingState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Sparse, public settings: built-in selections and explicit tweaks, without user preset metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsDocument {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomizationPreset {
    pub id: String,
    pub label: String,
    pub settings: CustomizationSettings,
}

/// A built-in preset, identified within its scope.
pub trait Preset {
    fn id(&self) -> &str;
}

impl Preset for RootPreset {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Preset for SkillPreset {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Preset for ItemPoolPreset {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Preset for StartingStatePreset {
    fn id(&self) -> &str {
        &self.id
    }
}

impl Preset for CustomizationPreset {
    fn id(&self) -> &str {
        &self.id
    }
}

pub struct ResolvedSettings {
    pub values: Settings,
    pub document: SettingsDocument,
}
