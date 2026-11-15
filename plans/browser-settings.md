# Browser WebAssembly and settings recovery

Status: Milestone 1 is complete, including native/WASM catalog decoding checks. Milestones 2 onward are pending. Use one Rust implementation of settings migration, preset resolution, validation, and recovery from the backend, CLI, and browser. Introduce one browser WebAssembly package for settings processing, then extend that package with ROM patching when browser patching is implemented. Saved presets are processed locally without backend requests.

The settings semantics and persisted formats are described in [settings](../docs/data-formats/settings.md), [shared settings data](../docs/data-formats/settings-data.md), and [seed data](../docs/data-formats/seed-data.md). This plan tracks the implementation of the shared processing boundary and browser integration.

## Scope and decisions

- Start with generation settings. The customization controls currently contain sample options; move customization onto the shared processing model when its real Rust settings are defined.
- Maintain migration and recovery code in Rust. Native consumers call it directly; the browser calls it through WebAssembly.
- Keep Svelte components responsible for presentation and ordinary editing. Replace the frontend's independent loading and type-recovery heuristics with the shared loader.
- Remove `PresetCatalog::definitions`. Use Serde for representation checks and explicit Rust checks for semantic constraints. Reuse existing prepared data for dynamic identities and allowed proficiency levels.
- Defer JSON Schema. Sharing the Rust implementation supplies consistent validation without maintaining validators in two languages. Reconsider a schema if it later simplifies tooling or exposing constraints to controls.
- Begin with one browser package and one WASM module. Keep large patch catalogs and assets separate from the binary. Split the module only if measured download or initialization costs justify it.
- The backend independently processes generation requests even when the browser has already processed them. A browser may be stale, and callers may bypass it.

## Milestone 1 Establish the browser build

- [x] Checkpoint the existing backend implementation and confirm a clean worktree before this refactor.
- [x] Confirm `wasm32-unknown-unknown` and `wasm-pack` are available in the development environment.
- [x] Add a thin `crates/browser` binding crate using `wasm-bindgen`.
- [x] Build with `wasm-pack build --target web` and load a small settings entry point from the Generate page.
- [x] Check the settings and catalog dependencies for WASM compatibility, particularly compression dependencies. Native and browser WASM decode and re-encode all four catalogs identically.
- [x] Check the patcher's dependency graph and build portability before committing to the combined package. The full patcher links into a temporary WASM module; full ROM-patching integration belongs to a later milestone.
- [x] Record initial WASM size and initialization time as a baseline.

Keep this milestone small. If portability requires substantial changes to catalog formats or dependency boundaries, stop and review those changes before proceeding with the loader refactor.

### Browser build results

`crates/browser` exports `get_settings_format_version`, reading the shared `seed_data::SETTINGS_FORMAT_VERSION`. The Generate page preloads the package, and generation requests use that value. `loadBrowser` reuses the initialization promise and allows retry after a failed download. Existing document loading and recovery remain for Milestones 2 through 5.

Vite builds the package into the ignored `build/browser/` directory when its configuration loads. Production emits `_app/immutable/assets/browser_bg.B0BRQumY.wasm`. Chrome checks verified that Home does not fetch WASM, entering Generate fetches and initializes it once, and leaving and returning through client navigation reuses it. An intentionally failed first download was reported through the existing error UI, and a subsequent generation request retried initialization successfully.

Baseline measured on 2026-11-14 using headless Google Chrome 156.0.8078.47 on Linux x86_64, Intel Core i7-1360P. Builds used the default wasm-pack release profile with wasm-opt. Each mode used an isolated browser profile and five loads with the HTTP cache disabled, against a local server. Timings exclude page startup and JavaScript-module download; fetch-to-ready runs from the WASM request start to completion of instantiation. Subsequent loads may benefit from browser compilation caches.

| Measurement | Vite development | Rust production server |
| --- | --- | --- |
| WASM bytes | 13,336 | 13,336 |
| First fetch-to-ready time | 3.5 ms | 2.7 ms |
| Median fetch-to-ready time over five loads | 3.5 ms | 5.1 ms |
| Median streaming compilation and instantiation time | 0.7 ms | 1.1 ms |

Generated JavaScript glue is 4,284 bytes before Vite bundling and minification. These are the small format-version entry point's baseline costs, not a prediction for the complete settings engine or patcher. Automatic Rust-source watching and explicit immutable cache headers remain Milestone 6 work.

### Catalog portability

Catalog files contain eight magic bytes followed by a Zstd-compressed bincode payload. Stale-catalog invalidation uses the builder's existing SHA-256 fingerprints of code, configuration, and data inputs. Runtime codecs do not compute or store schema/type fingerprints, avoiding architecture-dependent hashing and schema-reflection dependencies. Browser catalog assets must use content-hashed URLs and ship with their matching WASM build; manually supplied incompatible catalogs have no dedicated schema-mismatch check.

Temporary native and headless Chrome WASM checks decoded the rebuilt logic catalog (145 rooms), retiling catalog (89 areas), patch catalog (257 item locations), and preset catalog (2 root presets). Each re-encoded file matched its native-produced input byte-for-byte, including compression. Temporary verification sources were removed.

The preset catalog and patcher compile for `wasm32-unknown-unknown`; the full patcher and catalog codecs also linked into a temporary WASM module during the initial build checks. Actual ROM patching has not been exercised in WASM.

## Milestone 2 Consolidate settings processing

- [ ] Establish a shared Rust processing API that accepts a settings document and an initialized context containing the current prepared preset data and necessary semantic facts.
- [ ] Keep processing independent of filesystem access, HTTP, browser APIs, and catalog compression. Separate those concerns only where needed for native and WASM consumers to share the implementation.
- [ ] Move backend and CLI loading onto this API.
- [ ] Remove `SettingDefinition`, `SettingValues`, `PresetCatalog::definitions`, and their builder and export code after replacement validation covers their current responsibilities.
- [ ] Retain authoritative Rust validation for capacities, dynamic member identities, and permitted proficiency levels using existing prepared data.
- [ ] Update the settings revision computation so changes to accepted types, semantic rules, and prepared preset data are represented after removing `definitions`. Frontend and backend builds from the same sources must still use the same revision.
- [ ] Return a processing report containing the cleaned sparse document, resolved values, structured notices, and remaining blocking errors. Keep the captured preset baselines and provenance needed to construct a seed.

Initialize the browser's settings context once using the prepared data already bundled with the frontend. Processing each additional saved preset must not reload the context or make a backend request.

## Milestone 3 Implement validation and recovery

Process live settings in this order:

```text
structural and vocabulary migration
  -> selected preset baseline
  -> supplied tweaks and validation
  -> recovery of invalid entries
  -> normalization against the selected baseline
  -> remaining cross-setting findings
```

- [ ] Keep structural and vocabulary migration before validation or removal of entries. There are no historical transformations to implement yet; add actual migration rules when a setting or format changes.
- [ ] Supply missing entries from the selected presets and defaults.
- [ ] Accept unknown keys without rejecting the document. Ignore them for generation, optionally report a notice, and retain them in the preserved original input.
- [ ] Recover invalid known entries using their selected preset baseline, rather than a raw setting default or an arbitrary valid choice.
- [ ] Recover malformed containers at the affected container while retaining valid entries elsewhere in the document.
- [ ] Use `serde_path_to_error` for representation failures. Have explicit semantic checks produce the same structured path information.
- [ ] Ensure every recovery attempt removes an offending submitted entry. Return an error if an error cannot be attributed to a submitted entry; do not retry indefinitely or hide a broken baseline.
- [ ] Remove redundant tweaks after recovery without materializing inherited values as explicit tweaks.
- [ ] Keep cross-setting failures blocking and avoid silently changing values to satisfy them. Preserve the generator's current constraints, including its exact ordinary-item count requirement.
- [ ] Include the path, reason, and replacement value in recovery notices so the frontend can explain each adjustment.

Expected behavior includes:

| Submitted entry | Result |
| --- | --- |
| Missing `map_layout` | Inherit the selected baseline |
| `map_layout: 42` | Recover to the baseline and report a type notice |
| `map_layout: "garbage"` | Recover to the baseline and report an invalid-choice notice |
| `starting_items.bomb_capacity: 999` | Recover to the baseline and report a range notice |
| A valid explicit tweak | Retain it unless it equals the selected baseline |
| An unknown key | Accept the document and ignore that entry for generation |
| An invalid item-pool total | Report a blocking error without adjusting counts |

Live settings adopt current built-in baselines. Pinned seed settings use their captured baselines and require strict reading; do not apply the live-settings update or lenient recovery to historical seed records. Seed inspection and its browser entry point remain separate work.

## Milestone 4 Expose the shared loader through WASM

- [ ] Expose initialization and generation-settings processing through the browser binding crate.
- [ ] Choose a straightforward Serde interchange format for documents and reports. Use typed arrays for future ROM byte buffers.
- [ ] Keep settings data in the initialized Rust context so processing a list of presets does not repeatedly transfer the same catalog across the JavaScript boundary.
- [ ] Reuse one initialized module instance within the open page.
- [ ] Keep server, object-store, and generation dependencies out of the browser binding crate.

Only export implemented functionality. Add ROM-patching exports to this package when the browser patching feature is implemented; do not add placeholder exports now.

## Milestone 5 Integrate frontend loading and storage

- [ ] Process generation drafts, saved user presets, imported files and preset codes, and share-link settings before placing them into editor state.
- [ ] Replace generation settings calls to `readDocument` and its basic `typeof` filtering with the shared processing result. Keep customization on its existing loader until its Rust model is defined.
- [ ] Display recovery notices and blocking errors, including their setting paths.
- [ ] Preserve the original imported or stored document until an explicit save accepts the cleaned version. Decide its browser-storage representation before changing the autosave behavior.
- [ ] Prevent loading and recovery alone from triggering autosave effects that overwrite the preserved original. Handle drafts and saved user presets consistently.
- [ ] Keep ordinary editing local. Generation requests continue to submit the scoped document, and the backend continues to process it independently.
- [ ] Handle module-loading failure without discarding stored presets or drafts.

## Milestone 6 Integrate development builds and caching

- [ ] Build the browser package before frontend development, checking, or production builds need its generated bindings and type declarations.
- [ ] Rebuild WASM when relevant Rust sources change and refresh the browser to load the new module.
- [ ] Preserve normal Vite hot reload for Svelte, TypeScript, and CSS edits without restarting Rust.
- [ ] Load the browser module when entering Generate rather than on unrelated pages.
- [ ] Emit WASM and its JavaScript glue as content-hashed build assets.
- [ ] Serve hashed immutable assets with a long-lived immutable cache policy. Revalidate HTML and update metadata so browsers discover changed asset URLs.
- [ ] Keep API responses outside the static-asset cache policy.
- [ ] Confirm unchanged WASM keeps a reusable asset URL and changed WASM receives a new URL.

Use normal browser caching initially. A release that does not change the WASM bytes need not force another WASM download; browser cache eviction can still cause a download.

## Milestone 7 Verify and document

- [ ] Compare native and WASM processing results for complete values, sparse documents, preset scopes, normalization, and captured provenance.
- [ ] Cover wrong scalar types, invalid choices, invalid integer ranges, invalid proficiency levels, unknown keys and presets, malformed containers, missing entries, and documents from newer formats.
- [ ] Confirm migration runs before recovery when the first actual migration is introduced.
- [ ] Verify original drafts and presets survive loading and recovery until explicit save.
- [ ] Check bulk preset loading for repeated catalog transfers, backend requests, and browser responsiveness.
- [ ] Verify backend generation and CLI behavior through the shared loader.
- [ ] Verify fresh-checkout frontend builds, development rebuilds, asset hashes, and production cache headers.
- [ ] Record WASM download size and initialization time. Use those measurements to decide whether the combined module needs changing.
- [ ] Update the settings, shared-data, and backend documentation to describe the implemented processing boundary and recovery behavior.
- [ ] Remove temporary verification code before completing each implementation task, following the repository instructions.

## Deferred work

Full browser ROM patching, retrieval endpoints, real customization settings, additional WASM modules, background-worker execution, and JSON Schema adoption are separate work. Historical transformations are added for concrete format or vocabulary changes rather than speculative old formats.
