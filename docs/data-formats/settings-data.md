# Shared settings data

Status: draft. The frontend owns presentation through Svelte components. It bundles the technique, notable, proficiency, default, and preset data it needs at build time, using the same sources as the backend. A server-provided settings catalog or master UI description is not part of the interface.

## Ownership

| Owner | Responsibilities |
| --- | --- |
| `z3-json-data` | Technique and notable identities, upstream names, proficiency tiers, and underlying logic data |
| Randomizer | Ordinary setting identities, value semantics, defaults, option tokens, scopes, difficulty classifications, built-in presets, migrations, and authoritative validation |
| Frontend | Svelte components, section layout, navigation, controls, help, and presentation labels, using upstream names where appropriate |

Shared data describes game and settings semantics rather than widgets or layout. The frontend can generate lists of technique, notable, and proficiency controls from that data without hard-coding each member. Ordinary settings can be presented directly in components.

## Repository and build

The frontend lives at `z3-map-rando/frontend`, alongside the backend and its shared data sources in `data/`, configured in `catalog-build.toml`:

| Source | Contents |
| --- | --- |
| `schema/settings/` | Separately maintained JSON schemas for resolved generation values, skill assumptions, and starting state |
| `data/presets/` | Built-in preset files (see [Preset files](#preset-files)) |
| `data/tech_data.json`, `data/notable_data.json` | Difficulty classifications of techniques, proficiency levels, and notables; skill presets are derived from them |
| `data/items.json` | The item token vocabulary, with upstream item names, count limits, category, and placement tier; items in the `dungeon` and `prize` categories have their own placement and are not part of the item pool |

The preset directory is:

```text
data/
  presets/
    generation/
      root/
        default.json
        testing.json
      item_pool/
        default.json
      starting_state/
        default.json
        all_equipment.json
    customization/
      root/
        default.json
```

Skill presets have no files: they are derived from the difficulty classifications. Skill settings not covered by the classifications would get sparse files under `generation/skill/`, holding only those settings, with the derived members filled in by preparation.

Rust's `seed_data::Settings` types define setting representations and option tokens. `SettingsContext::process_settings` applies prepared presets, checks values, and returns a report with the preserved original, cleaned sparse document, resolved values, captured baselines, structured notices, and blocking errors. The context uses prepared proficiency levels, ordinary-item minimums, and the ordinary-location count derived from the logic catalog; derived skill presets and the ordinary-item vocabulary supply the known dynamic member identities independently of authored root presets. The processor embeds the schema resources and compiles one reusable offline validator per process or WASM module. Schema structure and Rust semantic verification both run before the single typed deserialization. There is no separate per-setting definition table. Svelte components own the controls and presentation.

```text
shared setting definitions + preset files + local classifications + upstream data
  -> common semantic preparation
  -> backend data and frontend build inputs

frontend build inputs + Svelte components
  -> frontend application with bundled data
```

The common preparation step is `catalog_builder presets` (`crates/catalog_builder/src/presets.rs`). It first prepares the logic catalog for the ordinary-location count, derives the skill presets, resolves each generation root preset into complete JSON values, validates them against the schemas and prepared semantic facts before deserializing into Rust settings, and writes two outputs from the same prepared data: the preset catalog (`preset_catalog.bin`) for the backend, and `settings_data.json` for the frontend. The latter also carries what the frontend shows about each setting: the difficulty ladder, the techniques, proficiencies, and notables with their upstream names and difficulties, the item records, and the customization presets. The frontend's Vite config runs the step whenever it loads and reruns it in dev when `data/` changes, so building or developing the frontend requires the Rust toolchain but no running backend.

Common preparation handles semantic work such as deriving skill presets and completing defaults consistently. The frontend can select and reshape the data it needs without a universal payload or UI-description format. Preparing these build inputs does not require a running backend; generation and other server-dependent features still do. Metadata preparation should not require compiling unrelated ROM assets.

## Preset files

Each built-in preset has its own source file, grouped by settings context and scope. The directory identifies its scope. A file contains its scope-local `id`, display `label`, and `settings` in the [scoped settings format](settings.md#format). Paths and filenames are source organization rather than persisted references. For example, `generation/root/default.json` contains:

```json
{
  "id": "default",
  "label": "Default",
  "settings": {
    "map_layout": "random",
    "skill": {
      "preset": "basic"
    },
    "item_pool": { "preset": "default" },
    "starting_state": { "preset": "default" }
  }
}
```

A root preset selects skill, item pool, and starting state presets through its nested `skill`, `item_pool`, and `starting_state` objects; a child preset's own values are directly inside its `settings` object. The item pool Default preset is authored in `generation/item_pool/default.json`; starting-state Default and All equipment presets are authored under `generation/starting_state/`. Item pool settings contain pickup counts in `items`, an equipment-deduction toggle, and bomb/arrow upgrade sizes. User-defined presets remain root-only and based directly on a built-in root preset.

Fixed Rust settings structs and authored preset-file wrappers reject unrecognized fields during Serde deserialization. Resolving a root preset overlays authored child-scope tweaks onto their selected baselines, preserving partial objects and retaining unknown entries for validation. Live documents inherit those completed root values unless they explicitly select a child preset, which replaces that scope.

Every built-in preset is complete over its scope, directly or through the presets it selects: the default root preset defines every root-scope setting and selects a preset for each child scope. Applying a complete scope preset replaces every value in that scope. The derived skill preset for a difficulty enables every technique and notable classified at or below it and sets each proficiency to its highest level classified at or below it; unclassified members, such as those marked "Ignored", are disabled.

The frontend bundles current built-ins for editing and preview. Seed inspection uses the seed's captured definitions rather than substituting these current built-ins.

## Setting definitions and scopes

Settings are organized hierarchically by semantic scope. Root scope is the settings document itself; `skill` contains skill assumptions, `item_pool` contains ordinary-item counts and pickup behavior, and `starting_state` contains initial equipment, resources, and flags. Each has its own built-in presets. Scope membership determines the persisted path independently of frontend layout. Moving a control between UI sections does not change that path; moving a setting across scopes requires explicit migration, whose details are deferred until needed.

Shared semantics include defaults, option tokens, upstream proficiency tiers, and preset baselines. Values are booleans, integers, or option tokens. A setting needing fractional precision uses an integer with a fixed scale, so equality and tweak comparisons behave consistently in Rust and JavaScript.

The backend validates generation inputs authoritatively in strict mode. The shared processing report rejects remaining item-pool totals after equipment deduction exceeding the prepared ordinary-location count, including when recovery is enabled. Starting equipment exceeding its pool count and starting ammunition exceeding capacity also remain blocking in recovery mode. The generator independently checks the same upper bound against its logic catalog and pads any unused ordinary locations with internal `Nothing` placements. The frontend will use the same processor through WASM for loading and feedback. Displaying a historical seed does not adjust its values to satisfy current generation constraints.

## Persisted settings

The backend first migrates a settings document's meaning, then applies current presets and defaults for live settings. An older frontend submits through this same generation path and receives the authoritative generated settings; see [generation](backend-api.md#generation).

Historical seeds are migrated to the current settings representation for display with current Svelte components. Their values and baselines come from their captured bundle. Historical UI descriptions or full shared-data exports are not required.

## Open questions

- The source representation of ordinary settings and whether to generate Rust or TypeScript types.
- The preparation and export tooling used by the frontend build.
- Browser integration of the shared Rust processor through WebAssembly.
- The source syntax for derived skill presets and cross-setting constraints.
