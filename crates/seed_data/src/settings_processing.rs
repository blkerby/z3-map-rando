use crate::{
    CustomizationPreset, CustomizationSettings, ItemPoolPreset, Preset, ResolvedSettings,
    RootPreset, SETTINGS_FORMAT_VERSION, Settings, SettingsDocument, SkillPreset,
    StartingStatePreset,
};
use anyhow::{Result, bail};
use common_types::Item;
use jsonschema::{Draft, Registry, Validator, error::ValidationErrorKind};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use serde_path_to_error::Segment;
use std::{collections::BTreeMap, sync::LazyLock};

static SETTINGS_VALIDATOR: LazyLock<Validator> = LazyLock::new(|| {
    let generation: Value =
        serde_json::from_str(include_str!("../../../schema/settings/generation.json")).unwrap();
    let skill: Value =
        serde_json::from_str(include_str!("../../../schema/settings/skill.json")).unwrap();
    let starting_state: Value =
        serde_json::from_str(include_str!("../../../schema/settings/starting_state.json")).unwrap();
    let registry = Registry::new()
        .add(generation["$id"].as_str().unwrap(), generation.clone())
        .unwrap()
        .add(skill["$id"].as_str().unwrap(), skill.clone())
        .unwrap()
        .add(
            starting_state["$id"].as_str().unwrap(),
            starting_state.clone(),
        )
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

static CUSTOMIZATION_VALIDATOR: LazyLock<Validator> = LazyLock::new(|| {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schema/settings/customization.json")).unwrap();
    jsonschema::options()
        .with_draft(Draft::Draft202012)
        .offline()
        .build(&schema)
        .expect("valid packaged customization schema")
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
    /// Identifies the processing code and prepared data: documents processed under the same
    /// revision give the same results.
    pub revision: String,
    /// Complete built-in baselines, the default first.
    pub root: Vec<RootPreset>,
    pub skill: Vec<SkillPreset>,
    pub item_pool: Vec<ItemPoolPreset>,
    pub starting_state: Vec<StartingStatePreset>,
    /// Built-in customization presets, the default first.
    pub customization: Vec<CustomizationPreset>,
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
    /// Blocking in both modes: an inconsistency in the prepared data rather than in the input.
    Block,
    /// A conflict between valid settings, such as starting ammunition above its capacity. It is
    /// left for the player to fix in both modes, and never makes a document unrecoverable.
    CrossSetting,
}

struct Finding {
    issue: SettingsIssue,
    recovery: Recovery,
}

/// The child scopes of a generation settings document, each with its own preset selection.
const CHILD_SCOPES: [&str; 3] = ["skill", "item_pool", "starting_state"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsReport<T> {
    /// Retained separately from the cleaned document until an explicit save.
    pub original: Value,
    /// Cleaned sparse document.
    pub document: SettingsDocument,
    pub values: Option<T>,
    pub notices: Vec<SettingsIssue>,
    /// Blocking findings. In recovery mode, these are only cross-setting findings.
    pub errors: Vec<SettingsIssue>,
}

impl SettingsReport<Settings> {
    /// Admit generation only when processing has no blocking errors.
    pub fn finish_processing(self) -> Result<ResolvedSettings, SettingsErrors> {
        if !self.errors.is_empty() {
            return Err(SettingsErrors(self.errors));
        }
        Ok(ResolvedSettings {
            values: self.values.unwrap(),
            document: self.document,
        })
    }
}

impl SettingsContext {
    /// Process a live document using current presets; pinned seed records use strict reading.
    ///
    /// In recovery mode the result can always be loaded: findings that per-setting repair cannot
    /// fix drop the tweaks of the child scopes they occur in, keeping their preset selections, and
    /// then all tweaks, keeping the root preset. Each step is reported. If findings remain even
    /// then, which indicates a bug, the document is unrecoverable and an error is returned.
    pub fn process_settings(
        &self,
        original: &Value,
        mode: SettingsMode,
    ) -> Result<SettingsReport<Settings>> {
        let (raw, mut issues) = read_document(original);
        let root = select_preset(
            &self.root,
            raw.get("preset"),
            &self.root[0],
            &["preset"],
            "root",
            &mut issues,
        );
        let inherited_skill = self
            .skill
            .iter()
            .find(|p| p.id == root.skill_preset)
            .unwrap();
        let skill = select_preset(
            &self.skill,
            raw.get("skill").and_then(|v| v.get("preset")),
            inherited_skill,
            &["skill", "preset"],
            "skill",
            &mut issues,
        );
        let inherited_item_pool = self
            .item_pool
            .iter()
            .find(|p| p.id == root.item_pool_preset)
            .unwrap();
        let item_pool = select_preset(
            &self.item_pool,
            raw.get("item_pool").and_then(|v| v.get("preset")),
            inherited_item_pool,
            &["item_pool", "preset"],
            "item pool",
            &mut issues,
        );
        let inherited_starting_state = self
            .starting_state
            .iter()
            .find(|p| p.id == root.starting_state_preset)
            .unwrap();
        let starting_state = select_preset(
            &self.starting_state,
            raw.get("starting_state").and_then(|v| v.get("preset")),
            inherited_starting_state,
            &["starting_state", "preset"],
            "starting state",
            &mut issues,
        );
        let (mut notices, errors) = route_issues(issues, mode);
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
        let starting_state_values = serde_json::to_value(&starting_state.settings)?;
        let selects_starting_state = raw
            .get("starting_state")
            .and_then(|value| value.get("preset"))
            .is_some();
        let overrides_starting_state =
            selects_starting_state && baseline["starting_state"] != starting_state_values;
        if selects_starting_state {
            baseline["starting_state"] = starting_state_values;
        }

        let check = |values: &Value| self.collect_findings(values);
        let (mut tweaks, mut outcome) =
            apply_tweaks::<Settings>(&raw, &baseline, mode, &check, &mut notices)?;
        if mode == SettingsMode::Recover && has_failures(&outcome) {
            let mut reduced = tweaks.clone();
            let mut dropped = false;
            for scope in CHILD_SCOPES {
                let fails = match &outcome {
                    Err(findings) => findings.iter().any(|finding| {
                        !matches!(finding.recovery, Recovery::CrossSetting)
                            && finding.issue.path.first().map(String::as_str) == Some(scope)
                    }),
                    Ok(_) => false,
                };
                if fails && reduced.as_object_mut().unwrap().remove(scope).is_some() {
                    notices.push(SettingsIssue {
                        path: vec![scope.to_owned()],
                        message: "These settings could not be read; their changes were removed."
                            .to_owned(),
                        replacement: None,
                    });
                    dropped = true;
                }
            }
            if dropped {
                (tweaks, outcome) = apply_tweaks(&reduced, &baseline, mode, &check, &mut notices)?;
            }
            if has_failures(&outcome) {
                (tweaks, outcome) = drop_all_tweaks(&baseline, &check, &mut notices)?;
            }
        }

        let mut report = SettingsReport {
            original: original.clone(),
            document: SettingsDocument {
                format_version: SETTINGS_FORMAT_VERSION,
                preset: root.id.clone(),
                tweaks: BTreeMap::new(),
            },
            values: None,
            notices,
            errors,
        };
        match outcome {
            Ok(settings) => {
                match settings.resolve_item_pool() {
                    Ok(items) => {
                        let mut count = 0_u64;
                        for &quantity in items.values() {
                            count += u64::from(quantity);
                        }
                        if count > u64::from(self.ordinary_item_count) {
                            report.errors.push(SettingsIssue {
                                path: vec!["item_pool".to_owned(), "items".to_owned()],
                                message: format!("item pool contains {count} items after deduction, but only {} ordinary locations are available", self.ordinary_item_count),
                                replacement: None,
                            });
                        }
                    }
                    Err(error) => {
                        report.errors.push(SettingsIssue {
                            path: vec![
                                "item_pool".to_owned(),
                                "items".to_owned(),
                                serde_json::to_value(error.item)?
                                    .as_str()
                                    .unwrap()
                                    .to_owned(),
                            ],
                            message: error.to_string(),
                            replacement: None,
                        });
                    }
                }
                report.values = Some(settings);
            }
            Err(findings) => {
                for finding in findings {
                    report.errors.push(finding.issue);
                }
            }
        }
        if skill.id != inherited_skill.id || overrides_skill {
            tweaks["skill"]["preset"] = json!(skill.id);
        }
        if item_pool.id != inherited_item_pool.id || overrides_item_pool {
            tweaks["item_pool"]["preset"] = json!(item_pool.id);
        }
        if starting_state.id != inherited_starting_state.id || overrides_starting_state {
            tweaks["starting_state"]["preset"] = json!(starting_state.id);
        }
        for (key, value) in tweaks.as_object().unwrap() {
            report.document.tweaks.insert(key.clone(), value.clone());
        }
        Ok(report)
    }

    /// Process a customization document using the current customization presets. Customization
    /// has only a root scope, so recovery falls back directly to the root preset without tweaks.
    pub fn process_customization(
        &self,
        original: &Value,
        mode: SettingsMode,
    ) -> Result<SettingsReport<CustomizationSettings>> {
        let (raw, mut issues) = read_document(original);
        let root = select_preset(
            &self.customization,
            raw.get("preset"),
            &self.customization[0],
            &["preset"],
            "customization",
            &mut issues,
        );
        let (mut notices, errors) = route_issues(issues, mode);
        let baseline = serde_json::to_value(&root.settings)?;
        let check = |values: &Value| {
            let mut findings = Vec::new();
            collect_schema_findings(&CUSTOMIZATION_VALIDATOR, values, &mut findings);
            findings
        };
        let (mut tweaks, mut outcome) = apply_tweaks(&raw, &baseline, mode, &check, &mut notices)?;
        if mode == SettingsMode::Recover && has_failures(&outcome) {
            (tweaks, outcome) = drop_all_tweaks(&baseline, &check, &mut notices)?;
        }
        let mut report = SettingsReport {
            original: original.clone(),
            document: SettingsDocument {
                format_version: SETTINGS_FORMAT_VERSION,
                preset: root.id.clone(),
                tweaks: BTreeMap::new(),
            },
            values: None,
            notices,
            errors,
        };
        match outcome {
            Ok(settings) => report.values = Some(settings),
            Err(findings) => {
                for finding in findings {
                    report.errors.push(finding.issue);
                }
            }
        }
        for (key, value) in tweaks.as_object().unwrap() {
            report.document.tweaks.insert(key.clone(), value.clone());
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
        let known =
            json!({"item_pool": {"items": &self.item_minimums}, "skill": &self.skill[0].settings});
        for mut path in [
            vec!["item_pool".to_owned(), "items".to_owned()],
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
            let path = vec!["item_pool".to_owned(), "items".to_owned(), key];
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
        for (amount, capacity) in [("bombs", "bomb_capacity"), ("arrows", "arrow_capacity")] {
            let resources = &values["starting_state"]["resources"];
            if let (Some(amount_value), Some(capacity_value)) =
                (resources[amount].as_u64(), resources[capacity].as_u64())
            {
                if amount_value > capacity_value {
                    append_finding(
                        &mut findings,
                        vec![
                            "starting_state".to_owned(),
                            "resources".to_owned(),
                            amount.to_owned(),
                        ],
                        format!("starting {amount} exceed {capacity}"),
                        Recovery::CrossSetting,
                    );
                }
            }
        }
        collect_schema_findings(&SETTINGS_VALIDATOR, values, &mut findings);
        findings
    }
}

/// Collect schema findings with their recovery kinds.
fn collect_schema_findings(validator: &Validator, values: &Value, findings: &mut Vec<Finding>) {
    for error in validator.iter_errors(values) {
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
                        findings,
                        child,
                        "Unknown setting.".to_owned(),
                        Recovery::Remove,
                    );
                }
            }
            ValidationErrorKind::Required { property } => {
                path.push(property.as_str().unwrap().to_owned());
                append_finding(
                    findings,
                    path,
                    "Required field missing after preset resolution.".to_owned(),
                    Recovery::Block,
                );
            }
            _ => append_finding(findings, path, error.to_string(), Recovery::Restore),
        }
    }
}

/// Normalize a document's numbers and check its shape and format version. A document that is not
/// an object is read as an empty one.
fn read_document(original: &Value) -> (Value, Vec<SettingsIssue>) {
    let mut raw = original.clone();
    normalize_numbers(&mut raw);
    let mut issues = Vec::new();
    if !raw.is_object() {
        issues.push(SettingsIssue {
            path: Vec::new(),
            message: "Expected a settings object.".to_owned(),
            replacement: Some(json!({})),
        });
        raw = json!({});
    }
    // Version 1 is the only authored format; there are no structural or vocabulary migrations.
    if raw.get("format_version").and_then(Value::as_u64) != Some(u64::from(SETTINGS_FORMAT_VERSION))
    {
        issues.push(SettingsIssue {
            path: vec!["format_version".to_owned()],
            message: "Unsupported settings format version.".to_owned(),
            replacement: Some(json!(SETTINGS_FORMAT_VERSION)),
        });
    }
    (raw, issues)
}

/// The preset a scope uses: the requested one, the inherited one when none is requested, or the
/// scope's first preset, with an issue, when the requested one is unknown.
fn select_preset<'a, P: Preset>(
    presets: &'a [P],
    requested: Option<&Value>,
    inherited: &'a P,
    path: &[&str],
    name: &str,
    issues: &mut Vec<SettingsIssue>,
) -> &'a P {
    let Some(id) = requested else {
        return inherited;
    };
    for preset in presets {
        if Some(preset.id()) == id.as_str() {
            return preset;
        }
    }
    let mut issue_path = Vec::new();
    for key in path {
        issue_path.push((*key).to_owned());
    }
    issues.push(SettingsIssue {
        path: issue_path,
        message: format!("Unknown {name} preset."),
        replacement: Some(json!(presets[0].id())),
    });
    &presets[0]
}

/// Split document-level issues into notices and errors: recovery reports them as notices, while
/// strict processing blocks without suggesting replacements.
fn route_issues(
    mut issues: Vec<SettingsIssue>,
    mode: SettingsMode,
) -> (Vec<SettingsIssue>, Vec<SettingsIssue>) {
    if mode == SettingsMode::Recover {
        return (issues, Vec::new());
    }
    for issue in &mut issues {
        issue.replacement = None;
    }
    (Vec::new(), issues)
}

/// Overlay a document's tweaks onto its baseline, repair them in recovery mode, then validate and
/// deserialize. Returns the remaining tweaks, normalized against the baseline, with either the
/// values or the findings that remain.
fn apply_tweaks<T: Serialize + DeserializeOwned>(
    raw: &Value,
    baseline: &Value,
    mode: SettingsMode,
    check: &dyn Fn(&Value) -> Vec<Finding>,
    notices: &mut Vec<SettingsIssue>,
) -> Result<(Value, Result<T, Vec<Finding>>)> {
    let mut values = baseline.clone();
    let mut tweaks = json!({});
    merge_tweaks(raw, baseline, &mut values, &mut tweaks, &mut Vec::new());
    let mut remaining = Vec::new();
    let mut repaired = false;
    for mut finding in check(&values) {
        let path = &finding.issue.path;
        if mode == SettingsMode::Recover && !path.is_empty() && get_value(&tweaks, path).is_some() {
            let replacement = match finding.recovery {
                Recovery::Remove => None,
                Recovery::Restore => get_value(baseline, path).cloned(),
                Recovery::Block | Recovery::CrossSetting => {
                    remaining.push(finding);
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
                notices.push(finding.issue);
                repaired = true;
                continue;
            }
        }
        remaining.push(finding);
    }
    if repaired {
        // A second validation is final; repair is not retried.
        remaining = check(&values);
    }
    if !remaining.is_empty() {
        return Ok((tweaks, Err(remaining)));
    }
    match serde_path_to_error::deserialize::<_, T>(&values) {
        Ok(settings) => {
            normalize_tweaks(&mut tweaks, baseline, &serde_json::to_value(&settings)?);
            Ok((tweaks, Ok(settings)))
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
            let finding = Finding {
                issue: SettingsIssue {
                    path,
                    message: error.inner().to_string(),
                    replacement: None,
                },
                recovery: Recovery::Block,
            };
            Ok((tweaks, Err(vec![finding])))
        }
    }
}

/// Do findings remain that the player cannot fix in the editor?
fn has_failures<T>(outcome: &Result<T, Vec<Finding>>) -> bool {
    match outcome {
        Ok(_) => false,
        Err(findings) => findings
            .iter()
            .any(|finding| !matches!(finding.recovery, Recovery::CrossSetting)),
    }
}

/// Last recovery step: keep only the selected presets. Fails if even those cannot be processed.
fn drop_all_tweaks<T: Serialize + DeserializeOwned>(
    baseline: &Value,
    check: &dyn Fn(&Value) -> Vec<Finding>,
    notices: &mut Vec<SettingsIssue>,
) -> Result<(Value, Result<T, Vec<Finding>>)> {
    notices.push(SettingsIssue {
        path: Vec::new(),
        message: "These settings could not be read; all changes were removed.".to_owned(),
        replacement: None,
    });
    let (tweaks, outcome) =
        apply_tweaks(&json!({}), baseline, SettingsMode::Recover, check, notices)?;
    if has_failures(&outcome) {
        bail!("settings could not be recovered");
    }
    Ok((tweaks, outcome))
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
        if (path.is_empty() && matches!(key.as_str(), "format_version" | "preset"))
            || (path.len() == 1 && CHILD_SCOPES.contains(&path[0].as_str()) && key == "preset")
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
