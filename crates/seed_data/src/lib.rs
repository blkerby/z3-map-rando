//! Settings and saved seed data shared by generation and patching.

pub mod layout;
pub mod output;
pub mod settings;
pub mod settings_document;
pub mod settings_processing;

pub use layout::{OverworldRoomPlacement, WorldLayout};
pub use output::{BorderSide, ItemPlacement, MapBorder, RoomData, SeedData, SeedMetadata};
pub use settings::{
    CustomizationSettings, InsufficientPoolItems, ItemPoolSettings, MapLayoutSetting, Settings,
    SkillSettings, StartingBoomerang, StartingBottle, StartingBow, StartingEquipment,
    StartingGlove, StartingMail, StartingOcarina, StartingResources, StartingShield, StartingState,
    StartingSword,
};
pub use settings_document::{
    CustomizationPreset, ItemPoolPreset, Preset, ResolvedSettings, RootPreset, SettingsDocument,
    SkillPreset, StartingStatePreset,
};
pub use settings_processing::{
    SettingsContext, SettingsErrors, SettingsIssue, SettingsMode, SettingsReport,
};
