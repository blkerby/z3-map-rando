# Randomizer backend

Run commands from the repository root. The server loads `logic_catalog.bin`, `retiling_catalog.bin`, and `preset_catalog.bin` from the `output_directory` in `catalog-build.toml` (default `build/`), and reads the configured `map_pool` (default `maps/map_pool.bin`). Prepare the catalogs with `cargo run --package catalog_builder -- all`. The server loads catalogs once at startup; restart it after rebuilding them.

Build the frontend with `npm --prefix frontend run build`, then start the server:

```sh
cargo run --package server -- --store mem
```

The server serves the built site at `http://127.0.0.1:8080/`. For frontend hot reload, also run `npm --prefix frontend run dev` and open Vite's URL. Its `/api/` proxy targets the Rust server on port 8080.

## Seed storage

`--store` selects one shared store for all server workers:

| Value | Storage |
| --- | --- |
| `mem` | In memory, until the server exits; the default |
| `file:path/to/store` | Filesystem directory, created if needed; relative to the working directory |
| `s3://bucket/prefix` | S3-compatible bucket, with all objects under the optional prefix |

For S3, use the `object_store` library's AWS environment configuration: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, optional `AWS_SESSION_TOKEN`, `AWS_DEFAULT_REGION`, and a custom `AWS_ENDPOINT_URL_S3` for compatible services. HTTP endpoints require `AWS_ALLOW_HTTP=true`. The bucket must already exist, and the provider must support conditional create-only PUTs. Storage errors never fall back to another backend.

Applications save and load whole objects using logical keys. The store compresses every saved object with Zstd level 3, appends `.zst` to its physical key, and decompresses when loading through the logical key. For example, `seeds/ABC123.json` is stored as `seeds/ABC123.json.zst`, or `prefix/seeds/ABC123.json.zst` inside the configured S3 bucket. Saved objects use atomic, create-only writes and cannot overwrite an existing key. Keep filesystem storage outside `frontend/build/`.

## Generation

`POST /api/generate` accepts a JSON object containing a scoped `settings` document. Settings retain built-in preset selections and sparse tweaks, rather than requiring resolved values. See [the settings format](data-formats/settings.md).

```json
{"settings":{"format_version":1,"revision":"<revision from settings_data.json generation.revision>","preset":"default","map_layout":"vanilla"}}
```

The backend processes settings strictly: it applies migrations, resolves current preset scopes, validates schema and semantic constraints, normalizes redundant tweaks, and calls the generator. Unknown keys, unknown presets, and invalid values return HTTP 422 with structured findings rather than being recovered; source-revision updates remain notices. The current settings format is version 1; no historical structural or vocabulary migrations are needed yet. Item pools must satisfy the generator's exact ordinary-location count. Invalid pool sizes return HTTP 422 before allocating the item pool; generation and storage failures return HTTP 500.

Requests wait asynchronously in acquisition order for one shared generation slot. Generation, serialization, and compression run on blocking workers, and storage uploads are awaited before releasing the slot. The queue is process-local and keeps HTTP requests open; it does not survive server restart and does not supply job IDs or polling. Deployment proxy timeouts must accommodate queueing and generation.

After successfully saving the seed, the server returns HTTP 201 with `{"seed":"<seed name>","notices":[]}`. No seed data is returned. Each saved seed contains its resolved generation settings, cleaned public settings document, source revision, current definitions revision, captured preset baselines, metadata, and generation results. Retrieval and customization endpoints are separate work.

The CLI uses the same processor in strict mode by default. Pass `--recover-settings` alongside `--settings path/to/settings.json` to remove unknown entries and restore invalid submitted entries from the selected baseline, with notices. Cross-setting failures remain blocking. It shares the seed metadata helpers, but reads and writes plain `.json` seeds. Compression belongs to object storage, independently of seed serialization.
