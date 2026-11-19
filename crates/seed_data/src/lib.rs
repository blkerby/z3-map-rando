//! Settings and saved seed data shared by generation and patching.

pub mod layout;
pub mod output;
pub mod settings;
pub mod settings_document;
pub mod settings_processing;

pub use layout::{AreaLayout, OverworldRoomPlacement, WorldLayout};
pub use output::{
    BorderSide, ItemPlacement, MapBorder, RoomData, SEED_FORMAT_VERSION, SeedData, SeedMetadata,
};
pub use settings::{
    InsufficientPoolItems, ItemPoolSettings, MapLayoutSetting, NotableSettings, Settings,
    SkillSettings, StartingBoomerang, StartingBow, StartingEquipment, StartingGlove,
    StartingMail, StartingOcarina, StartingResources, StartingShield, StartingState, StartingSword,
};
pub use settings_document::{
    ItemPoolPreset, ResolvedSettings, RootPreset, SETTINGS_FORMAT_VERSION, SettingsDocument,
    SkillPreset, StartingStatePreset,
};
pub use settings_processing::{
    SettingsContext, SettingsErrors, SettingsIssue, SettingsMode, SettingsReport,
};
