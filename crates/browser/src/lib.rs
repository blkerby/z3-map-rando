use wasm_bindgen::prelude::*;

/// Format of settings documents authored by this browser build.
#[wasm_bindgen]
pub fn get_settings_format_version() -> u32 {
    seed_data::SETTINGS_FORMAT_VERSION
}
