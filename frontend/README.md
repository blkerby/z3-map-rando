# ALTTP Map Randomizer frontend

Web UI for the randomizer (SvelteKit + Svelte 5 + Bootstrap), built as static files with `adapter-static`.

## Develop

```sh
npm install
npm run dev
npm run check   # type-check (svelte-check)
npm run build   # static site in build/
```

## Settings

The settings data (built-in presets, skill difficulties, items) comes from `settings_data.json`, which the randomizer's catalog builder prepares from its `data/` directory (the `$settings_data` alias). `vite.config.ts` runs the builder whenever the config loads, and in dev again whenever `data/` changes, so the Rust toolchain is required. See [`docs/data-formats`](../docs/data-formats/) for the settings document format.

The Generate page is written directly as Svelte components (`src/lib/generation/GenerationSettings.svelte`, `src/lib/customization/CustomizationSettings.svelte`). Each setting component names its path in the settings (e.g. `['skill', 'tech', '5']`) and registers its label with the settings store while mounted, which is how the summary panel and search know about it. The store (`src/lib/settings/store.svelte.ts`) handles presets, tweaks, and unsaved changes by path.

## ROM

You need your own copy of the game. The file stays in the browser (IndexedDB) and is never uploaded. ROMs are git-ignored; do not commit them.
