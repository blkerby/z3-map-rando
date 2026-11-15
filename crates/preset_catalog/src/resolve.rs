use crate::PresetCatalog;
use anyhow::Result;
use seed_data::{
    PresetBundle, ResolvedSettings, SETTINGS_FORMAT_VERSION, SettingsDocument, SettingsSource,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Semantic value rules exported to both the backend and frontend.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingDefinition {
    pub path: Vec<String>,
    pub values: SettingValues,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SettingValues {
    Boolean,
    Integer { min: u32, max: u32 },
    Choice(Vec<String>),
    Levels(Vec<u32>),
}

impl SettingValues {
    fn accepts(&self, value: &Value) -> bool {
        match self {
            Self::Boolean => value.is_boolean(),
            Self::Integer { min, max } => value
                .as_u64()
                .is_some_and(|v| v >= u64::from(*min) && v <= u64::from(*max)),
            Self::Choice(options) => value
                .as_str()
                .is_some_and(|v| options.iter().any(|o| o == v)),
            Self::Levels(levels) => value
                .as_u64()
                .is_some_and(|v| levels.iter().any(|l| u64::from(*l) == v)),
        }
    }
}

impl PresetCatalog {
    /// Load live settings against current built-ins, returning every fallback as a notice.
    /// Version 1 is the only authored format; newer inputs retain their recognized entries.
    pub fn resolve_settings(&self, raw: &Value) -> Result<ResolvedSettings> {
        let mut notices = Vec::new();
        let raw = if raw.is_object() {
            raw.clone()
        } else {
            notices.push("Settings could not be read; using the default preset.".to_owned());
            json!({})
        };
        if raw.get("format_version").and_then(Value::as_u64)
            != Some(u64::from(SETTINGS_FORMAT_VERSION))
        {
            notices.push(
                "Settings format differs from the current version; using recognized settings."
                    .to_owned(),
            );
        }
        let source_revision = raw
            .get("revision")
            .and_then(Value::as_str)
            .map(str::to_owned);
        if source_revision.as_deref() != Some(&self.revision) {
            notices.push(
                "Settings were updated to the current definitions and built-in presets.".to_owned(),
            );
        }

        let root = match raw.get("preset") {
            Some(id) => match self
                .root
                .iter()
                .find(|p| Some(p.id.as_str()) == id.as_str())
            {
                Some(preset) => preset,
                None => {
                    notices.push("Unknown root preset; using the default preset.".to_owned());
                    &self.root[0]
                }
            },
            None => &self.root[0],
        };
        let inherited_skill = self
            .skill
            .iter()
            .find(|p| p.id == root.skill_preset)
            .unwrap();
        let skill = match raw.get("skill").and_then(|v| v.get("preset")) {
            Some(id) => match self
                .skill
                .iter()
                .find(|p| Some(p.id.as_str()) == id.as_str())
            {
                Some(preset) => preset,
                None => {
                    notices
                        .push("Unknown skill preset; using the default skill preset.".to_owned());
                    &self.skill[0]
                }
            },
            None => inherited_skill,
        };
        let mut values = serde_json::to_value(&root.settings)?;
        values["skill"] = serde_json::to_value(&skill.settings)?;
        let mut document = json!({
            "format_version": SETTINGS_FORMAT_VERSION,
            "revision": self.revision,
            "preset": root.id,
        });
        if skill.id != inherited_skill.id {
            document["skill"] = json!({ "preset": skill.id });
        }
        apply_tweaks(
            self,
            &raw,
            &mut Vec::new(),
            &mut values,
            &mut document,
            &mut notices,
        );

        let mut skills = vec![inherited_skill.clone()];
        if skill.id != inherited_skill.id {
            skills.push(skill.clone());
        }
        Ok(ResolvedSettings {
            values: serde_json::from_value(values)?,
            source: SettingsSource {
                document: serde_json::from_value::<SettingsDocument>(document)?,
                source_revision,
                presets: PresetBundle {
                    root: root.clone(),
                    skill: skills,
                },
            },
            notices,
        })
    }
}

fn apply_tweaks(
    catalog: &PresetCatalog,
    raw: &Value,
    path: &mut Vec<String>,
    values: &mut Value,
    document: &mut Value,
    notices: &mut Vec<String>,
) {
    for (key, value) in raw.as_object().unwrap() {
        if (path.is_empty() && matches!(key.as_str(), "format_version" | "revision" | "preset"))
            || (path.as_slice() == ["skill"] && key == "preset")
        {
            continue;
        }
        path.push(key.clone());
        let definition = catalog.definitions.iter().find(|d| d.path == *path);
        if let Some(definition) = definition {
            if !definition.values.accepts(value) {
                notices.push(format!(
                    "Invalid value for {}; using its preset value.",
                    path.join("/")
                ));
            } else if get_value(values, path) != Some(value) {
                set_value(values, path, value.clone());
                set_value(document, path, value.clone());
            }
        } else if value.is_object() && get_value(values, path).is_some_and(Value::is_object) {
            apply_tweaks(catalog, value, path, values, document, notices);
        } else {
            notices.push(format!(
                "Unknown or unreadable setting {}; ignored.",
                path.join("/")
            ));
        }
        path.pop();
    }
}

fn set_value(root: &mut Value, path: &[String], value: Value) {
    let mut node = root;
    for key in &path[..path.len() - 1] {
        if node.get(key).is_none() {
            node[key] = json!({});
        }
        node = &mut node[key];
    }
    node[path.last().unwrap()] = value;
}

fn get_value<'a>(root: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut node = root;
    for key in path {
        node = node.get(key)?;
    }
    Some(node)
}
