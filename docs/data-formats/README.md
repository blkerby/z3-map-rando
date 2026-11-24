# Data formats

Status: draft. This directory defines the data formats that are persisted or exchanged by the randomizer. Most of them carry a commitment to indefinite backward compatibility: data written by any deployment, including dev-site builds, must remain usable by every later deployment. Therefore, these formats deserve special attention, and so are designed and documented separately from the implementation plans in [`plans/`](../../plans/).

## Documents

| Document | Covers | Stability |
| --- | --- | --- |
| [settings-data.md](settings-data.md) | Shared build-time data, source ownership, scopes, preset files, frontend-owned presentation | Internal build inputs |
| [settings.md](settings.md) | Scoped live and pinned settings, setting identifiers, preset resolution, vocabulary rules, migration and normalization, lenient loading, share links, customization presets | Permanent |
| [seed-data.md](seed-data.md) | Seed record, pinned settings document and bundled presets, resolved values, generation results, spoiler, captured ruleset, seed IDs, re-patching | Permanent |
| `autotracker.md` (not yet written) | Memory contract exposed to autotrackers | Permanent |
| [backend-api.md](backend-api.md) | Generation requests, upgrade notices, seed inspection, and runtime response wrappers | No stability commitment |

SRAM layout is internal and may change freely, so it is not covered here.

## Common principles

- **Formats carry no format version.** Compatible changes, such as optional fields whose absence keeps the old meaning, need none. A version field can be introduced if a change ever requires a structural migration that cannot recognize older documents by their shape.
- **Migrations preserve behavior as closely as possible.** Structural and semantic migration bring persisted settings and bundled presets into the current settings representation. Updating inherited values to current built-in presets and defaults is a separate operation applied only to live settings. Pinned settings resolve against their migrated bundled presets. The frontend displays migrated seed data with current Svelte components and bundled display data; historical UI descriptions are not retained or required.
