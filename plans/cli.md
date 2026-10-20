# CLI

The `z3-map-rando` binary calls the generator and patcher libraries. Arguments select generation, patching, or both; there are no subcommands.

Implemented: generation, patching saved seeds, and generation followed by patching, with the arguments below and overrides for all three catalogs. Catalog bundling is implemented through the default-off `bundle-catalogs` feature.

## Arguments

| Argument | Meaning | Default |
| --- | --- | --- |
| `--settings <PATH>` | Settings JSON used for generation | Bundled `data/settings.json` |
| `--seed <PATH>` | Existing Zstd-compressed seed JSON used for patching | Generate a new seed |
| `--rom <PATH>` | Vanilla ROM to patch | Generate a seed file only |
| `--theme <NAME>` | Whole-game theme selected during generation | `Base` |
| `--rng <U64>` | Numeric seed for item randomization | Choose randomly |
| `--output-seed <PATH>` | Output path for Zstd-compressed seed JSON | `z3-map-rando-<SEED-NAME>.json.zst` |
| `--compression-level <I32>` | Zstd compression level for the seed output | `3` |
| `--output-rom <PATH>` | Output path for the patched ROM | `z3-map-rando-<SEED-NAME>.sfc` |
| `--transition-asset-phase <PHASE>` | Patch-time transition asset loading: `pre-scroll`, `scroll`, or `post-scroll` | `scroll` |
| `--version` | Shared project version | |

`--settings` and `--seed` are mutually exclusive, and `--seed` requires `--rom`. When neither is supplied, use the bundled default settings. `--theme` and `--rng` apply to generation; patching an existing seed uses its recorded world layout and item placements.

| Input | ROM supplied | Operation | Outputs |
| --- | --- | --- | --- |
| Settings or defaults | No | Generate | Seed file |
| Settings or defaults | Yes | Generate, then patch | Seed file and ROM |
| Existing seed | Yes | Patch | ROM |

Output path arguments override filenames for the selected operation. Default paths are relative to the current working directory. Explicit paths do not change the stored seed name. Seed output and compression options apply only to generation; patch-only operation does not rewrite the saved seed. Write seed JSON with indentation through a Zstd encoder using `--compression-level` (default `3`), and progress and diagnostics to stderr. When `--rom` is supplied, read and verify it before loading catalogs or generating a seed. In combined operation, save the seed file before patching. Generation-only runs without `--settings` end with a tip showing how to patch the saved seed and how to add `--rom` to the initial command to generate and patch together. Read saved seeds through a buffered Zstd decoder and `serde_json::from_reader`, without buffering the complete decompressed JSON.

```sh
# Generate a compressed seed file using default settings.
z3-map-rando

# Generate with custom settings and a specified numeric RNG seed.
z3-map-rando --settings settings.json --rng 12345

# Patch an existing seed.
z3-map-rando --seed seed.json.zst --rom vanilla.sfc

# Generate and patch using default settings.
z3-map-rando --rom vanilla.sfc

# Override both output paths.
z3-map-rando --rom vanilla.sfc --output-seed seed.json.zst --output-rom seed.sfc
```

## Prototype generation

Expand the settings' item pool into individual pickups and uniformly randomize them across available locations. Maps and compasses are placed uniformly among all non-key, non-prize locations within their own dungeon. Ordinary items are shuffled globally across the remaining locations. SmallKey and BigKey placements and the Waterfall/Pyramid fairy exchange items remain vanilla. Pendants and crystals are shuffled only among prize locations. Record all of these placements in the output, including fixed placements. Return an error if the ordinary item pool quantity does not match the available locations.

Use one selected theme for the whole game and record each area’s theme in `WorldLayout`, with vanilla positions implicit. Logical item placement and overworld rearrangement are deferred. Preserve tech and proficiency settings in the output even though this prototype does not use them for placement.

The CLI handles files, timestamps, numeric seed selection, seed naming, and bundled assets. Libraries receive their inputs as arguments.

## Seed name and metadata

The initial serializable output types are defined in [`output.rs`](../crates/seed_data/src/output.rs). The planned format and compatibility changes are described in the [architecture plan](README.md#saved-seed-format). Persist a seed name in metadata when generating a new seed. Loading `--seed` reuses its saved name for default output filenames.

Generate a fresh nine-character name using the numeric RNG seed and the current Unix timestamp in nanoseconds:

1. Start with 32 zero bytes.
2. Copy the `u64` numeric seed's eight little-endian bytes into bytes `0..8`.
3. Copy the timestamp's sixteen little-endian bytes into bytes `8..24`.
4. Seed a separate `rand::rngs::StdRng` with those bytes.
5. Choose nine characters uniformly from this alphabet:

```text
256789BCDFGHJKLMNPQRSTVWXYZbcdfghjkmnpqrstvwxyz
```

The alphabet omits vowels and characters resembling vowels to reduce the chance of forming words. Naming randomness is independent of placement randomness. Repeated generation with the same `--rng` intentionally produces a fresh name; placements are reproducible with the same generator, catalogs, and settings.

Store the numeric RNG seed as a string in JSON to preserve precision. Record the generation timestamp in RFC 3339 UTC format and the SHA-256 hashes of the complete encoded logic and retiling catalogs used for generation. Catalog hashes are provenance only; patching does not require the original catalog versions. Save placements using stable authored `room_id` and `item_id` values, not catalog indices or ROM addresses.

The saved seed includes a `format_version` and stable `WorldLayout` choices, not graphical content. At patching time the CLI passes its already-loaded current `RetilingCatalog` unchanged to the patcher, which selects content using the saved layout. The latest patcher supports old seed formats and uses the current retiling and patch catalogs. Patch-time customization can therefore expose new options for existing seeds.

Read the shared project version from the root [`VERSION`](../VERSION) file through `common_types::VERSION`. The CLI version output and seed metadata use this value, independent of Cargo package versions. The first component identifies a regular release and the second a rare hotfix for a severe bug: `7.0`, `7.1`, `7.2`, then `8.0`. The CLI and future web service are released together. Local development leaves `generator_commit` as `None`, without Git queries or metadata rebuild tracking. When release packaging is introduced, CI will write a release metadata file containing the version from `VERSION` and full source commit, packaged with the application for the CLI and later web backend to load. Generating and loading that file are deferred until release packaging is set up.

## Bundled assets and builds

The CLI crate, `crates/cli`, embeds default settings from the checked-in [`data/settings.json`](../data/settings.json).

The `bundle-catalogs` Cargo feature builds all three catalogs and embeds their files directly from the configured output directory. Generation consumes the embedded logic and retiling catalogs; patching consumes the retiling and patch catalogs. The feature is disabled by default so ordinary `cargo check` and workspace builds work without generated catalogs. Explicit catalog paths override bundled bytes. If an unbundled binary receives no override, it reads the required catalogs from `build/logic_catalog.bin`, `build/retiling_catalog.bin`, and `build/patch_catalog.bin` relative to the working directory:

- `--logic-catalog <PATH>`
- `--retiling-catalog <PATH>`
- `--patch-catalog <PATH>`

Generation needs the logic and retiling catalogs to produce item placements and saved world-layout choices. Patching needs the saved seed, ROM, current retiling catalog, and current patch catalog matching the patcher. It does not need the logic catalog or archived versions of generation catalogs. Bundled binaries provide the catalogs without additional user arguments.

The `catalog_builder` crate exposes separate library modules through one builder CLI with `logic`, `retiling`, `patches`, and `all` subcommands. All read their options from shared configuration, with fixed catalog filenames inside the output directory. It can populate the ignored top-level `build/` cache independently of the randomizer CLI feature:

```sh
cargo run -p catalog_builder -- all
```

Source paths default to the root-level `z3-json-data` and `ALTTPRetiling` submodules. Builders and bundled builds copy the checked-in `catalog-build.default.toml` to the Git-ignored `catalog-build.toml` if missing, then read the local file without overwriting or merging it. Relative paths are resolved from the repository root. The builder CLI only selects which catalogs to build; all options come from local configuration.

By default, build the repository's patched Asar submodule incrementally with CMake under `build/asar/`, using Release mode and only the standalone target. Local configuration uses `asm` and `asar_source` to select the ASM directory and patched Asar checkout. An explicit `asar_executable` path in the local configuration skips building Asar and takes precedence over the source path. The template comment must warn that this override needs our patched IPS output support and that upstream Asar will not work. This requires initialized Asar sources, CMake, and a C++ toolchain, without automatic downloads. The builder assembles outdated patches, tracking recursive literal `incsrc` dependencies, and bundles them with current symbols and location mappings. IPS files become local artifacts rather than tracked data. Location mappings and item encodings come directly from `z3-json-data`, not the logic catalog. Application order and phases stay in patcher code. Symbol manifest generation and patch-catalog packaging are implemented: `symbols.asm` exports the shared interface into the catalog's typed `PatchSymbols`, with exact export correspondence checked during building. Shared configuration and catalog caching are implemented. IPS files are generated only under the local artifact directory; the tracked IPS files and old Python builder scripts have been removed. Catalog fingerprints use source Git commits, Rust code inputs collected at crate granularity for the shared builder and each catalog’s supporting crates, effective compression level, and external inputs. Dirty or non-Git source checkouts rebuild every time. The cache lock spans catalog updates.

With `bundle-catalogs`, the CLI's `build.rs` invokes the existing `CatalogCache::update_all()` on every build. One deliberately nonexistent rerun path ensures the builder cache can check for changes without duplicating source tracking. It uses the shared configuration and cache, builds outdated outputs under the shared lock, and supplies the configured output paths for direct `include_bytes!` embedding. Catalogs are not copied to `OUT_DIR`. Debug and release share expensive generation and assembly work. The CLI crate rebuilds each bundled invocation, even when the catalogs are reused. Without the feature, skip catalog generation and embedding and omit the builder dependency.

The build script releases the cache lock before Rust reads the catalogs for embedding. Avoid concurrent cache updates during compilation; there are no per-build catalog snapshots.

```sh
cargo build -p cli --release --features bundle-catalogs
```

Gate embedding declarations and the builder dependency with the feature. Bundled builds, including `cargo check --all-features`, need the source data and Asar for any artifacts requiring generation. The existing cache handles source changes and configured paths whenever the bundled build invokes it. See [the implementation sequence](../plan.md).

The CLI owns the embedded assets; the generator and patcher libraries accept data from callers. `patcher::patch_rom` now consumes seed data and the current catalogs to build a retiled ROM with the saved item placements. Patch-time `transition_asset_phase` defaults to `Scroll`. CLI patching integration is implemented; `theme_check` uses the shared retiling operation without randomized item placements.
