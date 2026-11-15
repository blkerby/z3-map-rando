# ALTTP Map Randomizer frontend

Web UI for the randomizer (SvelteKit + Svelte 5 + Bootstrap), built as static files with `adapter-static`.

## Develop

```sh
npm install
npm run dev
npm run check   # type-check (svelte-check)
npm run build   # static site in build/
```

Open the URL printed by Vite for frontend development. Its Node-based development server compiles Svelte and provides hot reload; frontend edits do not require starting or restarting the Rust server. Node is also needed to build the frontend, but is not needed to serve the built site.

The frontend also requires `wasm-pack` and the Rust `wasm32-unknown-unknown` target. Vite builds `crates/browser` into the ignored `build/browser/` directory when loading its configuration, including for `dev`, `check`, and `build`. Generate initializes that package once and reads its settings-format version for generation requests. During this initial build milestone, restart Vite after editing the Rust browser code; automatic Rust-source watching is tracked in [the browser settings plan](../plans/browser-settings.md).

## Serve the built site

From the repository root, after building the frontend:

```sh
cargo run --package server
```

Open `http://127.0.0.1:8080/`. The server reads `frontend/build/` from disk, so rebuilding the frontend does not require recompiling or restarting Rust; refresh the browser to see the new build. Pages use trailing slashes and are emitted as directory indexes, such as `generate/index.html`; `/generate` redirects to `/generate/`.

The server accepts `--bind` and `--static-dir` (relative to the current working directory). For example, to serve a deployed build using the compiled binary:

```sh
z3-map-rando-server --bind 0.0.0.0:8080 --static-dir /path/to/frontend/build
```

Run the Rust server alongside Vite to generate seeds during development. Vite forwards `/api/` requests to `http://127.0.0.1:8080`; use Vite's URL for hot reload. See [the backend setup](../docs/backend.md) for catalogs and seed storage configuration.

## Settings

The settings data (built-in presets, skill difficulties, items) comes from `settings_data.json`, which the randomizer's catalog builder prepares from its `data/` directory (the `$settings_data` alias). `vite.config.ts` runs the builder whenever the config loads, and in dev again whenever `data/` changes, so the Rust toolchain is required. See [`docs/data-formats`](../docs/data-formats/) for the settings document format.

The Generate page is written directly as Svelte components (`src/lib/generation/GenerationSettings.svelte`, `src/lib/customization/CustomizationSettings.svelte`). Each setting component names its path in the settings (e.g. `['skill', 'tech', '5']`) and registers its label with the settings store while mounted, which is how the summary panel and search know about it. The store (`src/lib/settings/store.svelte.ts`) handles presets, tweaks, and unsaved changes by path.

## ROM

You need your own copy of the game. The file stays in the browser (IndexedDB) and is never uploaded. ROMs are git-ignored; do not commit them.
