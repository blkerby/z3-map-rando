//! Settings and saved seed data shared by generation and patching.

pub mod output;
pub mod settings;
pub mod settings_document;

pub use output::{
    AreaLayout, ItemPlacement, MapBorder, OverworldRoomPlacement, SEED_FORMAT_VERSION, SeedData,
    SeedMetadata, WorldLayout,
};
pub use settings::{MapLayoutSetting, NotableSettings, Settings, SkillSettings, StartingItems};
pub use settings_document::{
    PresetBundle, ResolvedSettings, RootPreset, SETTINGS_FORMAT_VERSION, SettingsDocument,
    SettingsSource, SkillPreset,
};
