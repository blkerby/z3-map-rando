//! Settings and saved seed data shared by generation and patching.

pub mod output;
pub mod settings;
pub mod settings_document;
pub mod settings_processing;

pub use output::{
    AreaLayout, ItemPlacement, MapBorder, OverworldRoomPlacement, SEED_FORMAT_VERSION, SeedData,
    SeedMetadata, WorldLayout,
};
pub use settings::{
    InsufficientPoolItems, ItemPoolSettings, MapLayoutSetting, NotableSettings, Settings,
    SkillSettings, StartingBoomerang, StartingBow, StartingEquipment, StartingFlags, StartingGlove,
    StartingMail, StartingOcarina, StartingResources, StartingShield, StartingState, StartingSword,
};
pub use settings_document::{
    ItemPoolPreset, PresetBundle, ResolvedSettings, RootPreset, SETTINGS_FORMAT_VERSION,
    SettingsDocument, SettingsSource, SkillPreset, StartingStatePreset,
};
pub use settings_processing::{
    SettingsContext, SettingsErrors, SettingsIssue, SettingsMode, SettingsReport,
};
