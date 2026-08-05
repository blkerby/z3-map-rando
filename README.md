# Overview

The goal of this project is to create a randomizer for generating randomized *A Link to the Past* games, featuring a randomly rearranged and rethemed overworld. The randomizer will be usable primarily as a web service, but also as a library and command-line tool.

# Current status

An [overworld editor](https://github.com/blkerby/Z3OverworldEditor) exists, based around the ability to define and use custom palettes and tilesets with a degree of flexibility beyond what the vanilla game engine provides. The editor is functional enough to enable artists to create new themes (in progress [here](https://github.com/kjbranch/ALTTPRetiling)), but the work needed to translate this into a playable ROM is mostly still in a planning stage. The editor also still needs development, 1) to define structured variants of area perimeter/edge segments so they can connect cleanly with other areas after rearrangement, 2) to support customizing animations, including dungeon openings, 3) to support redrawing the Mode 7 overworld map. Currently the editor focuses on retiling the overworld BG2 layer and associated collision data, as this is primarily what is needed for the randomizer project; it may later be extended with support to customize other game elements such as enemies, entrances, secrets, items, and dungeons.

The randomizer itself, including item-placement logic, and the command-line tool and web service, are also in a planning stage. A [design doc](https://docs.google.com/document/d/1skZsIxZLKbCC8C_-3b3TTZVwblzNkN2ZTXraZ4Zxzkc) describes the planned tier-based item placement logic along with several balance-oriented gameplay changes. 

The current focus is on the technical foundation for the project, particularly the game engine changes needed to support the flexible palette and tileset system. For detailed plans, see the docs in [plans](plans/README.md).

# How to test current engine changes

Clone the repo:

```sh
git clone --recurse-submodules https://github.com/blkerby/z3-map-rando.git
```

Install Rust and Cargo with [rustup](https://www.rust-lang.org/tools/install); for example, on Linux or macOS:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Build a test ROM:

```sh
cargo run -p engine_check -- path/to/vanilla.sfc path/to/output.sfc
```

The input must be an unheadered 1 MiB ROM (original Japanese version), having SHA-256 digest `794e040b02c7591b59ad8843b51e7c619b88f87cddc6083a8e7a4027b96a2271`. The `engine_check` tool verifies the input, expands a copy to 2 MiB, applies the current engine patches, and writes the output ROM. Open the output ROM in an emulator or SNES-compatible platform to test the changes. If successful, currently it should behave just like the vanilla game, except with somewhat reduced overworld loading/transition times. 

To build the milestone 9C Desert-theme checkpoint instead, use:

```sh
cargo run -p theme_check -- path/to/vanilla.sfc path/to/desert.sfc
```

`theme_check` reads the Desert JSON from the checked-out `ALTTPRetiling`
submodule and produces a 4 MiB ROM. Authored animation tracks are supported;
other dynamic overworld graphics remain incomplete.

If you notice any issue while testing or if you run into any trouble following these instructions, please reach out in the [Discord](https://discord.gg/Mxb5zYZeVj) to let us know. Even in this early phase of the project, playtesting is very helpful!

# How to build the patches from source

The instructions above use the IPS patches already checked into the repo. If you want to build them scratch, do the following:

Install a C++ compiler and CMake; for example, on Ubuntu:

```sh
sudo apt install build-essential cmake
```

From the repository root, build the Asar assembler:

```sh
python3 scripts/build_asar.py
```

Build the ASM sources into IPS patches:

```sh
python3 scripts/build_patches.py
```

Note that we are using a custom version of `asar` that is modified to produce an IPS patch rather than modify a ROM in place; so it must be built as described here rather than using some other version of it.
