# Overview

This is the repository for the Link to the Past Map Rando project, a randomizer that generates randomized *Zelda: A Link to the Past* games, featuring a randomly rearranged and rethemed overworld.

# Current status

The randomizer is available in pre-release form as a CLI and as a web service. It can generate playable games and needs only a bit more bug-fixing and polish before an initial release. The "Desert" theme is complete; more themes are under development and will be added in the future.

An [overworld editor](https://github.com/blkerby/Z3OverworldEditor) was developed specifically for this randomizer, allowing artists to define custom palettes, tilesets, conditional edge layers, animations, cutscenes, and Mode 7 map artwork. The editor currently focuses on overworld tiling; it may later support customizing other game elements such as enemies, entrances, secrets, items, and dungeons. The retiling happens in the [ALTTPRetiling](https://github.com/kjbranch/ALTTPRetiling) repository. 

In order to support the flexible palette and tileset system, substantial changes to the game engine were needed. For some technical details about how this works, see the [architecture](docs/architecture.md) and [engine](docs/engine/) docs.

A [design doc](https://docs.google.com/document/d/1skZsIxZLKbCC8C_-3b3TTZVwblzNkN2ZTXraZ4Zxzkc) describes some plans for tier-based item placement logic along with balance-oriented gameplay changes. Many of these are not yet implemented in the randomizer but may become options in the future. Other options may be added based on player feedback.

# Development

If you notice any issue while testing or if you run into any trouble following these instructions, please reach out in the [Discord](https://discord.gg/Mxb5zYZeVj) to let us know.

## How to use the CLI

Install Rust with [rustup](https://www.rust-lang.org/tools/install), along with CMake and a C++ compiler, which are used to build a patched version of the Asar assembler (e.g. `sudo apt install build-essential cmake` on Ubuntu). Clone the repository with its submodules, and place the exported map pool at `maps/map_pool.bin`:

```sh
git clone --recurse-submodules https://github.com/blkerby/z3-map-rando.git
```

From the repository root, generate a seed with the default settings and patch it:

```sh
cargo run --release -p cli --features bundle-catalogs -- --rom <path-to-vanilla-rom>
```

By using the `bundle-catalogs` feature, the map pool and various catalogs (logic data, retiling data, assembled IPS patches, etc.) are automatically built and embedded into the binary, `target/release/z3-map-rando`, so it can be run as a self-contained executable. If you install that binary into your PATH, you can then generate and patch a seed by simply running

```sh
z3-map-rando --rom <path-to-vanilla-rom>
```

The ROM must be the unheadered 1 MiB Japanese 1.0 ROM, with SHA-256 digest `794e040b02c7591b59ad8843b51e7c619b88f87cddc6083a8e7a4027b96a2271`. This writes the seed `z3-map-rando-<SEED-NAME>.json` and the ROM `z3-map-rando-<SEED-NAME>.sfc` to the current directory. Use `--output-dir` to write to a different directory. Omit `--rom` to generate only the seed JSON, or use `--seed seed.json --rom <path-to-vanilla-rom>` to patch a ROM using an existing generated seed. Use `--preset <ID>` or `--settings settings.json` to choose the settings (see [the settings format](docs/data-formats/settings.md)), and `--help` for all options.

## How to run the website locally

In addition to the CLI setup above, install Node.js, the WebAssembly target, `wasm-pack`, and the frontend's packages:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
npm install
```

From the repository root, run `npm run dev` and use a browser to open the "Local" URL that it prints (e.g., `http://localhost:5173/`). This builds the catalogs as needed, and rebuilds and reloads as files change. Arguments after `--` go to the randomizer server; for example, `npm run dev -- --store file:seeds` keeps generated seeds in `seeds/` instead of in memory. See [the frontend README](frontend/README.md) and [the backend setup](docs/backend.md) for details, including serving a production build.

## Other tools

- `render_world_maps` renders Light World Mode 7 map PNGs for the vanilla and random layouts into `build/world-maps/` by default. Run `cargo run -p render_world_maps -- --rom path/to/vanilla.sfc --count 10`, and add `--both-worlds` to include the Dark World. This can be used, for example, to test the appearance of new themes.
- `build_tile_fingerprints` regenerates `data/tile_fingerprints.json`, which lets the catalog builder recognize vanilla graphics without a ROM: `cargo run -p build_tile_fingerprints -- path/to/vanilla.sfc data/tile_fingerprints.json`. This is done in order to avoid vanilla assets being copied into the retiling catalog. In practice it's not needed to ever run this again, but it is kept in the project in order to show where this data comes from.

## Catalog configuration

The catalog builder writes the catalogs to `build/`, configured by `catalog-build.toml`, which it copies from `catalog-build.default.toml` on first use. Edit it to change source paths, the output directory, or the Zstd `compression_level` (e.g., use `18` for smaller release artifacts). To use an existing patched Asar executable instead of building the `asar` submodule, set `asar_executable`; an upstream Asar will not work, since the build relies on its IPS output support. See [the catalog plan](docs/architecture.md#retiling-catalog) and [the patches doc](docs/engine/patches.md) for how the catalogs are built and cached.
