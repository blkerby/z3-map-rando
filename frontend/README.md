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

## ROM

You need your own copy of the game. The file stays in the browser (IndexedDB) and is never uploaded. ROMs are git-ignored; do not commit them.
