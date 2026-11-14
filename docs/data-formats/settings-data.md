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
| `data/presets/` | Built-in preset files (see [Preset files](#preset-files)) |
| `data/tech_data.json`, `data/notable_data.json` | Difficulty classifications of techniques, proficiency levels, and notables; skill presets are derived from them |
| `data/items.json` | The item token vocabulary, with upstream item names, count limits, display category, and placement tier |

The preset directory is:

```text
data/
  presets/
    generation/
      root/
        default.json
    customization/
      root/
        default.json
```

Skill presets have no files: they are derived from the difficulty classifications. Skill settings not covered by the classifications would get sparse files under `generation/skill/`, holding only those settings, with the derived members filled in by preparation.

The exact representation of ordinary setting definitions, including whether Rust and TypeScript types are handwritten or generated, remains open. The goal is to define shared semantics once and avoid per-setting translation code.

```text
shared setting definitions + preset files + local classifications + upstream data
  -> common semantic preparation
  -> backend data and frontend build inputs

frontend build inputs + Svelte components
  -> frontend application with bundled data
```

The common preparation step is `catalog_builder presets` (`crates/catalog_builder/src/presets.rs`). It derives the skill presets, resolves each generation root preset into complete settings, and writes two outputs from the same prepared data: the preset catalog (`preset_catalog.bin`) for the backend, and `settings_data.json` for the frontend. The latter also carries what the frontend shows about each setting: the difficulty ladder, the techniques, proficiencies, and notables with their upstream names and difficulties, the item records, and the customization presets. The frontend's Vite config runs the step whenever it loads and reruns it in dev when `data/` changes, so building or developing the frontend requires the Rust toolchain but no running backend.

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
    "item_pool": { "fire_rod": 1, ... }
  }
}
```

A root preset can select a skill preset through its nested `skill` object; a skill preset's own values are directly inside its `settings` object. User-defined presets remain root-only and based directly on a built-in root preset.

Every built-in preset is complete over its scope, directly or through the presets it selects: the default root preset defines every root-scope setting and selects a preset for each child scope. Applying a complete scope preset replaces every value in that scope. The derived skill preset for a difficulty enables every technique and notable classified at or below it and sets each proficiency to its highest level classified at or below it; unclassified members, such as those marked "Ignored", are disabled.

The frontend bundles current built-ins for editing and preview. Seed inspection uses the seed's captured definitions rather than substituting these current built-ins.

## Setting definitions and scopes

Settings are organized hierarchically by semantic scope. Root scope is the settings document itself; `skill` is a child scope containing skill assumptions. Scope membership determines the persisted path independently of frontend layout. Moving a control between UI sections does not change that path; moving a setting across scopes requires explicit migration, whose details are deferred until needed.

Shared semantics include defaults, option tokens, upstream proficiency tiers, and preset baselines. Values are booleans, integers, or option tokens. A setting needing fractional precision uses an integer with a fixed scale, so equality and tweak comparisons behave consistently in Rust and JavaScript.

The backend validates generation inputs authoritatively. The frontend can give immediate feedback using the same definitions. Cross-setting constraints, such as an item pool fitting the available locations, are independent of presentation; their declaration format remains open. Displaying a historical seed does not adjust its values to satisfy current generation constraints.

## Revisions and persisted settings

A settings data revision identifies the shared semantic definitions and preset data used by a build. Frontend and backend builds from the same sources use the same revision even if they package the data differently. The exact computation remains open; it does not depend on an HTTP response or frontend layout.

Settings documents record their source revision. The backend first migrates their meaning, then applies current presets and defaults for live settings. An older frontend submits through this same generation path and receives the authoritative generated settings and any upgrade notices; see [generation](backend-api.md#generation).

Historical seeds are migrated to the current settings representation for display with current Svelte components. Their values and baselines come from their captured bundle. Historical UI descriptions or full shared-data exports are not required.

## Open questions

- The source representation of ordinary settings and whether to generate Rust or TypeScript types.
- The preparation and export tooling used by the frontend build.
- Whether frontend resolution and baseline comparison use TypeScript or shared Rust code through WebAssembly.
- The source syntax for derived skill presets and cross-setting constraints.
