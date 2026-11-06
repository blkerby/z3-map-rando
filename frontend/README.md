# ALTTP Map Randomizer

Web UI for a *A Link to the Past* map randomizer (SvelteKit + Svelte 5 + Bootstrap).

## Develop

```sh
npm install
npm run dev
npm run check   # type-check (svelte-check)
```

## How the Generate page works

Every setting on the page comes from one file, `src/lib/master/master_data.json`
(tech, notables, proficiencies, map options, item counts). The page reads it through
`loadMaster()` in `src/lib/master/load.ts`; the generator is meant to read the same file.

- Format: `src/lib/master/master.ts` (types) and `adapter.ts` (file -> page).
- Filler items (`"calculated": true`) share out the leftover item slots: `src/lib/master/pool.ts`.
- Checks: `node scripts/validate-master.mjs ...` and
  `node --experimental-strip-types scripts/test-pool.mjs`.

## ROM

You need your own copy of the game. The file stays in the browser (IndexedDB) and is never
uploaded. ROMs are git-ignored; do not commit them.
