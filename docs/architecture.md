# Project architecture

The project has the following main components:

- [**ASM patches**](#asm-patches): modify the game engine and produce IPS patches and a symbol manifest.
- [**Catalog builders**](#catalog-builders): compile retiling, logic, patching, and preset data into compact binary catalogs.
- [**Rearranger**](#rearranger): rearranges the overworld areas in a geometrically coherent way, offline, into a pool of layouts.
- [**Generator**](#generator): places items to create a beatable randomized game ("seed") based on a rearrangement from the pool.
- [**Patcher**](#patcher): combines a verified vanilla ROM, IPS patches, and seed data into a randomized ROM.
- [**CLI**](#cli): exposes generation and patching as native local commands.
- [**Web backend**](#web-backend): exposes generation, patching, and seed storage behind an HTTP JSON API.
- [**Web frontend**](#web-frontend): requests seeds and patches the player's ROM locally through WebAssembly.

## ASM patches

The ASM patches contain seed-independent code and hooks needed to support the randomizer, including the retiled overworld. They are assembled offline from patch sources without reading a ROM. Assembly produces IPS patches and a typed symbol manifest for writing seed-specific data. Shared ASM symbols are defined in `patches/src/symbols.inc`; `%export_symbol` marks those consumed by Rust, and `symbols.asm` exports them through Asar's symbol output. The builder checks exact correspondence with the fields of `PatchSymbols`. Rust patching code reads the exported runtime table and storage addresses from the current patch catalog. Some patches are optional; patcher code decides their selection, application order, and phases.

Release binaries bundle the patch catalog, including IPS payloads and symbols, so ordinary users do not need an assembler. The engine changes are summarized in [engine modifications](engine/README.md).

## Catalog builders

The catalog builders run offline and produce these artifacts under `build/`:

1. **Retiling builder:** This consumes the [ALTTPRetiling](https://github.com/kjbranch/ALTTPRetiling) JSON data which contains tile graphics and area layouts, including rethemed areas and edge variants of areas. It emits a binary file (the "retiling catalog") collecting this data in a compact, internal format.
2. **Logic builder:** This consumes `z3-json-data` and emits a compact binary file (the "logic catalog"). See [the logic catalog](generation/logic.md).
3. **Patch builder:** This bundles named IPS patches and their matching symbol manifest, plus stable location mappings, ROM addresses, item receipt IDs, and prize encodings extracted directly from `z3-json-data`. It does not read the logic catalog. Individual IPS patches remain separate; patcher code controls their application. See [the patches doc](engine/patches.md).
4. **Preset builder:** This prepares the built-in generation presets from the preset files and difficulty classifications under `data/`, deriving the skill presets and resolving each root preset into complete settings (see [shared settings data](data-formats/settings-data.md)). Types are defined in [`preset_catalog`](../crates/preset_catalog/src/lib.rs).

The output paths are `build/retiling_catalog.bin`, `build/logic_catalog.bin`, `build/patch_catalog.bin`, and `build/preset_catalog.bin`. This directory is ignored by Git; checked-in inputs remain under `data/`. Catalogs survive `cargo clean`, which removes Cargo artifacts under `target/`. `catalog_builder all` builds all four catalogs; `patches` builds the patch catalog alone, reusing cached IPS and symbol artifacts.

### Catalog format

The catalogs use a Zstd-compressed `bincode-next` payload following eight uncompressed magic bytes identifying the catalog kind. There is no runtime schema fingerprint. The builder uses separate SHA-256 cache fingerprints of source code, configuration, and data inputs to invalidate stale catalogs.

The catalog APIs stream encoding through a buffered Zstd encoder and decoding through a buffered Zstd decoder, without buffering the entire uncompressed payload. The builder reads `compression_level` from `catalog-build.toml`, with `3` in the default template independently of Cargo profile; future CI release builds can explicitly use `18`. Changing compression does not change the decoded data. Prototype format changes require rebuilding catalogs, without legacy readers.

Building and consuming a catalog use the same project version. Browser catalogs must ship alongside their matching WASM build using content-hashed asset URLs. Manually supplied incompatible catalogs have no dedicated schema-mismatch check. Backward compatibility is not required for catalogs; this does not apply to saved seeds. The current retiling preparation code, patcher, and matching patch catalog must support older seeds without the original catalog versions.

### Retiling catalog

The retiling catalog includes all the data from `ALTTPRetiling` needed by the randomizer and patcher. It includes custom palettes, tilesets, 8x8 graphics, and area screen data for themes and edge variants.

Implemented: `catalog_builder retiling` emits the catalog without a ROM input. The patcher consumes that catalog through the shared reader, compiler, and asset writer. See the [catalog configuration](../README.md#catalog-configuration).

The Rust types are defined in [`retiling_catalog`](../crates/retiling_catalog/src/lib.rs). `RetilingCatalog` contains palettes keyed by their authored IDs, areas keyed by their source names, and dynamic tile replacement groups. Graphics remain inline in palette-local tile definitions and animation frames. Each area contains its named themes, which retain background settings, cutscenes, and ordered named BG1/BG2 layers. The builder combines each layer's editor screens into one sparse list of 8x8 tile placements with `u8` X/Y coordinates, retaining the full grid dimensions and omitting empty positions. Compositing and event interpretation remain in shared asset compilation. Areas without vanilla map IDs retain their template content. Structured edge-connection metadata is deferred until its source format is defined.

The retiling builder requires no ROM input. It recognizes vanilla graphics, including recolored tiles, through a checked-in JSON fingerprint index. A separate developer tool generates that index from a verified vanilla ROM. Each entry contains a SHA-256 hash of a tile's canonical form and its stable graphics sheet and tile offset; the index contains no tile pixels or palette colors. Include every tile in background sheets `$00–$60`, regardless of retiling project usage. Sheets `$61–$70` contain only blank tiles and are omitted. Exclude sprite sheets, the UI aliases `$71/$72`, and standalone graphics. The tool and index format are documented in the [main README](../README.md#other-tools).

Canonicalize an 8x8 tile as follows:

1. Consider the unflipped, horizontally flipped, vertically flipped, and both flipped orientations, in that order.
2. For each orientation, scan pixels from left to right, top to bottom. Assign consecutive canonical color indexes starting at zero as each distinct source color index first appears. Record the mapping from canonical indexes to source indexes for that orientation.
3. Choose the lexicographically smallest normalized 64-index array. Break ties using the orientation order above. Its mapping and flip flags belong to the chosen canonical form.

For every authored static tile and animation frame, the builder hashes the 64 canonical color-index bytes with SHA-256 and looks up the full 256-bit digest in the fingerprint index. A match becomes a vanilla reference containing the graphics sheet and tile offset, the mapping from canonical indexes to authored palette indexes `$0-$F`, and the flips from the canonical orientation to the authored orientation. Choose the lowest graphics sheet, then tile offset, when several vanilla tiles match. Unmatched tiles retain custom pixel data.

The patcher decodes the referenced tile from the player's verified ROM, canonicalizes it, applies the stored color-index mapping, and flips it back to the authored orientation. The index generator, builder, and patcher share the canonicalization routine. Reconstruction preserves every authored color index, even when distinct indexes have the same RGB color. Canonical index zero means the first encountered color; transparency depends on the final mapped palette index zero. Matching initially requires the same partition of pixels into distinct color indexes, so recolorings that merge vanilla colors remain custom graphics.

The catalog stores normalized authored content and vanilla references. Shared asset compilation code consumes catalog records to select variants, allocate palette and character slots, and generate Map16 and runtime assets. Vanilla references, rain graphics, sprites, and fallback maps and scenes are resolved from the ROM during patching. `patcher::patch_rom` uses compilation in `patcher::retiling` and asset writing in `patcher::asset_bundle`.

### Logic catalog

The [logic catalog](generation/logic.md) compiles room nodes, strats, and item locations from `z3-json-data`, with separate Light/Dark World vertices. Recursive requirements retain ordered resource use and alternative local states. Shared types are defined in [`logic_catalog`](../crates/logic_catalog/src/lib.rs); [`catalog_builder::logic`](../crates/catalog_builder/src/logic/mod.rs) builds the binary catalog from the source definitions. The catalog stores vanilla entrance, teleport, whirlpool, and adjacent overworld pairings separately from room edges. Generation expands these pairings or their randomized replacements into graph connections. This catalog retains stable location and item identities, while ROM addresses and item patch encodings are stored only in the patch catalog. Both builders read `z3-json-data` directly and may share source-reading types. [Door-specific keys](generation/keys.md) are persistent progression items, so key logic does not require alternative spending histories.

## Rearranger

A central feature of the randomizer is that it rearranges and rethemes the areas of the overworld. The overworld remains an 8x8 grid of 512x512 pixel units, and each area retains its interior features including enemies, entrances, items, and secrets. The core randomized elements are

- area placement: where an area is placed on the grid
- area theme: the tile theme used for the area, such as Desert, Swamp, or Forest
- area edge variants: modifications to allow areas to flexibly connect to their neighbors

Each Light World area and its corresponding Dark World area form one randomization unit: they move to corresponding slots together and use the same selected tile theme.  Mirror/portal correspondence therefore follows this same paired placement. An area's neighbor-edge connections also typically align between Light World and Dark World, though a small amount of exceptions are permitted, as in the vanilla game.

Edge variants are normalized to specific sizes and positions so that they can connect modularly to neighboring rooms. Not every edge variant works in every position of every room, because of constraints dependent on the room's shape and features, so rearrangement needs a large enough pool of edge variants not to be too heavily constrained.

The rearranger is a separate project, [z3-map-gen](https://github.com/blkerby/z3-map-gen), which uses a small reinforcement-learning model. It places areas one at a time, row by row from the top-left corner, skipping cells already filled by a large area, and its model predicts the final outcome of each candidate placement, such as the number of successfully placed areas and connected components. Two adjacent areas count as connected when any valid combination of traversable, compatible edge variants exists between them; concrete edge variants and themes are chosen later, during generation.

The rearranger runs offline to build a pool of layouts, so the CLI and server need no Python or machine-learning dependencies. A resampling step balances the pool, both in the geometric placement of individual rooms across their possible positions and in the possible edge connections, which would be hard to match with a live rearranger. The pool's format is documented in z3-map-gen's `docs/map_pool.md`; the randomizer reads it from `maps/map_pool.bin` (see [map randomization](map_randomization.md)).

## Generator

The generator is a Rust library for creating a randomized game ("seed"). For a random layout it selects a rearrangement from the map pool and chooses edge variants, then places items in a way that provides logical progression, so that the game is beatable at the selected level of difficulty. If it fails, it can retry with another layout.

[Door-specific keys](generation/keys.md) are assigned in a preliminary pass, one dungeon at a time, before ordinary item placement. Every new complete generation attempt reruns this pass.

[`generator::generate`](../crates/generator/src/lib.rs) fills logically collectible slots in waves using player settings and the connections of the selected world layout (see [item placement](generation/placement.md)). Maps and Compasses take priority within their source dungeon; ordinary items are sampled from the complete remaining pool. Keys remain at vanilla locations, Big Keys stay fixed, and prizes shuffle among prize locations. The Waterfall and Pyramid fairy locations are filled first in each wave, from items they can upgrade (see [fairies.md](generation/fairies.md)). Each candidate evaluates normal collection closure before committing. The generator allows 100 candidates per wave and 1000 complete attempts, restarting with fresh keys after a failed attempt. Completion requires every slot to be filled and every existing collected-location bit to be set, without another solve. Callers supply catalogs, settings, the selected theme, metadata, and RNG state; the library returns `SeedData` without ROM or filesystem access.

Successful seed generation produces the area placement coordinates, selected themes and edge variants, entrance connections, and item placements. Persist area positions, themes, and edge-variant choices in `WorldLayout` within `SeedData`; prepare graphical content later at patching time. Metadata records the seed name, generation timestamp, shared project version, source commit when available, numeric RNG seed, and catalog hashes for provenance. Catalog hashes do not select patching assets.

### Project version

The root [`VERSION`](../VERSION) file is the single source for the CLI, web service, frontend, seed metadata, and release packaging. The catalog builder computes the project version on every run and writes it to `build/version.txt`, outside its catalog cache. The server reads that file at startup, the CLI reads it like the catalogs (bundling it in release builds), and the frontend imports it; the generator receives the version from its caller. Since no Rust code embeds it, a commit does not recompile the workspace, but a running server reports the version from the catalog builder's last run. Versions have two components: the first increments for regular releases, and the second increments for rare hotfixes addressing severe bugs. For example, `7.0` can be followed by `7.1` and `7.2`, then `8.0` for the next regular release. Cargo package versions are unused.

`VERSION` names the upcoming release: bump it right after tagging a release, and set it on a hotfix branch to the hotfix version. A build whose HEAD carries the tag `v<VERSION>` is that release and reports `<VERSION>`. Any other build reports a prerelease of it, `<VERSION>-dev.<commit UTC date>.<commit UTC time>+<short commit hash>`, such as `8.0-dev.20261119.193204+1c63b11`, which orders before `8.0`. Using the commit time rather than the time of the run makes the version reproducible from the source. Builds without git report `<VERSION>-dev`. Uncommitted changes are not marked.

### Saved seed format

The shared [`seed_data`](../crates/seed_data/src/lib.rs) crate defines settings and saved seed types without depending on catalogs or the generator. The [`common_types`](../crates/common_types/src/lib.rs) crate defines stable room/item-location identities shared by seed data and catalogs. Catalog builders do not depend on saved seed types; the preset builder uses the settings types to produce complete preset settings. Saved placements are grouped by the authored `room_id` and contain the `item_id` plus a typed `item` using the shared `common_types::Item` enum. A Small Key includes its stable assigned `DoorId` in that item, and a Map, Compass, or Big Key includes its dungeon's room ID. Logic graph indices stay internal to the logic catalog.

Save the complete seed as plain JSON, including `WorldLayout` and excluding graphical catalog content. The CLI writes and reads buffered JSON directly. Backend object storage transparently applies Zstd compression and a physical `.zst` suffix to every object; application code uses logical keys and uncompressed bytes. The decoded seed itself remains in memory.

The seed has no format version. Compatible extensions, such as informational metadata, optional fields, or fields whose omission has a well-defined default preserving old behavior, need no migration. Defaults must express what an older seed meant, rather than merely provide convenient values. A version field can be introduced if a change ever requires a structural migration that cannot recognize older seeds by their shape. Long-term reader compatibility remains future release work.

Saved data describes the generated game, independently of any catalog version. Item placements identify locations by the authored `(room_id, item_id)` pair from `z3-json-data`, not a logic-catalog index or ROM address. These IDs are permanent: do not renumber existing locations or reuse removed IDs. `item_id` is the ID of the room's item entry, not its `itemLocation` logic node. Item identities also remain stable across releases. The current patch catalog maps location identities to patching instructions.

Saved seeds record each overworld room's `theme` and grid `position`, whether the layout is vanilla, and, for a randomized layout, the open borders between rooms; vanilla connections are implicit. Special areas take the theme of their parent overworld room. The patch catalog maps rooms to their map areas, so area IDs are not saved. Keep these identities permanent and map retired or restructured choices to current equivalents that preserve connectivity and gameplay meaning. Fine-grained graphical content is resolved from the current retiling catalog at patching time, so it can evolve without changing the persisted layout.

ROM addresses, symbol addresses, and runtime asset allocation decisions belong to the current patcher and patch catalog, rather than the saved seed. A seed does not require a particular patcher version or the original versions of the catalogs used to generate it.


## ROM patching

### Retiling catalog at patching time

Reuse `RetilingCatalog` as patching input alongside the persisted `WorldLayout`. The patcher selects the layouts, palettes, tiles, and other content required by the saved choices. The native CLI passes its already-loaded current catalog unchanged, without a filtering or conversion step. There is no separate `SeedRetilingData` type, and graphical content is not persisted in `SeedData`.

For the web service, the backend filters the current catalog according to the saved layout at patching time and sends the smaller `RetilingCatalog` to the client alongside the saved seed and current patching assets. The same patcher interface accepts full and filtered catalogs. The client needs neither the full retiling catalog nor the logic catalog. Both full and filtered catalog formats evolve with the current code and have no backward-compatibility requirement; persisted layout choices retain their long-term compatibility contract.

Initially filter areas and themes while retaining all layers within selected themes. Further filtering can remove unused layers, palettes, tiles, and dynamic-tile variants when their dependencies are accounted for, including alternate backgrounds, event overlays, cutscenes, and animation frames. Retain referenced content and preserve identities and tile indices so filtering does not change interpretation. Sanitized vanilla graphics references remain in the catalog; the patcher resolves them from the player's ROM. Runtime palette and character slots and ROM storage are assigned during patching.

### Patcher

The patcher is a Rust library for transforming a user-provided vanilla ROM into a randomized ROM. Its core is a pure function receiving the ROM, decoded seed, current full or filtered `RetilingCatalog`, current patch catalog, and patch-time customization settings, and returning an output ROM. It has no filesystem, network, clock, or RNG dependency, and does not consume the logic catalog.

Implemented: `patcher::patch_rom` validates the user ROM against a static checksum and copies it to an output buffer. It applies in-memory IPS patches in their required phases, resolves vanilla graphics references, compiles the area themes selected by `WorldLayout`, and writes runtime assets using current symbols. It returns a 4 MiB ROM with its checksum and complement updated. Serializable `PatchSettings` exposes `transition_asset_phase`, defaulting to `Scroll`. It applies saved item placements through current location mappings and item/prize encodings, with carrier hooks preserving collection state independently of the item. Keys remain vanilla. Pickup graphics show assigned items using existing sprite slots and artwork; equipment previews use common palettes independently of owned equipment, and receipt behavior remains vanilla. Progressive items resolve to the next tier when previewed and collected, and Ocarina to the inactive pickup. Door-specific keys remain deferred. HalfMagic uses its custom 16x16 icon for both pickups and receipts, with the common green item palette.

Use the latest patcher, its matching patch catalog, and the current full or filtered retiling catalog even for old seeds, so patching bug fixes and new customization options remain available. Compatibility is maintained by current seed readers, layout-choice mappings, and patching code, rather than by selecting archived catalogs or patchers based on seed metadata.

The native CLI calls the core function directly. A WebAssembly build exposes the same API to the TypeScript frontend, serializing the inputs to byte buffers. The behavior is identical in the native and WebAssembly builds.

## User interfaces

### CLI

The CLI is a native Rust binary that invokes the generator, patcher, or both according to its arguments. Distributed binaries bundle default settings and catalogs. See [the CLI doc](cli.md) for arguments, output naming, and build behavior.

### Web backend

The web backend is a Rust service built on the Rust library. It loads the logic and retiling catalogs, connects to an object store for seed storage, and exposes an HTTP JSON API for seed generation and other services needed by the frontend. At patching time it filters the current `RetilingCatalog` using the saved world layout and sends that catalog to the client.

### Web frontend

The web frontend is a TypeScript application that requests generation and fetches stored seeds. At patching time it obtains a filtered current `RetilingCatalog` from the backend, the latest WebAssembly patcher, and its matching patch catalog, including when loading an old seed. It selects the player's ROM through a local file input, invokes the WebAssembly patching API, and offers the returned ROM bytes as a download. The ROM and output exist only on the player's system.

## Crate boundaries

This project intentionally uses many small internal crates. It helps separate dependencies that serve different parts of the project. For example, any change to the dependencies of the catalog builders results in a cache miss and catalog rebuild; therefore we want to keep those dependencies to a minimum. The WebAssembly patcher is downloaded by the browser, so separating it from generation and catalog-building code lets the client include only the code needed for patching.
