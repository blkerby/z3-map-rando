# CLI

The `z3-map-rando` binary calls the generator and patcher libraries. Arguments select generation, patching, or both; there are no subcommands.

## Arguments

| Argument | Meaning | Default |
| --- | --- | --- |
| `--settings <PATH>` | Settings JSON used for generation | Bundled `data/settings.json` |
| `--seed <PATH>` | Existing Zstd-compressed seed JSON used for patching | Generate a new seed |
| `--rom <PATH>` | Vanilla ROM to patch | Generate a seed file only |
| `--theme <NAME>` | Whole-game theme selected during generation | `Base` |
| `--rng <U64>` | Numeric seed for item randomization | Choose randomly |
| `--output-seed <PATH>` | Output path for Zstd-compressed seed JSON | `z3-map-rando-<SEED-NAME>.json.zst` |
| `--output-rom <PATH>` | Output path for the patched ROM | `z3-map-rando-<SEED-NAME>.sfc` |
| `--version` | Generator version and commit, when available | |

`--settings` and `--seed` are alternative inputs. When neither is supplied, use the bundled default settings. `--theme` and `--rng` apply to generation; patching an existing seed uses its recorded theme and item placements.

| Input | ROM supplied | Operation | Outputs |
| --- | --- | --- | --- |
| Settings or defaults | No | Generate | Seed file |
| Settings or defaults | Yes | Generate, then patch | Seed file and ROM |
| Existing seed | Yes | Patch | ROM |

Output path arguments override filenames for the selected operation. Default paths are relative to the current working directory. Explicit paths do not change the stored seed name. Write seed JSON with indentation through a Zstd encoder, and progress and diagnostics to stderr. In combined operation, save the seed file before patching. Read saved seeds through a buffered Zstd decoder and `serde_json::from_reader`, without buffering the complete decompressed JSON.

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

Expand the settings' item pool into individual pickups and uniformly randomize them across available locations. Maps and compasses are randomized within their own dungeon. Keys and dungeon prizes keep their vanilla placements. Record all of these placements in the output, including fixed placements.

Use one selected theme for the whole game. Logical item placement and overworld rearrangement are deferred. Preserve tech and proficiency settings in the output even though this prototype does not use them for placement.

The CLI handles files, timestamps, numeric seed selection, seed naming, and bundled assets. Libraries receive their inputs as arguments.

## Seed name and metadata

The initial serializable output types are defined in [`output.rs`](../crates/generator/src/output.rs). The planned format and compatibility changes are described in the [architecture plan](README.md#saved-seed-format). Persist a seed name in metadata when generating a new seed. Loading `--seed` reuses its saved name for default output filenames.

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

Store the numeric RNG seed as a string in JSON to preserve precision. Record the generation timestamp in RFC 3339 UTC format and the SHA-256 hash of the complete encoded logic catalog used for generation. Catalog hashes are provenance only; patching does not require these catalogs. Save placements using stable authored `room_id` and `item_id` values, not catalog indices or ROM addresses.

The saved seed includes a `format_version` and the selected retiling content. The latest patcher supports old seed formats and uses its current patch catalog. Patch-time customization can therefore expose new options for existing seeds.

Capture the generator version from `CARGO_PKG_VERSION` at compile time. A build script captures the full Git commit when Git is available and the working tree is clean; otherwise, metadata contains `None` for the commit.

## Bundled assets and builds

The planned CLI crate, `crates/cli`, embeds default settings from the checked-in [`data/settings.json`](../data/settings.json).

The `bundle-catalogs` Cargo feature embeds the logic, retiling, and patch catalogs into the CLI binary. It is disabled by default so ordinary `cargo check` and workspace builds work without generated catalogs. Unbundled binaries load external assets through these arguments, which also override bundled assets when supplied:

- `--logic-catalog <PATH>`
- `--retiling-catalog <PATH>`
- `--patch-catalog <PATH>`

Generation needs the logic and retiling catalogs to produce placements and saved retiling content. Patching needs the saved seed, ROM, and the current patch catalog matching the patcher. It does not need either generation catalog. Bundled binaries provide the catalogs without additional user arguments.

The `catalog_builder` crate exposes separate library modules through one builder CLI with `logic`, `retiling`, and `all` subcommands; `patches` is planned. It can populate the ignored top-level `build/` cache independently of the randomizer CLI feature:

```sh
cargo run -p catalog_builder -- all
```

Source paths default to sibling `../z3-json-data` and `../ALTTPRetiling` directories. Builders and bundled builds copy the checked-in `catalog-build.default.toml` to the Git-ignored `catalog-build.toml` if missing, then read the local file without overwriting or merging it. Relative paths are resolved from the repository root. The builder CLI also accepts path overrides.

By default, build the repository's patched Asar submodule incrementally with CMake under `build/asar/`, using Release mode and only the standalone target. An explicit `asar` path in the local configuration skips building Asar. The template comment must warn that this override needs our patched IPS output support and that upstream Asar will not work. This requires initialized Asar sources, CMake, and a C++ toolchain, without automatic downloads. The builder assembles outdated patches, tracking recursive literal `incsrc` dependencies, and bundles them with current symbols and location mappings. IPS files become local artifacts rather than tracked data. Location mappings and item encodings come directly from `z3-json-data`, not the logic catalog. Application order and phases stay in patcher code. Symbol manifest generation is planned work; current assembly only emits IPS files.

With `bundle-catalogs`, the CLI's `build.rs` calls the same library operations to update missing or outdated artifacts automatically. Fingerprints include source inputs, builder code, assembler contents, and configuration, independent of Cargo profile. A shared lock covers updates and copying the completed catalogs into `OUT_DIR`; the CLI embeds those copies using `include_bytes!`. Debug and release share the expensive cache work, while each build embeds a consistent snapshot. Without the feature, skip catalog generation and embedding.

```sh
cargo build -p cli --release --features bundle-catalogs
```

Gate embedding declarations and the builder dependency with the feature. Bundled builds, including `cargo check --all-features`, need the source data and Asar for any artifacts requiring generation. Cargo rerun directives track source changes and path overrides. See [the implementation sequence](../plan.md).

The CLI owns the embedded assets; the generator and patcher libraries accept data from callers. The current ROM-building flow in `theme_check` still needs to become a reusable patcher entry point that consumes seed data.
