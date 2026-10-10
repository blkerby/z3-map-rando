# Prototype implementation plan

Implement these steps separately. The first usable prototype generates saved seeds with uniform item placement; ROM patching follows later. Logical placement, overworld rearrangement, and the web service remain outside this prototype.

See [the architecture plan](plans/README.md) and [CLI design](plans/cli.md) for the data boundaries and user-facing behavior. Commands below describe the planned crate and binary names.

## 1. Consolidate the existing catalog builders

Create `crates/catalog_builder` with a library containing separate logic and retiling modules. Move the existing builders into this crate, preserving their behavior and individual binaries, `build_logic_catalog` and `build_retiling_catalog`. Keep the runtime catalog crates separate.

The binaries handle arguments and files; library operations can also be called directly by a build script. Do not invoke nested `cargo run` from `build.rs`.

Result: both existing catalogs can be built through the shared builder crate.

## 2. Add cached ASM assembly

Add a patch-building module that automatically builds the repository's patched Asar submodule with CMake under `build/asar/`, using Release mode and only the standalone assembler target. Let CMake manage incremental C++ compilation. An explicit Asar executable override skips this build and must also support the patched IPS output mode; an upstream Asar executable will not work. Discover patch roots under `patches/src/` and recursively resolve literal `incsrc` paths relative to the including file. Support only this dependency syntax for now.

Cache each IPS independently. Its fingerprint covers source and included file paths and contents, the assembler contents, and assembly arguments and defines. Missing outputs or changed fingerprints trigger assembly; record success only after completed output is available. Use invocation-local temporary ROM paths instead of the shared `/tmp/dummy.smc`.

Result: editing one patch rebuilds only that patch; editing `symbols.inc` rebuilds its consumers. Unchanged patches, including `fastrom_base`, are reused.

## 3. Build the patch catalog

Define a runtime patch catalog and its encoder/decoder using the existing bincode envelope convention. Add `build_patch_catalog` to the builder crate. Bundle separate named IPS patches and their matching symbol manifest. Application order, phases, and optional patch selection stay in patcher code, including the initial `fastrom_base` transformation.

Symbol manifest generation is not implemented yet: `symbols.inc` provides the ASM interface and Rust currently duplicates patching address constants. Add an export step for the symbols the patcher needs and include the resulting manifest with the matching assembled patches. Select the export mechanism in this step.

Read `z3-json-data` directly to extract stable location mappings, ROM addresses, item receipt IDs, and prize encodings for the patch catalog. Share source-reading types with the logic builder, without depending on the generated logic catalog.

Result: one artifact supplies the fixed patches and symbols to native and browser patchers, together with the source-derived item/location patching data.

## 4. Add shared caching and the builder orchestrator

Add `build_catalogs` to update all three catalogs in one command:

```sh
cargo run -p catalog_builder --bin build_catalogs
```

Default source paths to sibling `../z3-json-data` and `../ALTTPRetiling` directories, resolved from the repository root. Check in `catalog-build.default.toml` and copy it to the Git-ignored `catalog-build.toml` if the local file is missing when a builder or bundled build runs. Read that local configuration without overwriting or merging it. Standalone binaries also accept path arguments; no custom environment variables are needed.

Use this default template:

```toml
logic_source = "../z3-json-data"
retiling_source = "../ALTTPRetiling"

# Override the automatically built assembler with our patched Asar executable.
# IPS output support is required; an upstream Asar executable will not work.
# asar = "/path/to/patched/asar"
```

Asar requires an initialized submodule, CMake, and a C++ toolchain. The source data repositories can remain sibling checkouts. Do not download repositories automatically.

Use the ignored top-level `build/` directory for catalogs, IPS files, and cache metadata. Fingerprint all relevant source inputs, builder code, and configuration so changed data or implementations update the appropriate outputs. Fingerprints do not depend on debug versus release profiles.

Use one shared cache lock during updates and snapshot copying. Write completed artifacts through temporary files so interrupted builds do not publish partial outputs. Standalone builders and the orchestrator share this implementation.

Result: manual and automatic builds reuse the same cache, which survives `cargo clean`.

## 5. Make seed identities independent of catalogs

Retain authored `(room_id, item_id)` pairs in logic-catalog item locations and replace saved `item_location_idx` values with these pairs. Establish that existing IDs are permanent and removed IDs are not reused. Item names remain the item identity; receipt IDs and ROM addresses are patching details. Remove location ROM addresses, item receipt IDs, and prize patch bytes from the logic catalog; the patch builder reads these directly from `z3-json-data`.

Add an integer seed `format_version` starting at `1`, following the [versioning rules](plans/README.md#saved-seed-format): bump for meaningful interpretation changes, not compatible extensions. Keep generator metadata and catalog hashes as provenance, without requiring an old catalog or patcher version. Make the saved-seed types available to generation and patching without making the patcher depend on the generator's implementation.

Result: reordering a logic catalog does not change saved location identities.

## 6. Define and prepare seed-specific retiling content

Define serializable `SeedRetilingData` containing the selected layouts, palettes, tiles, and other content needed for one seed. For the prototype, select one whole-game theme and retain vanilla area placement and connections.

Extract this content during generation without a ROM. Preserve sanitized vanilla graphics references with stable meanings, rather than copying vanilla pixels. Do not save runtime allocation choices, patch symbols, or patch-specific ROM addresses. The selected content must remain usable without its original catalog.

Result: saved seeds contain everything needed from the retiling catalog.

## 7. Implement the uniform generator library

Expand the settings item pool and uniformly shuffle it across eligible ordinary locations. Shuffle maps and compasses within their own dungeon. Keep keys and prizes vanilla, but record all placements. Preserve tech and proficiency settings without using them for placement yet.

Accept settings, generation catalogs, the selected theme, and deterministic randomness from the caller. Prepare the selected retiling content. Keep clocks, filesystem access, and seed-name generation outside the generator library.

Result: generation produces structured seed data without logical placement or ROM patching.

## 8. Add seed serialization and the generation CLI

Create `crates/cli` with binary `z3-map-rando`, initially supporting generation through external catalogs. Implement the arguments and defaults in [plans/cli.md](plans/cli.md), including bundled checked-in settings.

Choose or accept a `u64` RNG seed. Generate and persist the fresh nine-character seed name using the agreed separate naming RNG. Record the UTC timestamp, package version, clean Git commit when available, and catalog provenance.

Stream JSON through Zstd when saving, and deserialize through a buffered Zstd decoder when loading. Default seed output to `z3-map-rando-<SEED-NAME>.json.zst`. Naming remains fresh when `--rng` repeats, while placements remain reproducible with identical generation inputs.

Result: a usable generation-only CLI that writes a complete compressed seed.

## 9. Add automatic generation and embedding to bundled builds

Add the default-off `bundle-catalogs` feature and optional builder dependency. The CLI's `build.rs` calls the shared update operation only with this feature enabled. Track source directories/files, path overrides, and assembler changes using Cargo's rerun directives. Track the local configuration file, creating it from the template only for bundled builds or standalone builder invocations.

While holding the shared cache lock, update `build/` and copy the completed catalogs into this build's `OUT_DIR`. Embed those snapshots, not mutable cache paths. Debug and release share expensive generation and assembly work while each compilation has consistent inputs.

```sh
cargo build -p cli --release --features bundle-catalogs
```

Result: bundled builds automatically create or update their assets. Ordinary `cargo check` without the feature requires neither catalogs, source repos, nor Asar. External catalog arguments can override embedded catalogs.

## 10. Extract catalog-independent retiling patching

Refactor the ROM-building flow in `theme_check` into a reusable patcher operation consuming a ROM, decoded seed, current patch catalog, and customization settings. Consume IPS bytes from the catalog instead of filesystem paths. Resolve vanilla graphics references from the ROM and compile saved retiling content into the current runtime layout using current symbols.

Result: retiling patching needs neither the logic nor retiling catalog, and its core has no filesystem, clock, network, or RNG dependency.

## 11. Add item-placement patching

Use the patch catalog's source-derived location mappings and current item and prize encodings to translate saved identities. Extend this data where necessary for the patched pickup carriers. Implement the location-specific writes and hooks required by the prototype's randomized pickups.

Use the latest patcher and matching patch catalog for old seeds. Support older seed formats through defaults or migrations that preserve their meaning; new customization options remain patch-time inputs.

Result: the patcher applies saved placements without the original logic catalog or saved ROM-write addresses.

## 12. Complete CLI patching and remove tracked IPS artifacts

Wire `--seed` plus `--rom` to patching, and generation plus `--rom` to generation followed by patching. Save the seed before patching in combined operation. Reuse the saved seed name for default ROM output paths.

Move remaining IPS consumers and development instructions to generated outputs or the patch catalog, then remove `patches/ips/` files from source control. Update or retire `scripts/build_patches.py` and `scripts/build_asar.py` so there is one assembly and caching implementation. Document the local configuration, source-directory overrides, and Asar prerequisites.

Result: the CLI generates, patches, or does both based on its arguments, with no user-specified catalogs needed for a bundled binary.

## Verification during implementation

Use the existing checks and temporary development checks appropriate to each step; do not add permanent tests or validation without an explicit request. Check unchanged-cache reuse, individual ASM edits, shared include edits, and reuse between debug and release builds. Exercise seed serialization and CLI generation before moving on to ROM patching. Check that patch-only operation works without generation catalogs and that saved seeds remain usable after catalog rebuilds.
