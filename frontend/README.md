# ALTTP Map Randomizer frontend

Web UI for the randomizer (SvelteKit + Svelte 5 + Bootstrap), built as static files with `adapter-static`.

## Develop

From the repository root:

```sh
npm install
npm run dev                         # then open the URL printed by Vite
npm run dev -- --store file:seeds   # arguments after -- go to the randomizer server
```

`npm run dev` runs Vite with hot reload together with the randomizer server, which serves only the API; Vite forwards `/api/` requests to it. When other files in the repository change, it rebuilds and restarts the server and rebuilds the browser package as needed (see `scripts/dev.js`). Inside `frontend/`, `npm run check` type-checks. Node is needed to develop and build the frontend, but not to serve the built site.

The frontend also requires `wasm-pack` and the Rust `wasm32-unknown-unknown` target. Vite builds `crates/browser` into the ignored `build/browser/` directory when loading its configuration, including for `dev`, `check`, and `build`. Save ROM uses that package to patch the player's ROM in the browser with the seed from `/api/seed/<name>`, its filtered catalog from `/api/seed/<name>/retiling`, and the patch catalog bundled by Vite.

## Serve the built site

From the repository root:

```sh
npm run build
cargo run --release --package server
```

Open `http://127.0.0.1:8080/`. The server reads `frontend/build/` from disk, so rebuilding the frontend does not require recompiling or restarting Rust; refresh the browser to see the new build. Pages use trailing slashes and are emitted as directory indexes, such as `generate/index.html`; `/generate` redirects to `/generate/`. Seed pages (`/seed/<name>/`) are not prerendered; the server answers them with the `spa.html` fallback.

The server accepts `--bind` and `--static-dir` (relative to the current working directory). For example, to serve a deployed build using the compiled binary:

```sh
z3-map-rando-server --bind 0.0.0.0:8080 --static-dir /path/to/frontend/build
```

See [the backend setup](../docs/backend.md) for catalogs and seed storage configuration.

## Settings

The settings data (built-in presets, skill difficulties, items) comes from `settings_data.json`, which the randomizer's catalog builder prepares from its `data/` directory (the `$settings_data` alias). `vite.config.ts` runs the builder whenever the config loads, and in dev the dev script updates it again whenever `data/` changes, so the Rust toolchain is required. See [`docs/data-formats`](../docs/data-formats/) for the settings document format.

The Generate page is written directly as Svelte components (`src/lib/generation/GenerationSettings.svelte`, `src/lib/customization/CustomizationSettings.svelte`). Each setting component names its path in the settings (e.g. `['skill', 'tech', '5']`) and registers its label with the settings store while mounted, which is how the summary panel and search know about it. The store (`src/lib/settings/store.svelte.ts`) handles presets, tweaks, and unsaved changes by path.

## Logic pages

The Logic pages (`/logic/`) show the [`z3-json-data`](../z3-json-data/) logic: an index of rooms by type, a page per room (`/logic/<room_id>/`), and a page per strat (`/logic/<room_id>/<strat_id>/`, using the strat's stable `id`). They are prerendered from the room files at build time by `src/lib/server/logic.ts` (through the `$z3_json_data` alias), with each strat's difficulty from the catalog builder's `logic_data.json`: the first built-in skill preset, easiest first, that the strat passes with (or Ignored). A strat passes when, with every item and progress flag owned, one of its compiled edges succeeds from unused resources with any follower possible, for some combination of cleared room obstacles (`crates/generator/src/strat_check.rs`). Requirements and other strat properties are shown as their source JSON text, sliced from the room file so they keep its formatting. Diagrams come from the `roomDiagrams` folders (`src/lib/logic/diagrams.ts`): overworld rooms by file prefix, dungeons by name and floor, one floor at a time. Dev notes are hidden unless enabled with a toggle, remembered in the browser. A room page can list only the strats passing with a difficulty: a built-in skill preset, or a built-in or saved root preset that tweaks its skill preset (one without skill tweaks would repeat a listed option). The check runs in the browser through the `browser` WebAssembly package (`src/lib/logic/difficulty.ts`). A saved preset from an older settings revision is processed on the server when chosen, saved back, and what processing changed is shown. The choice is remembered in the browser.

## ROM

You need your own copy of the game. The file stays in the browser (IndexedDB) and is never uploaded. ROMs are git-ignored; do not commit them.
