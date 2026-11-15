use crate::{
    PresetBundle, ResolvedSettings, RootPreset, SETTINGS_FORMAT_VERSION, Settings,
    SettingsDocument, SettingsSource, SkillPreset,
};
use anyhow::Result;
use common_types::Item;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serde_path_to_error::Segment;
use std::collections::BTreeMap;

/// Prepared live-settings data, independent of catalog encoding and the host environment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsContext {
    pub revision: String,
    /// Complete built-in baselines, the default first.
    pub root: Vec<RootPreset>,
    pub skill: Vec<SkillPreset>,
    /// Permitted levels from the technique classifications, including unranked levels.
    pub proficiency_levels: BTreeMap<u32, Vec<u32>>,
    /// Ordinary item identities and minimum counts from the item records.
    pub item_minimums: BTreeMap<Item, u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsIssue {
    pub path: Vec<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<Value>,
}

impl std::fmt::Display for SettingsIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.path.is_empty() {
            write!(f, "{}: ", self.path.join("."))?;
        }
        f.write_str(&self.message)
    }
}

impl std::error::Error for SettingsIssue {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsReport {
    /// Retained separately from the cleaned document until an explicit save.
    pub original: Value,
    /// Cleaned sparse document, provenance, and captured preset baselines.
    pub source: SettingsSource,
    pub values: Option<Settings>,
    pub notices: Vec<SettingsIssue>,
    pub errors: Vec<SettingsIssue>,
}

impl SettingsReport {
    /// Admit generation only when processing has no blocking errors.
    pub fn finish_processing(self) -> Result<ResolvedSettings, SettingsIssue> {
        if let Some(error) = self.errors.into_iter().next() {
            return Err(error);
        }
        Ok(ResolvedSettings {
            values: self.values.unwrap(),
            source: self.source,
        })
    }
}

impl SettingsContext {
    /// Process a live document using current presets; pinned seed records use strict reading.
    pub fn process_settings(&self, original: &Value) -> Result<SettingsReport> {
        let mut notices = Vec::new();
        let empty = json!({});
        let raw = if original.is_object() {
            original
        } else {
            notices.push(SettingsIssue {
                path: Vec::new(),
                message: "Settings could not be read; using the default preset.".to_owned(),
                replacement: Some(empty.clone()),
            });
            &empty
        };
        // Version 1 is the only authored format; there are no structural or vocabulary migrations.
        if raw.get("format_version").and_then(Value::as_u64)
            != Some(u64::from(SETTINGS_FORMAT_VERSION))
        {
            notices.push(SettingsIssue {
                path: vec!["format_version".to_owned()],
                message:
                    "Settings format differs from the current version; using recognized settings."
                        .to_owned(),
                replacement: Some(json!(SETTINGS_FORMAT_VERSION)),
            });
        }
        let source_revision = raw
            .get("revision")
            .and_then(Value::as_str)
            .map(str::to_owned);
        if source_revision.as_deref() != Some(&self.revision) {
            notices.push(SettingsIssue {
                path: vec!["revision".to_owned()],
                message: "Settings were updated to the current rules and built-in presets."
                    .to_owned(),
                replacement: Some(json!(self.revision)),
            });
        }
        let root = match raw.get("preset") {
            Some(id) => match self
                .root
                .iter()
                .find(|p| Some(p.id.as_str()) == id.as_str())
            {
                Some(preset) => preset,
                None => {
                    notices.push(SettingsIssue {
                        path: vec!["preset".to_owned()],
                        message: "Unknown root preset; using the default preset.".to_owned(),
                        replacement: Some(json!(self.root[0].id)),
                    });
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
                    notices.push(SettingsIssue {
                        path: vec!["skill".to_owned(), "preset".to_owned()],
                        message: "Unknown skill preset; using the default skill preset.".to_owned(),
                        replacement: Some(json!(self.skill[0].id)),
                    });
                    &self.skill[0]
                }
            },
            None => inherited_skill,
        };
        let mut baseline = serde_json::to_value(&root.settings)?;
        baseline["skill"] = serde_json::to_value(&skill.settings)?;
        let mut values = baseline.clone();
        let mut tweaks = json!({});
        merge_tweaks(
            raw,
            &baseline,
            &mut values,
            &mut tweaks,
            &mut Vec::new(),
            &mut notices,
        );

        let mut skills = vec![inherited_skill.clone()];
        if skill.id != inherited_skill.id {
            skills.push(skill.clone());
        }
        let mut report = SettingsReport {
            original: original.clone(),
            source: SettingsSource {
                document: SettingsDocument {
                    format_version: SETTINGS_FORMAT_VERSION,
                    revision: self.revision.clone(),
                    preset: root.id.clone(),
                    tweaks: BTreeMap::new(),
                },
                source_revision,
                presets: PresetBundle {
                    root: root.clone(),
                    skill: skills,
                },
            },
            values: None,
            notices,
            errors: Vec::new(),
        };
        loop {
            let mut issue = match serde_path_to_error::deserialize::<_, Settings>(&values) {
                Ok(settings) => match self.validate_settings(&settings) {
                    Some(issue) => issue,
                    None => {
                        normalize_tweaks(&mut tweaks, &baseline, &serde_json::to_value(&settings)?);
                        report.values = Some(settings);
                        break;
                    }
                },
                Err(error) => {
                    let mut path = Vec::new();
                    // Enum map keys are opaque to path tracking; string keys retain the item token.
                    let error = if values["item_pool"].is_object()
                        && matches!(error.path().iter().next(), Some(Segment::Map { key }) if key == "item_pool")
                    {
                        match serde_path_to_error::deserialize::<_, BTreeMap<String, u32>>(
                            &values["item_pool"],
                        ) {
                            Err(error) => {
                                path.push("item_pool".to_owned());
                                error
                            }
                            Ok(_) => error,
                        }
                    } else {
                        error
                    };
                    for segment in error.path() {
                        match segment {
                            Segment::Map { key } => path.push(key.clone()),
                            Segment::Enum { variant } => path.push(variant.clone()),
                            Segment::Seq { index } => path.push(index.to_string()),
                            Segment::Unknown => break,
                        }
                    }
                    SettingsIssue {
                        path,
                        message: error.inner().to_string(),
                        replacement: None,
                    }
                }
            };
            // Only an explicitly submitted leaf or malformed container can be recovered.
            let mut path = issue.path.clone();
            let replacement = loop {
                if path.is_empty() {
                    break None;
                }
                if let (Some(submitted), Some(default)) =
                    (get_value(&tweaks, &path), get_value(&baseline, &path))
                {
                    if !submitted.is_object() || !default.is_object() {
                        break Some(default.clone());
                    }
                }
                path.pop();
            };
            let Some(replacement) = replacement else {
                report.errors.push(issue);
                break;
            };
            let mut node = &mut values;
            for key in &path[..path.len() - 1] {
                node = &mut node[key];
            }
            node[path.last().unwrap()] = replacement.clone();
            remove_tweak(&mut tweaks, &path);
            issue.path = path;
            issue.replacement = Some(replacement);
            report.notices.push(issue);
        }
        if skill.id != inherited_skill.id {
            tweaks["skill"]["preset"] = json!(skill.id);
        }
        for (key, value) in tweaks.as_object().unwrap() {
            report
                .source
                .document
                .tweaks
                .insert(key.clone(), value.clone());
        }
        Ok(report)
    }

    fn validate_settings(&self, settings: &Settings) -> Option<SettingsIssue> {
        for (name, capacity) in [
            ("bomb_capacity", settings.starting_items.bomb_capacity),
            ("arrow_capacity", settings.starting_items.arrow_capacity),
        ] {
            if capacity > 99 {
                return Some(SettingsIssue {
                    path: vec!["starting_items".to_owned(), name.to_owned()],
                    message: "expected a capacity from 0 to 99".to_owned(),
                    replacement: None,
                });
            }
        }
        for (item, minimum) in &self.item_minimums {
            if settings.item_pool[item] < *minimum {
                return Some(SettingsIssue {
                    path: vec![
                        "item_pool".to_owned(),
                        serde_json::to_value(item)
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_owned(),
                    ],
                    message: format!("expected an item count of at least {minimum}"),
                    replacement: None,
                });
            }
        }
        for (id, level) in &settings.skill.proficiencies {
            if !self.proficiency_levels[id].contains(level) {
                return Some(SettingsIssue {
                    path: vec![
                        "skill".to_owned(),
                        "proficiencies".to_owned(),
                        id.to_string(),
                    ],
                    message: format!(
                        "expected one of the proficiency levels {:?}",
                        self.proficiency_levels[id]
                    ),
                    replacement: None,
                });
            }
        }
        None
    }
}

fn merge_tweaks(
    raw: &Value,
    baseline: &Value,
    values: &mut Value,
    tweaks: &mut Value,
    path: &mut Vec<String>,
    notices: &mut Vec<SettingsIssue>,
) {
    for (key, value) in raw.as_object().unwrap() {
        if (path.is_empty() && matches!(key.as_str(), "format_version" | "revision" | "preset"))
            || (path.as_slice() == ["skill"] && key == "preset")
        {
            continue;
        }
        path.push(key.clone());
        if let Some(default) = baseline.get(key) {
            if value.is_object() && default.is_object() {
                let mut child = json!({});
                merge_tweaks(value, default, &mut values[key], &mut child, path, notices);
                if !child.as_object().unwrap().is_empty() {
                    tweaks[key] = child;
                }
            } else if value != default {
                values[key] = value.clone();
                tweaks[key] = value.clone();
            }
        } else {
            notices.push(SettingsIssue {
                path: path.clone(),
                message: "Unknown setting; ignored.".to_owned(),
                replacement: None,
            });
        }
        path.pop();
    }
}

fn get_value<'a>(root: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut node = root;
    for key in path {
        node = node.get(key)?;
    }
    Some(node)
}

fn normalize_tweaks(tweaks: &mut Value, baseline: &Value, values: &Value) {
    tweaks.as_object_mut().unwrap().retain(|key, tweak| {
        let value = &values[key];
        let default = &baseline[key];
        if value == default {
            return false;
        }
        if value.is_object() && default.is_object() {
            normalize_tweaks(tweak, default, value);
            !tweak.as_object().unwrap().is_empty()
        } else {
            *tweak = value.clone();
            true
        }
    });
}

fn remove_tweak(root: &mut Value, path: &[String]) {
    let object = root.as_object_mut().unwrap();
    if path.len() == 1 {
        object.remove(&path[0]);
    } else {
        let child = object.get_mut(&path[0]).unwrap();
        remove_tweak(child, &path[1..]);
        if child.as_object().unwrap().is_empty() {
            object.remove(&path[0]);
        }
    }
}
