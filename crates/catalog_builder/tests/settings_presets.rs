use catalog_builder::{config::BuildConfig, logic, presets};
use seed_data::SettingsMode;
use serde_json::json;
use std::{fs::File, path::Path};

#[test]
fn validate_builtin_presets() -> anyhow::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut config: BuildConfig =
        toml::from_str(include_str!("../../../catalog-build.default.toml"))?;
    config.resolve_paths(&root);
    let output = tempfile::tempdir()?;
    config.output_directory = output.path().to_owned();
    logic::build_catalog(
        &config.logic_source,
        &config.items,
        &output.path().join("logic_catalog.bin"),
        config.compression_level,
    )?;
    let path = output.path().join("preset_catalog.bin");
    presets::build_catalog(&config, &path, String::new())?;
    let context = preset_catalog::decode_catalog(File::open(path)?)?;
    for root in &context.root {
        let mut document = json!({
            "preset": root.id,
        });
        let report = context.process_settings(&document, SettingsMode::Strict)?;
        assert!(
            report.notices.is_empty(),
            "{}: {:?}",
            root.id,
            report.notices
        );
        assert!(report.errors.is_empty(), "{}: {:?}", root.id, report.errors);
        assert_eq!(
            serde_json::to_value(report.finish_processing()?.values)?,
            serde_json::to_value(&root.settings)?
        );
        for skill in &context.skill {
            document["skill"] = json!({"preset": skill.id});
            let report = context.process_settings(&document, SettingsMode::Strict)?;
            assert!(
                report.notices.is_empty(),
                "{}/{}: {:?}",
                root.id,
                skill.id,
                report.notices
            );
            assert!(
                report.errors.is_empty(),
                "{}/{}: {:?}",
                root.id,
                skill.id,
                report.errors
            );
            let mut expected = root.settings.clone();
            expected.skill = skill.settings.clone();
            assert_eq!(
                serde_json::to_value(report.finish_processing()?.values)?,
                serde_json::to_value(expected)?
            );
        }
    }
    Ok(())
}
