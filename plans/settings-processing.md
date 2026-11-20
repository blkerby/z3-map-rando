# Settings processing and browser loading

Status: Milestones 1 through 3 are complete. The native consumers share schema and semantic validation, explicit strict/recovery modes, batch repair, and blocking cross-setting findings. Milestones 4 onward bring browser loading onto the same processing through a backend endpoint. Use one Rust implementation of settings migration, preset resolution, validation, and recovery, called directly by the backend and CLI. The browser sends stored and imported documents to the backend for processing; its WebAssembly package is used only for ROM patching.

The settings semantics and persisted formats are described in [settings](../docs/data-formats/settings.md), [shared settings data](../docs/data-formats/settings-data.md), and [seed data](../docs/data-formats/seed-data.md). This plan tracks the implementation of the shared processing boundary and its use when loading settings in the browser.

## Scope and decisions

- Cover both generation and customization settings. Customization already has a Rust type (`patcher::CustomizationSettings`), a schema (`schema/settings/customization.json`), and built-in presets (`data/presets/customization/`), but no shared processing.
- Maintain migration and recovery code in Rust. Native consumers call it directly; the browser calls it through the backend. Processing in browser WebAssembly was measured and rejected: the `jsonschema` validator alone is about 4.2 MB (1.3 MB gzipped), roughly four times the current patching package, while the settings code itself is about 0.4 MB.
- Keep Svelte components responsible for presentation and ordinary editing. Replace the frontend's independent loading and type-recovery heuristics with the backend's processing results. Documents reaching the store are either backend results or stored by this browser with the current revision, so `readDocument` only converts them into store state, without type filtering.
- Keep Rust structs and JSON schemas separately maintained. JSON Schema defines object structure, scalar types, choices, and static numeric bounds; explicit Rust semantic verification uses existing prepared data for dynamic identities, allowed proficiency levels, and item-specific minimums. `PresetCatalog::definitions` remains removed.
- Use `schema/` as the project-wide home for JSON schemas, including a future `seed-data.json` for saved seed records. `schema/settings/generation.json` describes complete resolved generation values and references `schema/settings/skill.json` for skill assumptions; `schema/settings/customization.json` describes complete customization values. Small subordinate structures are inlined. Presets are validated after resolution; no separate preset-wrapper schema is needed for this work. Schemas for sparse document envelopes, seed records, and API wrappers are outside this implementation scope.
- Close fixed objects with `additionalProperties: false` and require fields guaranteed by preset resolution. Data-driven maps use generic value schemas through `additionalProperties`; specific technique, proficiency, room, notable, and item IDs never appear in the schema. Rust verifies their keys against the correct prepared sets.
- Apply structural and vocabulary migrations in every processing mode. Explicit migration rules rename, translate, or drop known retired identifiers before validation; recovery handles remaining unexpected or invalid input. Historical rules are added for concrete changes.
- Make strict processing the default. Recovery is used for browser loading and import, and through an opt-in CLI option. The generation endpoint and pinned seed reading are strict; pinned records continue using their captured baselines. Recovery is especially useful for development documents with incomplete migrations and documents from newer versions.
- Recovery always produces a loadable document: one whose remaining errors, if any, are cross-setting findings the player can fix in the editor. A document is unrecoverable only if even its root preset with no tweaks cannot be processed, which indicates a bug.
- Accept whole-valued numeric representations such as `10`, `10.0`, and `1e1` equivalently. Normalize the working JSON to checked integer representations before typed deserialization, without rounding fractions, clamping values, or converting strings. This normalization and ordinary preset inheritance apply in both modes and are not recovery.
- A settings revision identifies the processing code and prepared data. The browser stores it alongside each document it saves; documents stored with the current revision are loaded without a backend request. The revision is not part of the settings document, so it never appears in exports, share links, generation requests, or seed records.
- Cleaned documents replace the stored originals as soon as processing finishes. Notices are therefore shown once, when the change happens.
- Imported files and codes are always processed by the backend, whatever their origin. Exports record the randomizer version for provenance only.
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

- [x] Establish a shared Rust processing API that accepts a settings document and an initialized context containing the current prepared preset data and necessary semantic facts.
- [x] Keep processing independent of filesystem access, HTTP, browser APIs, and catalog compression. Separate those concerns only where needed for native and WASM consumers to share the implementation.
- [x] Move backend and CLI loading onto this API.
- [x] Remove `SettingDefinition`, `SettingValues`, `PresetCatalog::definitions`, and their builder and export code after replacement validation covers their current responsibilities.
- [x] Retain authoritative Rust validation for capacities, dynamic member identities, and permitted proficiency levels using existing prepared data.
- [x] Update the settings revision computation so changes to accepted types, semantic rules, and prepared preset data are represented after removing `definitions`. Frontend and backend builds from the same sources must still use the same revision.
- [x] Return a processing report containing the cleaned sparse document, resolved values, structured notices, and remaining blocking errors. Keep the captured preset baselines and provenance needed to construct a seed.

`seed_data::SettingsContext::process_settings` accepts a JSON document and returns a `SettingsReport`. The report holds the untouched original, cleaned document and captured baselines under `source`, optional resolved `values`, and structured `notices` and `errors` with string-segment paths and replacement values where applicable. `finish_processing` admits the resolved values to generation only when no blocking error remains. `preset_catalog::PresetCatalog` names the same context type; its crate supplies only the compressed codec. The context holds proficiency levels and ordinary-item minimums from preparation, and uses derived skill baselines and the ordinary-item vocabulary for known dynamic identities. The server retains its string notice response, and the CLI prints the same structured findings as text.

The shared revision hashes the dependency lockfile, the preset/settings/common-type sources and manifests, prepared semantic facts, and ordered preset IDs, scope selections, and values. Preset display labels do not affect it; source-only edits can conservatively advance it. The frontend export and native catalog carry identical contexts and revisions.

Temporary verification covered 106 recovery and normalization/idempotency cases, every root/skill preset combination, path diagnostics including enum-keyed item counts, blocking broken baselines, unchanged generator item-count rejection, native/frontend export parity, and revision behavior for label, preset-value, and proficiency-level changes. A further 117 cases checked recovery, idempotency, and canonical normalization of Serde's accepted enum representations. An Actix handler check verified HTTP 422 responses for blocking processing and item-count errors. A CLI seed generation verified recovered values, sparse normalization, source revision, captured baselines, and placements. Workspace checks, WASM compilation, and frontend type checks pass. Verification sources were removed.

The processor uses JSON Schema and semantic verification before one typed deserialization. Strict/recovery modes and batch repair are described in Milestone 3; the earlier Milestone 2 checks covered the original shared boundary.

## Milestone 3 Implement validation and recovery

Process live settings in this order, preserving the original input separately:

```text
structural and vocabulary migration
  -> preset selection and recursive overlay onto the selected baseline
  -> numeric representation normalization
  -> JSON Schema validation and semantic verification of map keys and per-member constraints
  -> strict failure or optional batch recovery and validation of the repaired result
  -> one typed Serde deserialization
  -> blocking cross-setting findings
  -> sparse normalization against the selected baseline and captured provenance
```

- [x] Author `schema/settings/generation.json` and `schema/settings/skill.json` independently of the Rust structs, using JSON Schema Draft 2020-12 with small subordinate structures inlined. Describe complete candidate settings rather than the sparse `SettingsDocument` envelope.
- [x] Evaluate the Rust `jsonschema` validator on the actual settings cases natively and in browser WASM, including URN references, multiple error paths, structured unknown-property details, malformed containers, and numeric representations.
- [x] Adopt the tested `jsonschema` version and initialize a reusable validator once alongside the prepared settings data. Resolve the schemas' URN identifiers and references through packaged resources in memory; browser processing must need no schema-fetch requests.
- [x] Package the same schema resources with native and browser builds and include every referenced schema's contents in catalog invalidation and the shared settings revision, alongside prepared semantic facts, presets, and relevant processing code. Missing references must fail preparation rather than trigger network retrieval.
- [x] Use `additionalProperties: false` on fixed schema objects and require their fields when resolution guarantees them.
- [x] Treat a missing required field after resolution as a blocking baseline/schema/resolver inconsistency, not an opportunity to invent a default.
- [x] Describe dynamic maps through generic value schemas: booleans for techniques and notable leaves, proficiency levels from 0 to the largest level defined in the current data (currently 5), and item counts from 0 to the ordinary item-location count (currently 151). Update these schema bounds when their corresponding data limits change; individual proficiencies still have their allowed levels checked semantically. Keep each starting-item constraint on its own property so their limits can evolve independently. Do not enumerate IDs or per-member allowed levels in the JSON schema.
- [x] Verify keys against prepared sets separately: techniques versus proficiencies, room IDs within the appropriate world, notable IDs within their room, and ordinary item-pool identities. Check proficiency levels and item-specific minimums using those same prepared facts.
- [x] Add an explicit processing mode with strict as the default. Keep migration, preset inheritance, numeric representation normalization, and removal of redundant tweaks available in both modes; recovery alone is opt-in.
- [x] Keep structural and vocabulary migration before validation or removal of unknown input. Explicitly drop known obsolete keys through migration rules and distinguish those removals from unexpected entries handled by recovery. There are no historical transformations to implement yet.
- [x] Preserve recursive overlay semantics so missing submitted fields inherit from the selected presets and partial objects retain inherited siblings.
- [x] Normalize whole-valued JSON numbers with checked conversions before typed deserialization. Accept `10.0` and `1e1` as `10` without a recovery notice; fractions, strings, and out-of-range values remain validation findings.
- [x] Collect schema and semantic findings with structured string-segment paths. Use keyword details to identify unexpected property names when a validator reports their containing object; do not parse display messages or broaden a repair to unrelated siblings.
- [x] In strict mode, reject unknown properties, unknown dynamic IDs, unknown presets, and invalid known values. Keep the original input and findings in the processing report.
- [x] In recovery mode, remove unknown entries and restore invalid known entries from the selected preset baseline. Recover malformed containers at the affected container while retaining valid entries elsewhere. An unknown entry with an invalid value produces one removal notice rather than a second value-recovery notice.
- [x] Handle unknown preset recovery before baseline resolution when recovery is enabled; return a resolution error in strict mode. Recovery is an explicit policy at the necessary processing stages, rather than exclusively a final pass.
- [x] Repair submitted entries in a batch, validate the repaired result, and deserialize into the existing Rust structs once. Replace the Serde error-by-error retry loop. Unattributable findings, missing required resolved fields, and failures remaining after repair stay blocking.
- [x] Remove redundant tweaks after recovery without materializing inherited values as explicit tweaks. Preserve the source revision, captured baselines, and untouched original input.
- [x] Make the generation endpoint strict and the CLI strict by default, with an explicit CLI option enabling recovery. Render migration/recovery notices appropriately without treating a successful migration as a strict-mode failure.
- [x] Surface cross-setting findings in the shared report and keep them blocking, without silently changing values to satisfy them. Preserve the generator's independent checks, including its exact ordinary-item count requirement.
- [x] Include the path, reason, and replacement value where applicable in recovery notices so the frontend can explain each adjustment.
- [x] Add the requested permanent built-in-preset consistency test: validate authored preset JSON during catalog preparation before typed deserialization, then process minimal current-version documents selecting each built-in root preset and supported root/skill combination, validate the complete values against the schema, deserialize into Rust, and require no notices or errors. This is a bounded compatibility check; obsolete optional schema properties can remain undetected, which is an accepted limitation.

The schema and semantic verification together define valid settings. For example, a technique map uses `{"type":"object","additionalProperties":{"type":"boolean"}}`; Rust determines whether each key names a known technique. All validation that can reject a map key runs before typed deserialization.

The authored schemas pass Draft 2020-12 schema checks with Python `jsonschema` and an in-memory resource registry. Verification covered 2 prepared root presets, 5 skill presets, all 10 root/skill combinations, 42 numeric-boundary and object-shape cases, and exact instance paths for five simultaneous invalid values. The permanent preset-processing consistency test also validates the shared processing path against both roots and all ten root/skill combinations.

The Rust compatibility probe uses `jsonschema` 0.61.2 with default features disabled, Draft 2020-12, an in-memory resource registry, and offline construction. Native execution and headless Chrome WASM produced identical reports for 2 root presets, 5 skill presets, 10 root/skill combinations, 36 scalar cases, 5 whole-valued decimal/exponent forms, 11 simultaneous invalid leaves, 5 unexpected properties across 4 containers, 6 malformed containers, 2 missing required fields, escaped JSON Pointer keys, and 100 repeated validator uses. Unknown-property names and missing required names are available as structured keyword operands. Integration checks found that the validator's segment iterator canonicalizes numeric-looking property keys such as `"05"`; the processor instead decodes the lossless JSON Pointer, preserving those keys and escaped characters exactly. A temporary batch-repair demonstration restored invalid values or removed unexpected keys, preserved a valid sibling tweak, revalidated, and deserialized the result into the actual Rust settings types; missing required fields remained blocking. Dynamic map keys remain unconstrained by the schema as intended. Missing URN resources and remote references failed construction offline, and the browser fetched only the WASM module, with no schema requests. Temporary probe sources were removed. The shared processor now uses this pinned dependency.

The optimized standalone probe module is 4,338,826 bytes, or 1,346,151 bytes with gzip compression at level 9. This includes probe code, prepared data, and schema fixtures; it is not a measurement of the production module's incremental size. Download size remains a consideration when integrating the validator into the shared browser package.

| Submitted entry or finding | Strict mode | Recovery mode |
| --- | --- | --- |
| Missing `map_layout` tweak | Inherit the selected baseline | Same |
| Known retired key | Apply its explicit migration, with a removal notice where appropriate | Same |
| `map_layout: 42` or `"garbage"` | Blocking validation error | Restore the selected baseline and report a notice |
| `starting_state.resources.bomb_capacity: 999` | Blocking range error | Restore the selected baseline and report a notice |
| Valid capacity written as `10.0` or `1e1` | Accept and normalize to integer `10` | Same |
| A valid explicit tweak | Retain it unless it equals the selected baseline | Same |
| Unknown property or dynamic map ID | Blocking validation error | Remove the submitted entry and report one notice |
| Unknown preset | Blocking resolution error | Select the scope's fallback preset and report a notice |
| Malformed container | Blocking type error | Restore that container's baseline and report a notice |
| Missing required field in resolved values | Blocking preparation inconsistency | Same |
| Invalid item-pool total | Blocking error without adjusting counts | Same |

Live settings adopt current built-in baselines. Pinned seed settings use their captured baselines and require strict reading; do not apply the live-settings update or lenient recovery to historical seed records. Seed inspection and its browser entry point remain separate work.

### Integrated processing results

`SettingsMode::Strict` is the default mode; callers select it explicitly. The generation endpoint is strict, and the CLI enables recovery only with `--recover-settings` alongside `--settings`. `SettingsErrors` retains all blocking findings for native callers and the HTTP 422 `errors` array. Revision updates remain document-level notices in both modes, with no field-path prefix. Format version 1 is the only authored format; unsupported formats are blocking in strict mode and interpreted through recognized fields in recovery mode.

The shared crate embeds the schemas and compiles a reusable offline validator once per process or WASM module. Schema contents are inputs to preset-catalog invalidation and the shared revision. Preset preparation first builds or reuses the logic catalog, from which it exports `ordinary_item_count` (currently 151). The shared report compares the item-pool total against that count; the generator independently checks against its own catalog through the same counting method. No resolved defaults are inferred from a pool total. Authored root presets are validated as resolved JSON before typed deserialization; fixed Rust settings structs and preset-file wrappers also reject unknown fields. Dynamic identities are checked against derived skill presets and ordinary-item records rather than trusting authored root baselines. Root skill tweaks overlay the selected child baseline; explicit child-preset selection in a live document replaces that scope.

Temporary integrated checks covered 145 documents in both modes (290 reports), including wrong types and choices, scalar and container recovery, generic and per-member limits, unknown fixed properties and dynamic IDs, unknown presets, newer formats, whole-valued numbers, sparse normalization, provenance, captured baselines, and idempotency. The notable-key cases included synthetic prepared room/notable entries for both worlds. Separate checks confirmed blocking invalid baselines and missing required fields. Headless Chrome WASM produced identical reports to native execution, including a repeated run; it made no schema-fetch requests. The temporary bindings are not production exports; those remain Milestone 4.

An Actix handler check verified aggregate strict-validation findings and item-total findings in HTTP 422 responses. The CLI rejected simultaneous invalid entries by default and generated a seed with explicit recovery, preserving a valid sibling capacity, the selected Testing baseline, and sparse source settings. Workspace checks, browser-target compilation, and frontend type checks pass. Only the requested built-in-preset consistency test remains; other verification sources were removed.

## Milestone 4 Settings revision and project version

- [x] Compute a settings revision during preset preparation, covering everything that can change processing results. The preset catalog's cache fingerprint serves as the revision: it covers the dependency lockfile, the catalog builder, settings processing, preset, and common-type sources, the settings schemas, the preset and classification data, and the logic catalog. It is conservative, since unrelated edits to its inputs also advance it. The revision is `SettingsContext::revision`, so it reaches the backend through the preset catalog and the frontend through `settings_data.json` as `generation.revision`.
- [x] Compute the project version as described in the [architecture overview](README.md#project-version). The catalog builder writes it to `build/version.txt` on every run, outside its catalog cache, so neither the revision nor any Rust compilation depends on it. The server reads the file at startup, the CLI reads or bundles it like the catalogs, and the generator receives the version from its caller. `common_types::VERSION` is removed.

## Milestone 5 Backend processing endpoint

- [ ] Move `CustomizationSettings` into `seed_data`, add the customization presets to the preset catalog, and process customization documents through the same `SettingsContext` path with their own schema. Replace the Rust `Default` for customization with the default customization preset, so the defaults have one source; the CLI uses that preset when no customization file is given.
- [ ] Guarantee a loadable result in recovery mode. If findings other than cross-setting ones remain after per-setting repair, fall back in steps, reporting each: drop the tweaks of each child scope that still has errors while keeping its preset selection, then drop all tweaks while keeping the root preset (or the default root preset if it is unknown). A document still failing after that is unrecoverable.
- [ ] Add `POST /api/settings/process`, taking batches of generation and customization documents and processing each in recovery mode. Return one result per document: the cleaned document, notices, and remaining cross-setting errors, or an unrecoverable marker. Include the backend's settings revision in the response. Processing uses no generation slot.
- [ ] Process each document under `catch_unwind`, so a panic or error caused by one document makes only that document unrecoverable rather than failing the batch.
- [ ] Size the request body limit for the largest valid batch: the maximum user presets of both kinds plus drafts, and the import size limit.
- [ ] Include the frontend's revision in the generation request wrapper, outside the settings document. A mismatch returns an error the frontend shows as a reload prompt.
- [ ] Verify recovery robustness with temporary fuzzing: mutated documents with swapped types, removed and added keys, and scalars in place of containers must each produce a loadable result, never a panic or `Err`. Remove the fuzzing code afterwards.

## Milestone 6 Frontend loading and storage

- [ ] Store the settings revision alongside each stored document: the generation draft, the customization draft, and each user preset list. A document stored with the current revision is loaded directly; one with another revision, or none, is sent to the endpoint and written back with the current revision as soon as its result arrives.
- [ ] Hold back everything below the navbar until the stored documents are ready, on both Generate and seed pages. Show a progress indicator only after a short delay, so fast responses do not flash. Move the seed page's data loading from its SvelteKit `load` into the page component, so the navbar stays visible there too.
- [ ] On a network failure, timeout, or server error for the whole request, show the error with a Retry button. Leave stored data untouched; autosave stays disabled until loading completes.
- [ ] Treat stored values that are not valid JSON as unrecoverable without sending them.
- [ ] For each unrecoverable document, show a blocking prompt naming it ("Your draft", "Preset *X*"), offering to reset a draft to Default or delete a preset. Offer "Copy settings" first, so the raw input can be kept for a bug report. Other documents load normally.
- [ ] Show recovery notices once, in a dismissible alert at the top of the main content, grouped by document, with setting paths. Show remaining cross-setting errors through the editor's existing problem display.
- [ ] If the endpoint's revision differs from the bundled revision, the page is stale: ask the player to reload instead of loading the results.
- [ ] Process imported files and preset codes through the endpoint before adding them; the import dialog waits for the result and shows its notices.
- [ ] Record the randomizer version, imported from `build/version.txt`, in exported preset files, for provenance only. Exports carry no settings revision.
- [ ] Read the settings format version from `settings_data.json` rather than the WebAssembly package, so generation does not wait for it.
- [ ] Remove the type filtering from `readDocument`, keeping its conversion of preset selections and known setting paths into store state.
- [ ] Keep ordinary editing local. Generation requests submit the cleaned scoped document; the backend independently runs strict processing and reports remaining invalid input.

## Milestone 7 Verify and document

- [ ] Cover multiple simultaneous findings, wrong scalar types, invalid choices, invalid integer ranges, invalid proficiency levels, unknown properties and dynamic IDs, malformed containers, missing entries, and documents from newer formats, for both generation and customization. Verify additional-property errors remove only the offending submitted keys in recovery mode.
- [ ] Confirm required fields are supplied by resolution, partial objects preserve inherited siblings, and a broken baseline or schema mismatch blocks strict processing.
- [ ] Confirm migration runs before recovery when the first actual migration is introduced, including explicit obsolete-key removal accepted by strict consumers.
- [ ] Check in the browser: current-revision storage loads without a request; stale storage is processed, written back, and its notices shown once; network failure leaves storage intact and retries; an unrecoverable document can be copied and discarded; a stale tab is asked to reload.
- [ ] Verify strict backend generation, strict CLI defaults, and explicit CLI recovery through the shared loader. Keep pinned seed reading strict against captured baselines.
- [ ] Verify release and dev version strings from tagged and untagged commits.
- [ ] Update the settings, shared-data, backend API, and backend documentation to describe the implemented endpoint, revision, and loading behavior.
- [ ] Keep the user-requested built-in-preset consistency test. Remove other temporary verification code before completing each implementation task, following the repository instructions.

## Deferred work

Full browser ROM patching, share links, retrieval endpoints, schemas for seed records or API wrappers, and background-worker execution are separate work. When browser patching is implemented, its WebAssembly package needs development rebuilds on Rust changes, content-hashed assets with long-lived immutable caching, and loading only on pages that patch. Historical transformations are added for concrete format or vocabulary changes rather than speculative old formats.
