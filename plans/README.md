# Project architecture

The project has the following main components:

- [**ASM patches**](#asm-patches): modify the game engine and produce IPS patches and a symbol manifest.
- [**Catalog builders**](#catalog-builders): compile retiling and logic source data into compact binary catalogs.
- [**Rearranger**](#rearranger): rearranges the overworld areas in a geometrically coherent way.
- [**Generator**](#generator): places items to create a beatable randomized game ("seed") based on a rearrangement.
- [**Patcher**](#patcher): combines a verified vanilla ROM, IPS patches, and seed data into a randomized ROM.
- [**CLI**](#cli): exposes generation and patching as native local commands.
- [**Web backend**](#web-backend): exposes generation, patching, and seed storage behind an HTTP JSON API.
- [**Web frontend**](#web-frontend): requests seeds and patches the player's ROM locally through WebAssembly.

## ASM patches

The ASM patches contain seed-independent code and hooks needed to support the randomizer, including modifications to the game engine for the retiled overworld. They are assembled offline directly from patch sources, without reading a ROM, into IPS patches and a machine-readable JSON manifest containing symbol addresses needed to write seed-specific data later. Some patches are optional, and may be applied or not, depending on settings. 

The IPS patches may be included in a release or otherwise stored so ordinary users do not need an assembler. The planned work on the game engine is described in [engine.md](engine.md), which is currently a primary focus.

## Catalog builders

The catalog builders are small Rust executables which run offline; they bundle and preprocess the relevant content of two source projects:

1. **Retiling builder:** This consumes the [ALTTPRetiling](https://github.com/kjbranch/ALTTPRetiling) JSON data which contains tile graphics and area layouts, including rethemed areas and edge variants of areas. It emits a binary file (the "retiling catalog") collecting this data in a compact, internal format.
2. **Logic builder:** This consumes the future logic project's JSON and emits a compact binary file (the "logic catalog").

### Catalog format

The retiling and logic catalogs are encoded in a compact, internal binary format using `bincode-next`, using `type_hash` to identify the format revision. Each file has a small stable envelope followed by a `bincode-next` payload. The envelope includes:

- Magic bytes to identify the format.
- The `type_hash` value for the payload's root Rust type (`RetilingCatalog` or `LogicCatalog`).

A reader checks the envelope first and rejects a mismatch before attempting `bincode-next` deserialization. Any Rust type change that alters the root type hash creates a new binary format revision automatically. Backward compatibility is not required, as it is intended that the same version of the project be used to both build the catalog and consume it.

### Retiling catalog

The retiling catalog includes all the data from `ALTTPRetiling` needed by the randomizer and patcher. It includes custom palettes, tilesets, 8x8 graphics, and area screen data for themes and edge variants.

The retiling-data build uses a verified vanilla ROM as a private reference to avoid copying vanilla graphics into the distributable catalog file. For every retiled 8x8 graphic definition:

1. It resolves its 4bpp pixel indices through its retiled palette to canonical SNES RGB color values.
2. It decodes candidate vanilla 3bpp 8x8 tiles from the reference ROM and compares their resolved RGB pixels under all four horizontal/vertical flip combinations. Matching is based on the final RGB values, not equality of palette indexes or bitplane bytes.
3. When a match exists, it emits a vanilla-reference record instead of custom pixel data. The record contains the stable vanilla tile identity, a mapping from each vanilla color index `$0-$7` to a retiled color index `$0-$F`, and horizontal- and vertical-flip booleans describing how to reproduce the retiled orientation.
4. When multiple vanilla candidates reproduce the same resolved tile, it selects one by a documented deterministic ordering so rebuilding produces identical bytes.

### Logic catalog

The logic catalog includes all the necessary data from a separate logic project to allow the randomizer to generate beatable seeds of appropriate difficulty based on the user's selected settings. The source logic project contains JSON defining nodes and strats within overworld areas and dungeons. Nodes may represent entrances/exits, traversal regions, item locations, or other logically meaningful positions. A **strat** is a directed way to move from one node to another and carries an ID, description, and a condition over acquired items and enabled tech. Tech represent player skills, fine-grained difficulty settings which may be individually toggled on or off for seed generation.

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

Successful seed generation results in a seed JSON object representing the following core data: area placement coordinates, selected theme and edge variants, entrance connections, and item placements. The seed JSON also records the randomizer version, the source commit that it was built with, and the RNG seed used.

Conceptually, rearrangement can be considered part of the generation process, and it is possible that they are combined in a single binary. However, because of the likely language boundary (with rearrangement likely happening primarily in Python), it may be more convenient for them to be separate services. On the other hand, there is also a possibility of building an offline pool of rearrangements, which would eliminate the need for rearrangement as a generation-time service.

## ROM patching

### Seed-specific retiling data

As a preliminary step in patching, the full retiling catalog is reduced to a seed-specific `SeedRetilingData` object containing only the selected area variants and the palettes and tiles needed to patch that seed. It must retain sanitized vanilla-graphics references rather than expanding them. For the randomizer web service, `SeedRetilingData` is constructed on the server side and then transmitted to the client (alongside other data including the seed JSON and the IPS patches), allowing it to patch the ROM without receiving the entire retiling catalog. `SeedRetilingData` will be encoded using `bincode-next`, using the same envelope rules as the other binary-encoded data.

### Patcher

The patcher is a Rust library for transforming a user-provided vanilla ROM into a randomized ROM based on a generated seed. Its core is a pure function that receives the ROM, IPS patches, symbol manifest, seed JSON, `SeedRetilingData`, and customization settings, and returns an output ROM. It has no filesystem, network, clock, or RNG dependency.

The patcher first validates the user ROM against a static checksum, copies it to a new output ROM buffer, and applies the stored IPS. It then resolves sanitized vanilla graphics references from the ROM and populates seed-specific assets and other data into locations specified in the symbol manifest. Finally, it updates the checksum and complement.

The native CLI calls the core function directly. A WebAssembly build exposes the same API to the TypeScript frontend, serializing the inputs to byte buffers. The behavior is identical in the native and WebAssembly builds.

## User interfaces

### CLI

The CLI is a native Rust binary built on the Rust library. Its seed-generation command accepts a settings JSON and the retiling and logic catalogs, then writes a seed JSON. Its patching command accepts customization settings, the seed JSON, the IPS patches, and a local verified ROM, then writes an output ROM locally.

### Web backend

The web backend is a Rust service built on the Rust library. It loads the logic and retiling catalogs, connects to an object store for seed storage, and exposes an HTTP JSON API for seed generation and other services needed by the frontend.

### Web frontend

The web frontend is a TypeScript application that calls the backend JSON API to request generation and fetch stored seed data. It obtains the compatible WebAssembly package built from the Rust library, plus the IPS patches and seed-specific retiling data. It selects the player's ROM through a local file input, invokes the library's WebAssembly patching API in the browser, and offers the returned ROM bytes as a local download. The ROM and output exist only on the player's system.

## Crate boundaries

Keeping the generator and patcher in separate crates cleanly manages dependencies because the WebAssembly package depends only on the patcher; for example, this prevents it from pulling in generator-specific dependencies that are unavailable or unneeded on that platform. The generator and patcher Rust libraries are primarily geared toward internal use: they are not published to crates.io, and their APIs are unstable, but they are nevertheless designed with an understanding that other projects might want to reuse them, such as for multiworld integration or plandos. Therefore, we make an effort to present a relatively clean, flexible API that could serve most such needs.
