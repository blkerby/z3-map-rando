# Project architecture

See [the prototype implementation plan](../plan.md) for the sequence of work.

The project has the following main components:

- [**ASM patches**](#asm-patches): modify the game engine and produce IPS patches and a symbol manifest.
- [**Catalog builders**](#catalog-builders): compile retiling, logic, and patching data into compact binary catalogs.
- [**Rearranger**](#rearranger): rearranges the overworld areas in a geometrically coherent way.
- [**Generator**](#generator): places items to create a beatable randomized game ("seed") based on a rearrangement.
- [**Patcher**](#patcher): combines a verified vanilla ROM, IPS patches, and seed data into a randomized ROM.
- [**CLI**](#cli): exposes generation and patching as native local commands.
- [**Web backend**](#web-backend): exposes generation, patching, and seed storage behind an HTTP JSON API.
- [**Web frontend**](#web-frontend): requests seeds and patches the player's ROM locally through WebAssembly.

## ASM patches

The ASM patches contain seed-independent code and hooks needed to support the randomizer, including the retiled overworld. They are assembled offline from patch sources without reading a ROM. Assembly produces IPS patches; a machine-readable symbol manifest for writing seed-specific data is also planned but not implemented. Shared ASM symbols are defined in `patches/src/symbols.inc`, and Rust currently defines patching addresses separately. Some patches are optional; patcher code decides their selection, application order, and phases.

The IPS patches may be included in a release or otherwise stored so ordinary users do not need an assembler. The planned work on the game engine is described in [engine.md](engine.md), which is currently a primary focus.

## Catalog builders

The catalog builders run offline and produce these artifacts under `build/`:

1. **Retiling builder:** This consumes the [ALTTPRetiling](https://github.com/kjbranch/ALTTPRetiling) JSON data which contains tile graphics and area layouts, including rethemed areas and edge variants of areas. It emits a binary file (the "retiling catalog") collecting this data in a compact, internal format.
2. **Logic builder:** This consumes `z3-json-data` and emits a compact binary file (the "logic catalog"). See [the logic catalog plan](logic.md).
3. **Patch builder:** This bundles named IPS patches and their matching symbol manifest, plus stable location mappings, ROM addresses, item receipt IDs, and prize encodings extracted directly from `z3-json-data`. It does not read the logic catalog. Individual IPS patches remain separate; patcher code controls their application. See [the patch plan](patches.md).

The output paths are `build/retiling_catalog.bin`, `build/logic_catalog.bin`, and `build/patch_catalog.bin`. This directory is ignored by Git; checked-in inputs remain under `data/`. Catalogs survive `cargo clean`, which removes Cargo artifacts under `target/`. The patch catalog builder is planned work.

### Catalog format

The catalogs use a Zstd-compressed `bincode-next` payload following a 16-byte uncompressed envelope. The envelope includes:

- Magic bytes to identify the format.
- A 64-bit schema identifier for the payload's root Rust type.

The retiling catalog currently uses `type_hash`. The logic catalog uses `serde-reflection` to describe its complete schema, including recursive types and enum names, tags, and payloads. Its identifier is the first eight SHA-256 bytes of that registry encoded with the standard bincode configuration, interpreted as a little-endian `u64`.

The catalog APIs stream encoding through a buffered Zstd encoder and decoding through a buffered Zstd decoder, without buffering the entire uncompressed payload. The builder accepts `--compression-level`, defaulting to `3` independently of Cargo profile; future CI release builds can explicitly use `18`. Changing compression does not change the decoded data or schema identifier. Prototype format changes require rebuilding catalogs, without legacy readers.

A reader checks the envelope before decoding the payload. Schema changes that alter the identifier create a new format revision. Backward compatibility is not required; building and consuming a catalog use the same project version. This applies to catalogs, not saved seeds. The latest patcher and its matching patch catalog must support older seeds without their original catalogs.

### Retiling catalog

The retiling catalog includes all the data from `ALTTPRetiling` needed by the randomizer and patcher. It includes custom palettes, tilesets, 8x8 graphics, and area screen data for themes and edge variants.

Implemented: `catalog_builder retiling` emits the catalog without a ROM input. `theme_check` consumes that catalog through the shared reader, compiler, and asset writer. See the [build instructions](../README.md#how-to-build-the-retiling-catalog).

The Rust types are defined in [`retiling_catalog`](../crates/retiling_catalog/src/lib.rs). `RetilingCatalog` contains palettes keyed by their authored IDs, areas keyed by their source names, and dynamic tile replacement groups. Graphics remain inline in palette-local tile definitions and animation frames. Each area contains its named themes, which retain background settings, cutscenes, and ordered named BG1/BG2 layers. The builder combines each layer's editor screens into one sparse list of 8x8 tile placements with `u8` X/Y coordinates, retaining the full grid dimensions and omitting empty positions. Compositing and event interpretation remain in shared asset compilation. Areas without vanilla map IDs retain their template content. Structured edge-connection metadata is deferred until its source format is defined.

The retiling builder requires no ROM input. It recognizes vanilla graphics, including recolored tiles, through a checked-in JSON fingerprint index. A separate developer tool generates that index from a verified vanilla ROM. Each entry contains a SHA-256 hash of a tile's canonical form and its stable graphics sheet and tile offset; the index contains no tile pixels or palette colors. Include every tile in background sheets `$00–$60`, regardless of retiling project usage. Sheets `$61–$70` contain only blank tiles and are omitted. Exclude sprite sheets, the UI aliases `$71/$72`, and standalone graphics. The tool and index format are documented in the [build instructions](../README.md#how-to-build-the-tile-fingerprint-index).

Canonicalize an 8x8 tile as follows:

1. Consider the unflipped, horizontally flipped, vertically flipped, and both flipped orientations, in that order.
2. For each orientation, scan pixels from left to right, top to bottom. Assign consecutive canonical color indexes starting at zero as each distinct source color index first appears. Record the mapping from canonical indexes to source indexes for that orientation.
3. Choose the lexicographically smallest normalized 64-index array. Break ties using the orientation order above. Its mapping and flip flags belong to the chosen canonical form.

For every authored static tile and animation frame, the builder hashes the 64 canonical color-index bytes with SHA-256 and looks up the full 256-bit digest in the fingerprint index. A match becomes a vanilla reference containing the graphics sheet and tile offset, the mapping from canonical indexes to authored palette indexes `$0-$F`, and the flips from the canonical orientation to the authored orientation. Choose the lowest graphics sheet, then tile offset, when several vanilla tiles match. Unmatched tiles retain custom pixel data.

The patcher decodes the referenced tile from the player's verified ROM, canonicalizes it, applies the stored color-index mapping, and flips it back to the authored orientation. The index generator, builder, and patcher share the canonicalization routine. Reconstruction preserves every authored color index, even when distinct indexes have the same RGB color. Canonical index zero means the first encountered color; transparency depends on the final mapped palette index zero. Matching initially requires the same partition of pixels into distinct color indexes, so recolorings that merge vanilla colors remain custom graphics.

The catalog stores normalized authored content and vanilla references. Shared asset compilation code consumes catalog records to select variants, allocate palette and character slots, and generate Map16 and runtime assets. Vanilla references, rain graphics, sprites, and fallback maps and scenes are resolved from the ROM during patching. `theme_check` loads the catalog and uses shared compilation in `patcher::retiling` and asset writing in `patcher::asset_bundle`.

### Logic catalog

The [logic catalog](logic.md) compiles room nodes, strats, and item locations from `z3-json-data`, with separate Light/Dark World vertices. Recursive requirements retain ordered resource use and alternative local states. Shared types are defined in [`logic_catalog`](../crates/logic_catalog/src/lib.rs); [`catalog_builder::logic`](../crates/catalog_builder/src/logic/mod.rs) builds the binary catalog from the source definitions. The catalog stores vanilla entrance, teleport, whirlpool, and adjacent overworld pairings separately from room edges. Generation expands these pairings or their randomized replacements into graph connections. The planned boundary keeps stable location and item identities in this catalog, while ROM addresses and item patch encodings go only into the patch catalog. Both builders read `z3-json-data` directly and may share source-reading types. [Door-specific keys](keys.md) are persistent progression items, so key logic does not require alternative spending histories.

## Rearranger

A central feature of the randomizer is that it rearranges and rethemes the areas of the overworld. The overworld remains an 8x8 grid of 512x512 pixel units, and each area retains its interior features including enemies, entrances, items, and secrets. The core randomized elements are

- area placement: where an area is placed on the grid
- area theme: the tile theme used for the area, such as Desert, Swamp, or Forest
- area edge variants: modifications to allow areas to flexibly connect to their neighbors

Each Light World area and its corresponding Dark World area form one randomization unit: they move to corresponding slots together and use the same selected tile theme.  Mirror/portal correspondence therefore follows this same paired placement. An area's neighbor-edge connections also typically align between Light World and Dark World, though a small amount of exceptions are permitted, as in the vanilla game.

Edge variants are designed to be normalized to specific sizes and positions so that they can connect modularly to neighboring rooms. Not every edge variant works in every position of every room, because of constraints dependent on the room's shape and features. In order for the rearrangement process to be effective, there must be a large enough pool of edge variants available so that rearrangements are not too heavily constrained. This requires development work on the overworld editor, to support representing edge variants in a structured way; it also requires a significant amount of retiling work to create variants that integrate seamlessly with the area interior. The exact structure of how edge variants will be encoded still needs to be determined. Therefore, automating the overworld rearrangement is not a priority at this stage of the project. Nevertheless, we can describe the current plan:

Assuming that the edge variants are constrained enough that a brute-force approach to rearrangement is not viable, the plan is to use a small reinforcement-learning model. Areas can be placed one at a time, starting in the top-left corner and proceeding row by row, skipping any cells already filled by a large area on the previous row. The model can consider candidates for each placement and predict final outcomes, such as the number of successfully placed areas and number of connected components, conditioned on each placement. These predictions can be aggregated to form reward scores, with higher-scoring candidates being assigned a higher probability of being selected. In addition to the essential scoring terms relating to the validity of the rearrangement, additional terms can be added to shape desirable characteristics, such as rewarding variety in placements (i.e., disincentivizing commonly selected placements or pairings of areas).

During the area placement process, two adjacent areas can be assumed to be connected as long as there is any valid combination of traversible, compatible edge variants between them. Assignment of themes and concrete edge variants can be selected as a post-processing step, after a rearrangement's validity has already been determined: at this stage, some traversible edges may be replaced by non-traversible ones (e.g. rock walls), as long as global connectivity is preserved. Global connectivity may also take into account cave networks and the possibility of using whirlpools, flute transport, and portals/mirror.

The rearranger will likely be written as a Python application with a Rust subcomponent (e.g. using PyO3 bindings managed with `maturin`). Python is appealing for easiest access to machine-learning functionality, while Rust is convenient for environment simulation and feature extraction. The rearranger will use the retiling catalog in order to determine if a given rearrangement (including assigned themes and edges) will satisfy hardware limits on palette colors and tilesets, and this would also fit on the Rust side.

## Generator

The generator is a Rust library for creating a randomized game ("seed"). It invokes the rearranger to obtain a rearranged overworld, then places items in a way that provides logical progression, so that the game is beatable at the selected level of difficulty. If it fails, it can retry with a fresh rearrangement.

[Door-specific keys](keys.md) can share a progression-placement process with other items, subject to dungeon placement restrictions. A separate key-placement phase is optional.

Successful seed generation produces the area placement coordinates, selected themes and edge variants, entrance connections, item placements, and seed-specific retiling content. Metadata records the seed name, generation timestamp, generator version, source commit when available, numeric RNG seed, and catalog hashes for provenance. Catalog hashes do not select patching assets.

### Saved seed format

Save the complete seed as JSON compressed with Zstd, including `SeedRetilingData`. Settings files remain ordinary JSON. Serialization can write directly through a Zstd encoder, and `serde_json::from_reader` can deserialize from a buffered Zstd decoder without holding the entire decompressed JSON or an intermediate JSON value in memory. The decoded seed itself remains in memory.

The seed has an integer `format_version`, starting at `1`, separate from the generator version. Increment it for meaningful interpretation changes: changes to field meanings or units, representation changes requiring conversion, or new essential content that cannot be reconstructed from older seed data. Also increment it when an older reader ignoring a new field would interpret the generated game incorrectly, even if the latest reader can default that field for older seeds.

Do not increment it for compatible extensions such as informational metadata, optional fields, or fields whose omission has a well-defined default preserving old behavior. A rename handled by a deserialization alias needs no bump. Defaults must express what an older seed meant, rather than merely provide convenient values. For example, adding generation-duration metadata needs no bump; changing coordinates from pixels to tiles does.

Document the meaning of each format version. The latest patcher retains support for earlier versions through defaults or migrations that preserve their meaning.

Saved data describes the generated game, independently of any catalog version. Item placements identify locations by the authored `(room_id, item_id)` pair from `z3-json-data`, not a logic-catalog index or ROM address. These IDs are permanent: do not renumber existing locations or reuse removed IDs. `item_id` is the ID of the room's item entry, not its `itemLocation` logic node. Item identities also remain stable across releases. The current patch catalog maps location identities to patching instructions.

ROM addresses, symbol addresses, and runtime asset allocation decisions belong to the current patcher and patch catalog, rather than the saved seed. A seed does not require a particular patcher version or the catalogs used to generate it.

Conceptually, rearrangement can be considered part of the generation process, and it is possible that they are combined in a single binary. However, because of the likely language boundary (with rearrangement likely happening primarily in Python), it may be more convenient for them to be separate services. On the other hand, there is also a possibility of building an offline pool of rearrangements, which would eliminate the need for rearrangement as a generation-time service.

## ROM patching

### Seed-specific retiling data

During generation, reduce the retiling catalog to a `SeedRetilingData` object containing the selected layouts, palettes, tiles, and other required content. Save this content in the seed rather than references to entries in the original catalog. It retains sanitized vanilla-graphics references with stable meanings; the patcher resolves these from the player's ROM. Runtime palette and character slots and ROM storage are assigned during patching.

For the web service, this data is prepared on the server and included in the Zstd-compressed seed JSON. The client needs neither the full retiling catalog nor the logic catalog. `SeedRetilingData` uses the saved seed format, not the catalogs' bincode envelope.

### Patcher

The patcher is a Rust library for transforming a user-provided vanilla ROM into a randomized ROM. Its core is a pure function receiving the ROM, decoded seed, current patch catalog, and patch-time customization settings, and returning an output ROM. It has no filesystem, network, clock, or RNG dependency, and does not consume the logic or retiling catalogs.

The patcher validates the user ROM against a static checksum and copies it to an output buffer. It applies patches in their required phases, resolves vanilla graphics references, compiles the saved content into runtime assets, and writes item placements and other seed data using current location mappings and symbols. Finally, it updates the checksum and complement.

Use the latest patcher and its matching patch catalog even for old seeds, so patching bug fixes and new customization options remain available. Compatibility is maintained by the current seed readers and patching implementation, rather than by selecting archived patchers or patch catalogs based on seed metadata.

The native CLI calls the core function directly. A WebAssembly build exposes the same API to the TypeScript frontend, serializing the inputs to byte buffers. The behavior is identical in the native and WebAssembly builds.

## User interfaces

### CLI

The CLI is a native Rust binary that invokes the generator, patcher, or both according to its arguments. Distributed binaries bundle default settings and catalogs. See [the CLI plan](cli.md) for arguments, output naming, and build behavior.

### Web backend

The web backend is a Rust service built on the Rust library. It loads the logic and retiling catalogs, connects to an object store for seed storage, and exposes an HTTP JSON API for seed generation and other services needed by the frontend.

### Web frontend

The web frontend is a TypeScript application that requests generation and fetches stored Zstd-compressed seeds. It obtains the latest WebAssembly patcher and its matching patch catalog, including when loading an old seed. It selects the player's ROM through a local file input, invokes the WebAssembly patching API, and offers the returned ROM bytes as a download. The ROM and output exist only on the player's system.

## Crate boundaries

Keeping the generator and patcher in separate crates cleanly manages dependencies because the WebAssembly package depends only on the patcher; for example, this prevents it from pulling in generator-specific dependencies that are unavailable or unneeded on that platform. The generator and patcher Rust libraries are primarily geared toward internal use: they are not published to crates.io, and their APIs are unstable, but they are nevertheless designed with an understanding that other projects might want to reuse them, such as for multiworld integration or plandos. Therefore, we make an effort to present a relatively clean, flexible API that could serve most such needs.
