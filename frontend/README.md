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

The settings data is imported at build time from the randomizer's `data/` directory (the `$data` alias): `settings.json` holds the default settings, `tech_data.json` and `notable_data.json` the difficulties that skill presets are derived from, and `items.json` the item pool's items. See [`docs/data-formats`](../docs/data-formats/) for the settings document format.

The Generate page is written directly as Svelte components (`src/lib/generation/GenerationSettings.svelte`, `src/lib/customization/CustomizationSettings.svelte`). Each setting component names its path in the settings (e.g. `['skill', 'tech', '5']`) and registers its label with the settings store while mounted, which is how the summary panel and search know about it. The store (`src/lib/settings/store.svelte.ts`) handles presets, tweaks, and unsaved changes by path.

## ROM

You need your own copy of the game. The file stays in the browser (IndexedDB) and is never uploaded. ROMs are git-ignored; do not commit them.
