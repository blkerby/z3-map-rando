# Prototype implementation plan

Prototype formats may change freely: rebuild catalogs or regenerate seeds rather than adding migrations or legacy readers. Saved seeds with stable identities and high-level layout choices remain a design goal for eventual long-term compatibility; patching resolves them using current catalogs rather than archived generation catalogs.

Implement these steps separately. The first usable prototype generates saved seeds with uniform item placement; ROM patching follows later. Logical placement, overworld rearrangement, and the web service remain outside this prototype.

See [the architecture plan](plans/README.md) and [CLI design](plans/cli.md) for the data boundaries and user-facing behavior. Commands below describe the planned crate and binary names.

## 1. Consolidate the existing catalog builders (implemented)

Create `crates/catalog_builder` with a library containing separate logic and retiling modules and one CLI binary. Move the existing builders into this crate, preserving their behavior. Keep the runtime catalog crates separate.

The CLI handles arguments and dispatches through `logic`, `retiling`, and `all` subcommands. Initially, `all` builds both existing catalogs; extend it when the patch builder is added. Library operations can also be called directly by a build script. Do not invoke nested `cargo run` from `build.rs`.

```sh
mkdir -p build
cargo run -p catalog_builder -- logic
cargo run -p catalog_builder -- retiling
cargo run -p catalog_builder -- all
```

Implemented: one builder CLI can build either existing catalog or both, with shared library modules available to other callers. All subcommands read their options from the root configuration introduced in step 5.

## 2. Compress catalog payloads (implemented)

Keep the magic bytes and 64-bit schema identifier uncompressed, and encode the bincode payload through Zstd. Add streaming writer and reader APIs so serialization and deserialization do not require a complete uncompressed payload buffer. Update catalog consumers to read the compressed format; the future patch catalog uses the same convention.

Configure `compression_level` in `catalog-build.toml`, with the template setting level `3` independently of Cargo profile. Higher levels remain explicit choices; the future CI release process can select level `18`. Change the prototype format directly and rebuild catalogs, without legacy readers or backward compatibility work.

Shared configuration and caching in step 5 incorporate the compression option. Keep all generated artifacts in `build/`; changing the level regenerates compressed catalog outputs without rebuilding Asar or reassembling unchanged patches.

Implemented: logic and retiling catalogs have streaming writer/reader APIs and Zstd-compressed payloads. The builder reads `compression_level` from local configuration, with `3` in the default template. `theme_check` reads the compressed catalog directly from a file stream. No legacy format reader is retained.

## 3. Add cached ASM assembly (implemented)

Add a patch-building module that automatically builds the repository's patched Asar submodule with CMake under `build/asar/`, using Release mode and only the standalone assembler target. Let CMake manage incremental C++ compilation. An explicit Asar executable override skips this build and must also support the patched IPS output mode; an upstream Asar executable will not work. Discover patch roots under `patches/src/` and recursively resolve literal `incsrc` paths relative to the including file. Support only this dependency syntax for now.

Cache each IPS independently. Its fingerprint covers source and included file paths and contents, the assembler contents, and assembly arguments and defines. Missing outputs or changed fingerprints trigger assembly; record success only after completed output is available. Use invocation-local temporary ROM paths instead of the shared `/tmp/dummy.smc`.

Result: editing one patch rebuilds only that patch; editing `symbols.inc` rebuilds its consumers. Unchanged patches, including `fastrom_base`, are reused.

Implemented: `catalog_builder patches` invokes the shared patch-building module, builds the standalone patched Asar with CMake in Release mode, updates `build/patches/*.ips` with per-patch fingerprint files, and packages the patch catalog introduced in step 4. `asm` selects the ASM directory (default `patches/src/`); `asar_source` selects the patched Asar checkout containing `src/CMakeLists.txt` (default `asar/`); `asar_executable` takes precedence and skips the CMake build. `output_directory` selects the artifact directory. Step 5 reads these settings from local configuration and resolves relative paths from the repository root. A shared `cache.lock` protects updates, and temporary files keep failed assembly from publishing incomplete outputs. Existing tracked IPS consumers and Python scripts remain until step 13.

Dependency tracking supports only standalone, single-line `incsrc "literal/path"` directives, resolved relative to the including file. Includes are followed recursively, including those in conditional branches. It does not evaluate Asar defines or macros, resolve include search paths, or track `incbin` or other file-reading directives. Extend the scanner before introducing those dependency forms; otherwise cached patches could become stale.

## 4. Build the patch catalog (implemented)

Define a runtime patch catalog and its encoder/decoder using the existing bincode envelope convention. Add a `patches` subcommand to the builder CLI and include it in `all`. The subcommand prepares Asar, assembles outdated patches, and packages the catalog. Bundle separate named IPS patches and their matching symbol manifest. Application order, phases, and optional patch selection stay in patcher code, including the initial `fastrom_base` transformation.

Symbol manifest generation is implemented: `%export_symbol(InternalName, exported_name, $Value)` in `symbols.inc` defines an ASM symbol and an explicitly named `export_` label. The standalone `symbols.asm` entry point generates a cached symbol file alongside the IPS artifacts. Plain `PatchSymbols` and `PatchIps` structs use Serde's `Deserialize` derive and `deny_unknown_fields` to import in-memory entries, rejecting missing or extra exports and patches. `build_patches` returns typed IPS payloads read under the assembly cache lock and this typed manifest. Runtime consumers switch from their existing address constants to the catalog during the later patcher integration.

Read `z3-json-data` directly to extract stable location mappings, ROM addresses, item receipt IDs, and prize encodings for the patch catalog. Share source-reading types with the logic builder, without depending on the generated logic catalog.

Result: one artifact supplies the fixed patches and symbols to native and browser patchers, together with the source-derived item/location patching data.

Implemented: `catalog_builder patches` writes `build/patch_catalog.bin`, and `all` now builds all three catalogs. The patch catalog has streaming Zstd/bincode writer and reader APIs with its magic bytes and `type_hash` envelope. Logic and patch builders share `catalog_builder::z3_json_data` types and file readers. The patch builder reads `rooms/` and `items.json` directly, retains authored IDs and ordered prize offsets, and excludes the fixed flute-activation event from placement locations. Catalog files are written directly. Local configuration selects source paths, assembler options, output directory, and compression level; step 5 provides shared configuration and skips unchanged catalog packaging alongside the existing assembly cache.

## 5. Add shared configuration and caching

Add catalog caching so the existing `all` subcommand reuses unchanged catalogs:

```sh
cargo run -p catalog_builder -- all
```

Default source paths to the root-level `z3-json-data` and `ALTTPRetiling` submodules, resolved from the repository root. Check in `catalog-build.default.toml` and copy it to the Git-ignored `catalog-build.toml` if the local file is missing when a builder or bundled build runs. Read that local configuration without overwriting or merging it. The template is the single source of defaults; all configuration fields are required except the optional `asar_executable`. The builder CLI only selects which catalogs to build; all options come from local configuration, without custom environment variables.

Use this default template:

```toml
logic_source = "z3-json-data"
retiling_source = "ALTTPRetiling"
asm = "patches/src"
asar_source = "asar"
output_directory = "build"
tile_fingerprints = "data/tile_fingerprints.json"
compression_level = 3

# Override the automatically built assembler with our patched Asar executable.
# IPS output support is required; an upstream Asar executable will not work.
# asar_executable = "/path/to/patched/asar"
```

The required `compression_level` configuration value is set to `3` in the template. Compression level is part of the catalog output fingerprint, not the Asar or IPS fingerprints, so changing it does not trigger assembly.

Asar requires an initialized submodule, CMake, and a C++ toolchain. The source data repositories are root-level submodules; local configuration can select other checkouts. Do not download repositories automatically.

Use the ignored top-level `build/` directory for catalogs, IPS files, and cache metadata. Use each source repository’s checked-out Git `HEAD` commit and canonical path in its catalog fingerprint. Staged, unstaged, or untracked changes force a rebuild without recording a reusable fingerprint. Non-Git source overrides also rebuild every time. Ignored files do not affect Git status; source inputs must be tracked or unignored. Include the external tile fingerprint index for retiling and assembled IPS/symbol artifacts for patches. Collect code inputs at crate granularity at compile time. The entire `catalog_builder` crate and workspace manifest/lockfile affect all three fingerprints. Logic additionally includes `logic_catalog` and `common_types`; retiling includes `retiling_catalog`; patches include `patch_catalog`, `common_types`, and `logic_catalog`, whose enums are used by source deserialization. Rust sources and Cargo manifests are collected recursively within each selected crate. Generator, patcher, and unrelated tool sources do not invalidate catalogs. Include the effective compression level rather than configuration file text. Fingerprints do not depend on debug versus release profiles.

Use one shared cache lock during updates and snapshot copying. Write catalogs and their fingerprint metadata directly; invalidate old fingerprints before rebuilding and record new fingerprints only after a successful write. Interrupted catalog writes may leave incomplete files, which the next invocation rebuilds. Individual subcommands, `all`, and the randomizer's build script share this library implementation.

Implemented: `catalog_builder` loads the root configuration, resolves all relative paths from the repository root, and routes individual catalog commands and `all` through `CatalogCache`. All subcommands read their options from configuration, with fixed catalog filenames inside the output directory. The cache holds `build/cache.lock` until dropped, including any snapshot copying a library caller performs while holding it. Catalogs and fingerprint metadata are written directly, and metadata is invalidated before rebuilding so failed or interrupted writes cannot produce a false cache hit. Asar’s CMake cache and the existing per-ASM dependency cache remain independent. The future bundled build can use the same library API; its integration is step 10.

Result: manual and automatic builds reuse the same cache, which survives `cargo clean`.

## 6. Make seed identities independent of catalogs

Retain authored `(room_id, item_id)` pairs in logic-catalog item locations and replace saved `item_location_idx` values with these pairs. Establish that existing IDs are permanent and removed IDs are not reused. Item names remain the item identity; receipt IDs and ROM addresses are patching details. Remove location ROM addresses, item receipt IDs, and prize patch bytes from the logic catalog; the patch builder reads these directly from `z3-json-data`.

Add an integer seed `format_version` starting at `1`, following the [versioning rules](plans/README.md#saved-seed-format): bump for meaningful interpretation changes, not compatible extensions. Keep generator metadata and catalog hashes as provenance, without requiring an old catalog or patcher version. Make the saved-seed types available to generation and patching without making the patcher depend on the generator's implementation.

Implemented: the shared `seed_data` crate owns settings, `SeedData`, metadata, and item placements. The `common_types` crate owns the single definitions of `SourceRoomId` and `ItemLocationId`; catalogs depend on it without depending on settings or saved seed data. Generator and patcher expose the shared seed type without depending on one another. Logic-catalog locations store an `id` containing the authored room/item pair; saved placements store `item_location_id`. Graph effects still use internal location indices. Logic-catalog items contain names, and locations contain their stable identity, display name, and vertex; ROM addresses, receipt IDs, and prize bytes remain in the patch catalog. `SeedData::format_version` is an integer and writers use `SEED_FORMAT_VERSION = 1`. Metadata includes logic and retiling catalog hashes for provenance. The catalog cache fingerprints the shared types for logic and patch outputs.

Result: reordering a logic catalog does not change saved location identities.

## 7. Define stable world layout

Define serializable `WorldLayout` as part of persisted `SeedData`. Initially record stable area identities and assigned themes, leaving area positions and edge variants out of the types. For the prototype, select one whole-game theme and retain implicit vanilla area placement and connections. Extend the layout with positions in the world grid and enabled edge variants when rearrangement is implemented. Keep fine-grained graphical content and runtime allocation choices out of saved seeds.

Reuse `RetilingCatalog` as patching input alongside `WorldLayout`; do not introduce a separate seed-specific retiling type or persist graphical content in `SeedData`. The native CLI can pass its already-loaded catalog unchanged. The future web backend filters the current catalog to the content needed by the saved layout before sending it to the client. Filtering is a transport optimization, not a required native preparation step. Full and filtered catalogs use the same type and have no backward-compatibility requirement.

For eventual saved-seed compatibility, keep area, theme, and edge-variant identities stable and provide mappings for retired or restructured choices. Preserve layout connectivity and gameplay meaning when resolving old choices against current content. Catalog hashes remain provenance rather than selectors for archived catalogs. Do not implement migrations or legacy readers during prototyping.

Implemented: `seed_data::WorldLayout` contains an array of `AreaLayout` entries with `area_id` (the top-left vanilla map ID) and `theme_name` (the authored theme identity). `SeedData.world_layout` replaces its former whole-game `theme_name` field. Area positions and edge variants are deferred. Web catalog filtering is deferred until the web service is introduced.

Result: generation saves high-level layout decisions; patching selects their graphical content from the current full or filtered retiling catalog.

## 8. Implement the uniform generator library (implemented)

Expand the settings item pool and uniformly shuffle it across eligible ordinary locations. Place maps and compasses uniformly among all non-key, non-prize locations within their own dungeon, then shuffle ordinary items globally across the remaining locations. Keep SmallKey and BigKey placements vanilla, shuffle pendants and crystals only among prize locations, and record all placements. Return an error if the ordinary pool quantity does not match the available locations. Preserve tech and proficiency settings without using them for placement yet.

Accept settings, generation catalogs, the selected theme, and deterministic randomness from the caller. Produce the prototype `WorldLayout`, recording the selected theme for each area with vanilla positions implicit. Keep clocks, filesystem access, and seed-name generation outside the generator library.

Implemented: `generator::generate` accepts settings, logic and retiling catalogs, the selected theme, caller-prepared metadata, and a caller-owned RNG. The logic catalog retains each location’s vanilla item index for dungeon restrictions, prize shuffling, and fixed keys. The result records all pickups and selected mapped areas that provide the requested theme, sorted by vanilla map ID; omitted areas retain vanilla content. No clocks, files, ROM access, or RNG seeding occur inside the generator.

Result: generation produces structured seed data without logical placement or ROM patching.

## 9. Add seed serialization and the generation CLI

Create `crates/cli` with binary `z3-map-rando`, initially supporting generation through external catalogs. Implement the arguments and defaults in [plans/cli.md](plans/cli.md), including bundled checked-in settings.

Choose or accept a `u64` RNG seed. Generate and persist the fresh nine-character seed name using the agreed separate naming RNG. Record the UTC timestamp, package version, clean Git commit when available, and catalog provenance.

Stream JSON through Zstd when saving, and deserialize through a buffered Zstd decoder when loading. Default seed output to `z3-map-rando-<SEED-NAME>.json.zst`. Naming remains fresh when `--rng` repeats, while placements remain reproducible with identical generation inputs.

Result: a usable generation-only CLI that writes a complete compressed seed.

## 10. Add automatic generation and embedding to bundled builds

Add the default-off `bundle-catalogs` feature and optional builder dependency. The CLI's `build.rs` calls the shared update operation only with this feature enabled. Track source directories/files, configured paths, and assembler changes using Cargo's rerun directives. Track the local configuration file, creating it from the template only for bundled builds or standalone builder invocations.

While holding the shared cache lock, update `build/` and copy the completed catalogs into this build's `OUT_DIR`. Embed those snapshots, not mutable cache paths. Debug and release share expensive generation and assembly work while each compilation has consistent inputs.

```sh
cargo build -p cli --release --features bundle-catalogs
```

Result: bundled builds automatically create or update their assets. Ordinary `cargo check` without the feature requires neither catalogs, source repos, nor Asar. External catalog arguments can override embedded catalogs.

## 11. Extract retiling patching

Refactor the ROM-building flow in `theme_check` into a reusable patcher operation consuming a ROM, decoded seed, current retiling catalog, current patch catalog, and customization settings. Select area content using the saved `WorldLayout`; accept either a full or filtered `RetilingCatalog` through the same interface. The CLI passes its loaded catalog unchanged. Consume IPS bytes from the patch catalog instead of filesystem paths. Resolve vanilla graphics references from the ROM and compile the selected content into the current runtime layout using current symbols.

Result: native and future browser patchers share one interface accepting the current retiling catalog. The core does not need the logic catalog and has no filesystem, clock, network, or RNG dependency.

## 12. Add item-placement patching

Use the patch catalog's source-derived location mappings and current item and prize encodings to translate saved identities. Extend this data where necessary for the patched pickup carriers. Implement the location-specific writes and hooks required by the prototype's randomized pickups.

Use the current patcher and matching patch catalog independently of the catalog versions used for generation; customization remains a patch-time input. Do not implement seed migrations or legacy readers during prototyping.

Result: the patcher applies saved placements without the original logic catalog or saved ROM-write addresses.

## 13. Complete CLI patching and remove tracked IPS artifacts

Wire `--seed` plus `--rom` to patching, and generation plus `--rom` to generation followed by patching. Save the seed before patching in combined operation. Reuse the saved seed name for default ROM output paths.

Move remaining IPS consumers and development instructions to generated outputs or the patch catalog, then remove `patches/ips/` files from source control. Update or retire `scripts/build_patches.py` and `scripts/build_asar.py` so there is one assembly and caching implementation. Document the local configuration, source-directory overrides, and Asar prerequisites.

Result: the CLI generates, patches, or does both based on its arguments, with no user-specified catalogs needed for a bundled binary.

## Verification during implementation

Use the existing checks and temporary development checks appropriate to each step; do not add permanent tests or validation without an explicit request. Check compressed catalog round trips, unchanged-cache reuse, individual ASM edits, shared include edits, and reuse between debug and release builds. Changing compression level should update catalog outputs while reusing unchanged assembly artifacts. Exercise seed serialization and CLI generation before moving on to ROM patching. Check that patch-only operation works without the logic catalog, uses the current retiling catalog directly, and leaves persisted seed data unchanged. Check that saved seeds remain usable after catalog rebuilds without selecting archived catalogs.
