use crate::{
    ItemPoolPreset, PresetBundle, ResolvedSettings, RootPreset, SETTINGS_FORMAT_VERSION, Settings,
    SettingsDocument, SettingsSource, SkillPreset,
};
use anyhow::Result;
use common_types::Item;
use jsonschema::{Draft, Registry, Validator, error::ValidationErrorKind};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serde_path_to_error::Segment;
use std::{collections::BTreeMap, sync::LazyLock};

static SETTINGS_VALIDATOR: LazyLock<Validator> = LazyLock::new(|| {
    let generation: Value =
        serde_json::from_str(include_str!("../../../schema/settings/generation.json")).unwrap();
    let skill: Value =
        serde_json::from_str(include_str!("../../../schema/settings/skill.json")).unwrap();
    let registry = Registry::new()
        .add(generation["$id"].as_str().unwrap(), generation.clone())
        .unwrap()
        .add(skill["$id"].as_str().unwrap(), skill.clone())
        .unwrap()
        .prepare()
        .unwrap();
    jsonschema::options()
        .with_draft(Draft::Draft202012)
        .with_registry(&registry)
        .offline()
        .build(&generation)
        .expect("valid packaged settings schemas")
});

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsMode {
    #[default]
    Strict,
    Recover,
}

/// Prepared live-settings data, independent of catalog encoding and the host environment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsContext {
    pub revision: String,
    /// Complete built-in baselines, the default first.
    pub root: Vec<RootPreset>,
    pub skill: Vec<SkillPreset>,
    pub item_pool: Vec<ItemPoolPreset>,
    /// Permitted levels from the technique classifications, including unranked levels.
    pub proficiency_levels: BTreeMap<u32, Vec<u32>>,
    /// Ordinary item identities and minimum counts from the item records.
    pub item_minimums: BTreeMap<Item, u32>,
    pub ordinary_item_count: u32,
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

#[derive(Debug)]
pub struct SettingsErrors(pub Vec<SettingsIssue>);

impl std::fmt::Display for SettingsErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, issue) in self.0.iter().enumerate() {
            if index != 0 {
                f.write_str("\n")?;
            }
            write!(f, "{issue}")?;
        }
        Ok(())
    }
}

impl std::error::Error for SettingsErrors {}

#[derive(Clone, Copy)]
enum Recovery {
    Remove,
    Restore,
    Block,
}

struct Finding {
    issue: SettingsIssue,
    recovery: Recovery,
}

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
    pub fn finish_processing(self) -> Result<ResolvedSettings, SettingsErrors> {
        if !self.errors.is_empty() {
            return Err(SettingsErrors(self.errors));
        }
        Ok(ResolvedSettings {
            values: self.values.unwrap(),
            source: self.source,
        })
    }
}

impl SettingsContext {
    /// Process a live document using current presets; pinned seed records use strict reading.
    pub fn process_settings(&self, original: &Value, mode: SettingsMode) -> Result<SettingsReport> {
        let mut notices = Vec::new();
        let mut errors = Vec::new();
        let mut migrated = original.clone();
        normalize_numbers(&mut migrated);
        let empty = json!({});
        let raw = if migrated.is_object() {
            &migrated
        } else {
            errors.push(SettingsIssue {
                path: Vec::new(),
                message: "Expected a settings object.".to_owned(),
                replacement: Some(empty.clone()),
            });
            &empty
        };
        // Version 1 is the only authored format; there are no structural or vocabulary migrations.
        if raw.get("format_version").and_then(Value::as_u64)
            != Some(u64::from(SETTINGS_FORMAT_VERSION))
        {
            errors.push(SettingsIssue {
                path: vec!["format_version".to_owned()],
                message: "Unsupported settings format version.".to_owned(),
                replacement: Some(json!(SETTINGS_FORMAT_VERSION)),
            });
        }
        let source_revision = raw
            .get("revision")
            .and_then(Value::as_str)
            .map(str::to_owned);
        if source_revision.as_deref() != Some(&self.revision) {
            notices.push(SettingsIssue {
                path: Vec::new(),
                message: "Your settings have been updated for this version of the randomizer."
                    .to_owned(),
                replacement: None,
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
                    errors.push(SettingsIssue {
                        path: vec!["preset".to_owned()],
                        message: "Unknown root preset.".to_owned(),
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
                    errors.push(SettingsIssue {
                        path: vec!["skill".to_owned(), "preset".to_owned()],
                        message: "Unknown skill preset.".to_owned(),
                        replacement: Some(json!(self.skill[0].id)),
                    });
                    &self.skill[0]
                }
            },
            None => inherited_skill,
        };
        let inherited_item_pool = self
            .item_pool
            .iter()
            .find(|p| p.id == root.item_pool_preset)
            .unwrap();
        let item_pool = match raw.get("item_pool").and_then(|v| v.get("preset")) {
            Some(id) => match self
                .item_pool
                .iter()
                .find(|p| Some(p.id.as_str()) == id.as_str())
            {
                Some(preset) => preset,
                None => {
                    errors.push(SettingsIssue {
                        path: vec!["item_pool".to_owned(), "preset".to_owned()],
                        message: "Unknown item pool preset.".to_owned(),
                        replacement: Some(json!(self.item_pool[0].id)),
                    });
                    &self.item_pool[0]
                }
            },
            None => inherited_item_pool,
        };
        if mode == SettingsMode::Recover {
            notices.append(&mut errors);
        } else {
            for issue in &mut errors {
                issue.replacement = None;
            }
        }
        let mut baseline = serde_json::to_value(&root.settings)?;
        let skill_values = serde_json::to_value(&skill.settings)?;
        let selects_skill = raw
            .get("skill")
            .and_then(|value| value.get("preset"))
            .is_some();
        let overrides_skill = selects_skill && baseline["skill"] != skill_values;
        if selects_skill {
            baseline["skill"] = skill_values;
        }
        let item_pool_values = serde_json::to_value(&item_pool.settings)?;
        let selects_item_pool = raw
            .get("item_pool")
            .and_then(|value| value.get("preset"))
            .is_some();
        let overrides_item_pool = selects_item_pool && baseline["item_pool"] != item_pool_values;
        if selects_item_pool {
            baseline["item_pool"] = item_pool_values;
        }
        let mut values = baseline.clone();
        let mut tweaks = json!({});
        merge_tweaks(raw, &baseline, &mut values, &mut tweaks, &mut Vec::new());

        let mut skills = vec![inherited_skill.clone()];
        if skill.id != inherited_skill.id {
            skills.push(skill.clone());
        }
        let mut item_pools = vec![inherited_item_pool.clone()];
        if item_pool.id != inherited_item_pool.id {
            item_pools.push(item_pool.clone());
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
                    item_pool: item_pools,
                },
            },
            values: None,
            notices,
            errors,
        };
        let findings = self.collect_findings(&values);
        let mut repaired = false;
        for mut finding in findings {
            let path = &finding.issue.path;
            if mode == SettingsMode::Recover
                && !path.is_empty()
                && get_value(&tweaks, path).is_some()
            {
                let replacement = match finding.recovery {
                    Recovery::Remove => None,
                    Recovery::Restore => get_value(&baseline, path).cloned(),
                    Recovery::Block => {
                        report.errors.push(finding.issue);
                        continue;
                    }
                };
                if matches!(finding.recovery, Recovery::Remove) || replacement.is_some() {
                    let mut parent = &mut values;
                    for key in &path[..path.len() - 1] {
                        parent = &mut parent[key];
                    }
                    let key = path.last().unwrap();
                    match &replacement {
                        Some(value) => {
                            parent[key] = value.clone();
                        }
                        None => {
                            parent.as_object_mut().unwrap().remove(key);
                        }
                    }
                    remove_tweak(&mut tweaks, path);
                    finding.issue.replacement = replacement;
                    if matches!(finding.recovery, Recovery::Remove) {
                        finding.issue.message = "Unknown setting; removed.".to_owned();
                    }
                    report.notices.push(finding.issue);
                    repaired = true;
                    continue;
                }
            }
            report.errors.push(finding.issue);
        }
        if repaired {
            // A second validation is blocking; recovery never retries deserialization.
            report.errors.clear();
            for finding in self.collect_findings(&values) {
                report.errors.push(finding.issue);
            }
        }
        if report.errors.is_empty() {
            match serde_path_to_error::deserialize::<_, Settings>(&values) {
                Ok(settings) => {
                    let mut count = 0_u64;
                    for &quantity in settings.item_pool.values() {
                        count += u64::from(quantity);
                    }
                    if count > u64::from(self.ordinary_item_count) {
                        report.errors.push(SettingsIssue {
                            path: vec!["item_pool".to_owned()],
                            message: format!("item pool contains {count} items, but only {} ordinary locations are available", self.ordinary_item_count),
                            replacement: None,
                        });
                    }
                    normalize_tweaks(&mut tweaks, &baseline, &serde_json::to_value(&settings)?);
                    report.values = Some(settings);
                }
                Err(error) => {
                    let mut path = Vec::new();
                    for segment in error.path() {
                        match segment {
                            Segment::Map { key } => path.push(key.clone()),
                            Segment::Enum { variant } => path.push(variant.clone()),
                            Segment::Seq { index } => path.push(index.to_string()),
                            Segment::Unknown => break,
                        }
                    }
                    report.errors.push(SettingsIssue {
                        path,
                        message: error.inner().to_string(),
                        replacement: None,
                    });
                }
            }
        }
        if skill.id != inherited_skill.id || overrides_skill {
            tweaks["skill"]["preset"] = json!(skill.id);
        }
        if item_pool.id != inherited_item_pool.id || overrides_item_pool {
            tweaks["item_pool"]["preset"] = json!(item_pool.id);
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

    /// Normalize numeric representations and strictly validate complete JSON before deserialization.
    pub fn validate_settings(&self, values: &mut Value) -> Result<(), SettingsErrors> {
        normalize_numbers(values);
        let mut errors = Vec::new();
        for finding in self.collect_findings(values) {
            errors.push(finding.issue);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(SettingsErrors(errors))
        }
    }

    fn collect_findings(&self, values: &Value) -> Vec<Finding> {
        let mut findings = Vec::new();
        let known = json!({"item_pool": &self.item_minimums, "skill": &self.skill[0].settings});
        for mut path in [
            vec!["item_pool".to_owned()],
            vec!["skill".to_owned(), "proficiencies".to_owned()],
            vec!["skill".to_owned(), "tech".to_owned()],
            vec![
                "skill".to_owned(),
                "notables".to_owned(),
                "overworld".to_owned(),
            ],
            vec![
                "skill".to_owned(),
                "notables".to_owned(),
                "underworld".to_owned(),
            ],
        ] {
            if let (Some(value), Some(known)) = (get_value(values, &path), get_value(&known, &path))
            {
                collect_unknown_entries(value, known, &mut path, &mut findings);
            }
        }
        for (id, levels) in &self.proficiency_levels {
            let path = vec![
                "skill".to_owned(),
                "proficiencies".to_owned(),
                id.to_string(),
            ];
            if let Some(level) = get_value(values, &path).and_then(Value::as_u64) {
                if !levels.iter().any(|&allowed| u64::from(allowed) == level) {
                    findings.push(Finding {
                        issue: SettingsIssue {
                            path,
                            message: format!("expected one of the proficiency levels {levels:?}"),
                            replacement: None,
                        },
                        recovery: Recovery::Restore,
                    });
                }
            }
        }
        for (item, minimum) in &self.item_minimums {
            let key = serde_json::to_value(item)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned();
            let path = vec!["item_pool".to_owned(), key];
            if let Some(count) = get_value(values, &path).and_then(Value::as_u64) {
                if count < u64::from(*minimum) {
                    findings.push(Finding {
                        issue: SettingsIssue {
                            path,
                            message: format!("expected an item count of at least {minimum}"),
                            replacement: None,
                        },
                        recovery: Recovery::Restore,
                    });
                }
            }
        }
        for error in SETTINGS_VALIDATOR.iter_errors(values) {
            let mut path = Vec::new();
            // The segment iterator canonicalizes numeric-looking object keys such as "05".
            for segment in error.instance_path().as_str().split('/').skip(1) {
                path.push(segment.replace("~1", "/").replace("~0", "~"));
            }
            match error.kind() {
                ValidationErrorKind::AdditionalProperties { unexpected } => {
                    for key in unexpected {
                        let mut child = path.clone();
                        child.push(key.clone());
                        append_finding(
                            &mut findings,
                            child,
                            "Unknown setting.".to_owned(),
                            Recovery::Remove,
                        );
                    }
                }
                ValidationErrorKind::Required { property } => {
                    path.push(property.as_str().unwrap().to_owned());
                    append_finding(
                        &mut findings,
                        path,
                        "Required field missing after preset resolution.".to_owned(),
                        Recovery::Block,
                    );
                }
                _ => append_finding(&mut findings, path, error.to_string(), Recovery::Restore),
            }
        }
        findings
    }
}

fn collect_unknown_entries(
    values: &Value,
    known: &Value,
    path: &mut Vec<String>,
    findings: &mut Vec<Finding>,
) {
    let Some(object) = values.as_object() else {
        return;
    };
    for (key, value) in object {
        path.push(key.clone());
        match known.get(key) {
            None => append_finding(
                findings,
                path.clone(),
                "Unknown setting.".to_owned(),
                Recovery::Remove,
            ),
            Some(default) if default.is_object() => {
                collect_unknown_entries(value, default, path, findings)
            }
            _ => {}
        }
        path.pop();
    }
}

fn append_finding(
    findings: &mut Vec<Finding>,
    path: Vec<String>,
    message: String,
    recovery: Recovery,
) {
    // Suppress bad values beneath an unknown entry and duplicate keyword failures at one path.
    if findings
        .iter()
        .any(|finding| path.starts_with(&finding.issue.path))
    {
        return;
    }
    findings.push(Finding {
        issue: SettingsIssue {
            path,
            message,
            replacement: None,
        },
        recovery,
    });
}

fn normalize_numbers(value: &mut Value) {
    match value {
        Value::Number(number) if number.is_f64() => {
            let value_number = number.as_f64().unwrap();
            if value_number.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&value_number) {
                *value = json!(value_number as u32);
            }
        }
        Value::Object(object) => {
            for child in object.values_mut() {
                normalize_numbers(child);
            }
        }
        Value::Array(array) => {
            for child in array {
                normalize_numbers(child);
            }
        }
        _ => {}
    }
}

fn merge_tweaks(
    raw: &Value,
    baseline: &Value,
    values: &mut Value,
    tweaks: &mut Value,
    path: &mut Vec<String>,
) {
    for (key, value) in raw.as_object().unwrap() {
        if (path.is_empty() && matches!(key.as_str(), "format_version" | "revision" | "preset"))
            || (path.len() == 1
                && matches!(path[0].as_str(), "skill" | "item_pool")
                && key == "preset")
        {
            continue;
        }
        path.push(key.clone());
        if let Some(default) = baseline.get(key) {
            if value.is_object() && default.is_object() {
                let mut child = json!({});
                merge_tweaks(value, default, &mut values[key], &mut child, path);
                if !child.as_object().unwrap().is_empty() {
                    tweaks[key] = child;
                }
            } else if value != default {
                values[key] = value.clone();
                tweaks[key] = value.clone();
            }
        } else {
            values[key] = value.clone();
            tweaks[key] = value.clone();
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
