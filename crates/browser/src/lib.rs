use seed_data::SeedData;
use wasm_bindgen::prelude::*;

/// Format of settings documents authored by this browser build.
#[wasm_bindgen]
pub fn get_settings_format_version() -> u32 {
    seed_data::SETTINGS_FORMAT_VERSION
}

/// Patch a vanilla ROM with a saved seed (JSON) and the catalogs matching this build.
#[wasm_bindgen]
pub fn patch_rom(
    rom: &[u8],
    seed_json: &[u8],
    retiling_catalog: &[u8],
    patch_catalog: &[u8],
) -> Result<Vec<u8>, JsError> {
    let patch = || -> anyhow::Result<Vec<u8>> {
        let seed = SeedData::read(seed_json)?;
        let retiling = retiling_catalog::decode_catalog(retiling_catalog)?;
        let patches = patch_catalog::decode_catalog(patch_catalog)?;
        patcher::patch_rom(rom, &seed, &retiling, &patches, &Default::default())
    };
    patch().map_err(|error| JsError::new(&format!("{error:#}")))
}
