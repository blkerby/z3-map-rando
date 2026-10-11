//! Settings and saved seed data shared by generation and patching.

pub mod output;
pub mod settings;

pub use output::{ItemPlacement, SEED_FORMAT_VERSION, SeedData, SeedMetadata};
pub use settings::{ItemPoolEntry, ProficiencySetting, Settings, TechSetting};
