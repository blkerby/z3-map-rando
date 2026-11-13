# Randomizer backend API

Status: draft. This document describes the planned HTTP endpoints and their request/response wrappers. Exact endpoint names and wrapper fields remain open.

## Stability

There is no stability commitment for this API. Its main consumer is the frontend, which is deployed together with the backend, so endpoints and wrapper fields may change in any release. Persisted [settings documents](settings.md) and [seed records](seed-data.md) keep their own compatibility rules. Supporting old saved data does not require maintaining every historical frontend/API pairing.

## Endpoints

- **Generation:** generate a seed from a [settings document](settings.md), migrating and updating live settings before generation.
- **Seed data:** return a generated seed's settings and bundled presets migrated to the current representation. Pinned references use that bundle rather than current built-ins; the stored record remains unchanged.
- **Logic data:** supply runtime logic features as needed. The settings editor does not depend on this endpoint to obtain technique, notable, proficiency, or preset data.

The frontend bundles [shared settings data](settings-data.md) at build time. There is no settings-catalog endpoint or runtime catalog-fetch/cache protocol.

## Generation

A generation request consists mainly of a scoped settings document, including its format version, source settings data revision, preset selections, and tweaks. It preserves inheritance rather than turning every resolved value into an explicit tweak.

The backend uses the ordinary live-settings path: migrate while preserving meaning, adopt current built-in presets/defaults for inherited values, validate, then generate. This also handles requests from an outdated frontend.

The response supplies the authoritative settings actually used and their frozen preset bundle, either directly or through the resulting seed's data endpoint. It also supplies migration/update notices where applicable. For example, the frontend may report: "Your settings were from an older version and were updated." The exact response fields remain open.

The user can inspect those settings through the normal generated-seed view. An outdated client does not need a separate preview or version-negotiation workflow before generation. If a frontend is too old to display the returned representation, ordinary refresh/recovery handling can apply; optimizing this uncommon case is not a design priority.

## Seed inspection

Newly generated and historical seeds use the same inspection path. The backend migrates concrete settings and captured presets to the current representation while preserving their behavior; it does not apply live preset/default updates.

The frontend presents that result with its Svelte components and bundled display data. The returned values and captured baselines are authoritative, including when the seed was just generated from an older frontend. The frontend must not reconstruct the generation settings using its own bundled built-ins.
