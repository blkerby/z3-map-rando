# Data formats

Status: draft. This directory defines the data formats that are persisted or exchanged by the randomizer. Most of them carry a commitment to indefinite backward compatibility: data written by any deployment, including dev-site builds, must remain usable by every later deployment. Therefore, these formats deserve special attention, and so are designed and documented separately from the implementation plans in [`plans/`](../../plans/).

## Documents

| Document | Covers | Stability |
| --- | --- | --- |
| [identifiers.md](identifiers.md) | Setting keys and paths, option tokens, preset scopes, built-in preset IDs, data-driven families, vocabulary rules, upstream ID dependency | Permanent |
| [settings-data.md](settings-data.md) | Shared build-time data, source ownership, scopes, preset files, frontend-owned presentation | Internal build inputs |
| [settings.md](settings.md) | Scoped live and pinned settings, preset resolution, migration and normalization, lenient loading, share links, customization presets | Permanent |
| [seed-data.md](seed-data.md) | Seed record, pinned settings document and bundled presets, resolved values, generation results, spoiler, captured ruleset, seed IDs, re-patching | Permanent |
| `autotracker.md` (not yet written) | Memory contract exposed to autotrackers | Permanent |
| [backend-api.md](backend-api.md) | Generation requests, upgrade notices, seed inspection, and runtime response wrappers | No stability commitment |

SRAM layout is internal and may change freely, so it is not covered here.

## Common principles

- **Each format has its own format version**, recorded in every document of that format. For persisted formats, it is incremented manually by the commit making a change that requires migrating existing documents, together with the corresponding migration. Releases do not increment it. A format's document may define exceptions.
- **Every permanent format version has committed sample documents.** Each sample exercises the fields of its format version. CI runs every sample through the migration chain to the current version and checks that it loads and matches its expected result. A missing or wrong migration, or a change to the stored format through Serde attributes, then fails CI. Incrementing a format version includes adding samples for the new version.
- **Migrations preserve behavior as closely as possible.** Structural and semantic migration bring persisted settings and bundled presets into the current settings representation. Updating inherited values to current built-in presets and defaults is a separate operation applied only to live settings. Pinned settings resolve against their migrated bundled presets. The frontend displays migrated seed data with current Svelte components and bundled display data; historical UI descriptions are not retained or required.
