# Randomizer backend

Run commands from the repository root. For development, `npm run dev` runs the server together with the frontend's development server (see [the frontend README](../frontend/README.md#develop)); arguments after `--` go to the server, e.g. `npm run dev -- --store mem`. To serve the built site at `http://127.0.0.1:8080/` instead:

```sh
npm run build
cargo run --release --package server -- --store mem
```

The server loads `logic_catalog.bin`, `retiling_catalog.bin`, `patch_catalog.bin`, and `preset_catalog.bin` from the `output_directory` in `catalog-build.toml` (default `build/`), and reads the configured `map_pool` (default `maps/map_pool.bin`), once at startup. Building the server updates the catalogs whenever their inputs change, using the same `catalog-build.toml` as the CLI.

## Seed storage

`--store` selects one shared store for all server workers:

| Value | Storage |
| --- | --- |
| `mem` | In memory, until the server exits; the default |
| `file:path/to/store` | Filesystem directory, created if needed; relative to the working directory |
| `s3://bucket/prefix` | S3-compatible bucket, with all objects under the optional prefix |

For S3, use the `object_store` library's AWS environment configuration: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, optional `AWS_SESSION_TOKEN`, `AWS_DEFAULT_REGION`, and a custom `AWS_ENDPOINT_URL_S3` for compatible services. HTTP endpoints require `AWS_ALLOW_HTTP=true`. The bucket must already exist. Storage errors never fall back to another backend.

Applications save and load whole objects using logical keys. The store compresses every saved object with Zstd level 3, appends `.zst` to its physical key, and decompresses when loading through the logical key. For example, `seeds/ABC123.json` is stored as `seeds/ABC123.json.zst`, or `prefix/seeds/ABC123.json.zst` inside the configured S3 bucket. Writes are atomic and create-only by default. Keep filesystem storage outside `frontend/build/`.

Pass `--store-check-before-put` for stores that lack conditional create-only PUTs, such as Backblaze B2. The server writes only when `HEAD` reports that the key is missing. This check and the write are separate operations, so concurrent writers targeting the same key can overwrite each other.

## Generation

`POST /api/generate` accepts a JSON object containing a scoped `settings` document. Settings retain built-in preset selections and sparse tweaks, rather than requiring resolved values. See [the settings format](data-formats/settings.md).

```json
{"settings":{"format_version":1,"preset":"default","map_layout":"vanilla"}}
```

The backend processes settings strictly: it applies migrations, resolves current preset scopes, validates schema and semantic constraints, normalizes redundant tweaks, and calls the generator. Unknown keys, unknown presets, and invalid values return HTTP 422 with structured findings rather than being recovered. The current settings format is version 1; no historical structural or vocabulary migrations are needed yet. Item pools after optional starting-equipment deduction may be smaller than the ordinary-location count; unused locations are empty. Oversized pools return HTTP 422 before allocating the item pool; generation and storage failures return HTTP 500.

Requests wait asynchronously in acquisition order for one shared generation slot. Generation, serialization, and compression run on blocking workers, and storage uploads are awaited before releasing the slot. The queue is process-local and keeps HTTP requests open; it does not survive server restart and does not supply job IDs or polling. Deployment proxy timeouts must accommodate queueing and generation.

After successfully saving the seed, the server returns HTTP 201 with `{"seed":"<seed name>","notices":[]}`. No seed data is returned. Each saved seed contains its resolved generation settings, cleaned public settings document, metadata, and generation results.

## Settings processing

`POST /api/settings/process` processes one stored or imported settings document in recovery mode, so the frontend can load it. The request names the settings context, `generation` or `customization`:

```json
{"kind":"generation","document":{"format_version":1,"preset":"default","map_layout":"weird"}}
```

The response always has HTTP 200 and the server's settings `revision`. A `status` of `processed` comes with the cleaned `document`, the `notices` describing what recovery changed, and the cross-setting `errors` left for the player to fix, such as an item pool larger than the available locations. A `status` of `unrecoverable` means recovery failed even without tweaks, which indicates a bug; it is logged, and the frontend can only offer to discard the document. Other HTTP errors are about the request or server, not the document.

## Seed retrieval

`GET /api/seed/<seed name>` returns the saved seed JSON, or HTTP 404 if no such seed exists. The frontend's seed link is `/seed/<seed name>/`; seeds are not known at build time, so the server answers that path with the frontend's SPA fallback page (`spa.html`), which loads the seed through the API.

`GET /api/seed/<seed name>/retiling` returns the binary retiling catalog for that seed. The server resolves static layers and dynamic tile dependencies from the current catalogs on request. Save ROM uses the result with the matching browser patcher and patch catalog. The CLI can use it when patching the same seed with `--seed` and `--retiling-catalog`.

The CLI uses the same processor in strict mode by default. Pass `--recover-settings` alongside `--settings path/to/settings.json` to remove unknown entries and restore invalid submitted entries from the selected baseline, with notices. Cross-setting failures remain blocking. It shares the seed metadata helpers, but reads and writes plain `.json` seeds. Compression belongs to object storage, independently of seed serialization.
