# Overview

The goal of this project is to create a randomizer for generating randomized *A Link to the Past* games, featuring a randomly rearranged and rethemed overworld. The randomizer will be usable primarily as a web service, but also as a library and command-line tool.

# Current status

An [overworld editor](https://github.com/blkerby/Z3OverworldEditor) exists, based around the ability to define and use custom palettes and tilesets with a degree of flexibility beyond what the vanilla game engine provides. The editor is functional enough to enable artists to create new themes (in progress [here](https://github.com/kjbranch/ALTTPRetiling)), but the work needed to translate this into a playable ROM is mostly still in a planning stage. The editor also still needs development, 1) to define structured variants of area perimeter/edge segments so they can connect cleanly with other areas after rearrangement, 2) to support customizing animations, including dungeon openings, 3) to support redrawing the Mode 7 overworld map. Currently the editor focuses on retiling the overworld BG2 layer and associated collision data, as this is primarily what is needed for the randomizer project; it may later be extended with support to customize other game elements such as enemies, entrances, secrets, items, and dungeons.

A prototype generator and CLI now produce structured seeds with uniform item placement; logical item placement and the web service remain planned. A [design doc](https://docs.google.com/document/d/1skZsIxZLKbCC8C_-3b3TTZVwblzNkN2ZTXraZ4Zxzkc) describes the planned tier-based item placement logic along with several balance-oriented gameplay changes.

The current focus is on the technical foundation for the project, particularly the game engine changes needed to support the flexible palette and tileset system. For detailed plans, see the docs in [plans](plans/README.md).

# How to generate a prototype seed

From the repository root, build the generation catalogs and invoke the CLI:

```sh
cargo run -p catalog_builder -- logic
cargo run -p catalog_builder -- retiling
cargo run -p cli -- --rng 12345
```

The CLI uses bundled default settings and catalogs from `build/`, and writes `z3-map-rando-<SEED-NAME>.json.zst` in the working directory. Omit `--rng` to choose a numeric seed randomly. Use `--settings settings.json`, `--theme Desert`, or `--output-seed seed.json.zst` to override generation inputs or the output path. Set `--compression-level 18` to use a higher Zstd compression level instead of the default `3`. If catalogs are stored elsewhere, specify `--logic-catalog` and `--retiling-catalog`. Run `cargo run -p cli -- --help` for the implemented arguments.

The prototype shuffles ordinary items globally, maps and compasses within their dungeon, and pendants and crystals among prize locations. SmallKey and BigKey placements remain vanilla. It records the selected theme for each supported area with vanilla placement implicit. It does not check logical progression or produce a ROM. See [the CLI design](plans/cli.md) for planned patching and bundled catalogs.

# How to test current engine changes

Clone the repo:

```sh
git clone --recurse-submodules https://github.com/blkerby/z3-map-rando.git
```

Install Rust and Cargo with [rustup](https://www.rust-lang.org/tools/install); for example, on Linux or macOS:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Build the retiling catalog, then a test ROM:

```sh
mkdir -p build
cargo run -p catalog_builder -- retiling
cargo run -p theme_check -- path/to/vanilla.sfc path/to/output.sfc build/retiling_catalog.bin --theme Base
```

The input must be the unheadered 1 MiB Japanese 1.0 ROM with SHA-256 digest `794e040b02c7591b59ad8843b51e7c619b88f87cddc6083a8e7a4027b96a2271`. `theme_check` reads the selected theme from the catalog and produces a 4 MiB ROM.

If you notice any issue while testing or if you run into any trouble following these instructions, please reach out in the [Discord](https://discord.gg/Mxb5zYZeVj) to let us know. Even in this early phase of the project, playtesting is very helpful!

# How to build the retiling catalog

The catalog bundles artwork and overworld layouts from `ALTTPRetiling` into one file for building retiled ROMs. Building it does not require a game ROM. Rebuild the catalog whenever you update your copy of `ALTTPRetiling`.

From the repository root:

```sh
mkdir -p build
cargo run -p catalog_builder -- retiling
```

Initialize the source submodules from the repository root:

```sh
git submodule update --init --recursive
```

To build all three catalogs from the `z3-json-data` and `ALTTPRetiling` submodules into `build/`, run:

```sh
cargo run -p catalog_builder -- all
```

Builders copy `catalog-build.default.toml` to the Git-ignored `catalog-build.toml` if missing, then read that local configuration without overwriting or merging it. Edit that local file to set source paths, the output directory, tile fingerprints, assembler options, and compression level. All relative paths are resolved from the repository root. Building the patch catalog requires an initialized patched Asar submodule, CMake, and a C++ compiler unless `asar_executable` supplies an existing patched executable. Each catalog uses its fixed filename inside the configured output directory; the subcommands have no positional arguments.

Catalog payloads use Zstd compression, defaulting to level `3` for every Cargo profile. Set `compression_level` in `catalog-build.toml` to change the level. Use level `18` when prioritizing smaller release artifacts. Unchanged catalogs are reused using fingerprints of source Git commits, Rust code, compression level, and external inputs. Dirty Git checkouts and non-Git source directories rebuild every time; ignored files do not affect Git status. The shared cache under `build/` survives `cargo clean`.

```toml
# In catalog-build.toml:
compression_level = 18
```

# How to build the patch catalog

The patch catalog bundles the separate IPS patches, their typed symbol manifest, and item/location patching data read directly from `z3-json-data`. It uses the same streaming Zstd-compressed bincode format as the other catalogs and requires neither a ROM nor a generated logic catalog.

From the repository root:

```sh
cargo run -p catalog_builder -- patches
```

This writes `build/patch_catalog.bin`, reusing cached assembly artifacts. Settings in `catalog-build.toml` select the data source, ASM directory, Asar checkout or executable, artifact directory, and compression level. Changing compression does not invalidate assembly. Unchanged catalog packaging is skipped through the shared cache and configuration. Existing ROM tools still consume the tracked IPS files until their later integration with the patch catalog.

# How to build the tile fingerprint index

The fingerprint index helps the catalog builder recognize graphics from the original game and store references to them. The index contains no artwork and allows the catalog to be built without a ROM.

An index is already included in this repository. To regenerate it, use the same Japanese 1.0 ROM described above and run this command from the repository root:

```sh
cargo run -p build_tile_fingerprints -- path/to/vanilla.sfc data/tile_fingerprints.json
```

See the [catalog plan](plans/README.md#retiling-catalog) for implementation details.

# How to build the patches from source

The instructions above use the IPS patches already checked into the repo. To build cached patches locally, install a C++ compiler and CMake; for example, on Ubuntu:


```sh
sudo apt install build-essential cmake
```

From the repository root:

```sh
cargo run -p catalog_builder -- patches
```

This automatically builds the repository's patched Asar submodule in Release mode under `build/asar/`, using CMake's incremental compilation, writes IPS files and their fingerprints under `build/patches/`, and packages `build/patch_catalog.bin`. Initialize the submodule when cloning, as shown above. Only the standalone assembler target is compiled. Each patch is reused when its source files, transitive includes, assembler contents, and assembly options are unchanged. Builder implementation changes also invalidate the fingerprints. Updates hold `build/cache.lock`; temporary ROMs and outputs are local to each invocation, and failed assembly leaves the previous completed patch available.

The builder also caches `build/patches/symbols.sym`, exported independently from `patches/src/symbols.asm`, and imports it into the catalog's typed `PatchSymbols`. `%export_symbol` in the shared interface marks Rust-facing symbols; missing or unconsumed exports fail the build. See [the symbol manifest design](plans/patches.md#symbol-manifest).

`asm` selects the ASM source directory, defaulting to `patches/src/`. `asar_source` selects the patched Asar checkout containing `src/CMakeLists.txt`, defaulting to `asar/`. `output_directory` selects the artifact directory, defaulting to `build/`. An explicit `asar_executable` path in `catalog-build.toml` skips the automatic assembler build and takes precedence over `asar_source`:

```toml
# In catalog-build.toml:
asar_executable = "path/to/patched/asar"
```

We use a patched version of Asar with IPS output support; an upstream Asar executable will not work. The new command leaves tracked `patches/ips/` files unchanged. Existing ROM-building tools still use those tracked patches until the later patch-catalog integration; the old Python scripts remain available for that workflow.

Dependency tracking supports only standalone, single-line `incsrc "literal/path"` directives. Paths are relative to the including file and followed recursively, including conditional branches. The scanner does not evaluate defines or macros, resolve include search paths, or track `incbin` or other file-reading directives. Extend it before adding any of these forms, or cached outputs could become stale.
