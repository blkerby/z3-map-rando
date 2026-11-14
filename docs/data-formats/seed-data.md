# Seed data

Status: draft, partially written. A seed record is everything stored for a generated seed. Seed records from every deployment, including dev-site builds, must remain viewable and playable indefinitely.

## Contents

A seed record contains at least:

- its format version and the release that generated it;
- the **pinned settings document** used for generation, with its source settings data revision identifying the shared definitions and presets used by the generating build (see [settings.md](settings.md#live-and-pinned-settings));
- the bundled built-in preset definitions needed to resolve that settings document, including required transitive references and concrete default values;
- the **resolved values** generation actually used;
- the generation results (to be written).

The pinned settings document is the public form of the creator's settings document: it includes unsaved edits as tweaks and carries no user preset name or other user metadata. References to built-in presets are not private and are kept as the creator selected them.

## Settings

The resolved values are authoritative for what generation used. They retain the scope hierarchy with concrete scalar values in place of preset references and sparse value entries. A behavior-preserving migration of these values supplies the seed baseline for customizable patch settings and racing rules, without needing current built-in preset resolution at patch time.

The pinned settings document and bundled presets explain those values. The bundle captures required defaults and preset applications explicitly, so resolution does not need a historical shared-data export. It records preset scopes where needed, but not UI layout or control definitions.

To display a seed, the backend migrates a copy of its settings document, bundled definitions, and resolved values to the current setting definitions, preserving behavior as closely as possible. Migrations apply inside captured presets as well as to final values. Resolve the migrated settings document against its bundle and use that baseline for tweak highlighting. Do not replace captured presets or defaults with current ones, or filter their entries by current scope membership.

The frontend displays the result with current Svelte components and [bundled display data](settings-data.md). New settings receive historical compatibility values from migration; renamed or removed options receive the closest current representation, with any limitation reported. No historical UI description or renderer is needed.

Newly generated seeds use this same inspection path, including seeds requested by an outdated frontend. The displayed values and tweak baselines come from the backend's seed data, not from re-resolving the request against the frontend's bundled presets. Generation can also return notices explaining that older settings were upgraded.

Re-resolution must agree with the migrated authoritative values. A mismatch indicates a bug in migration or resolution and is reported; it is not repaired by substituting current presets. Apply equivalent historical conversions to the explanation and concrete snapshot so they continue to describe the same generation inputs.

The current patcher reads customizable gameplay values by their current keys. The same behavior-preserving migrations, including [vocabulary rules](settings.md#vocabulary-rules), therefore apply before its inputs are constructed. The stored seed is never rewritten. Importing its settings document for future generation additionally performs the separate live-settings update to current built-in presets/defaults; historical viewing and patching omit that update.

## Reading seed records

Seed records, including their pinned settings documents, are read strictly. Malformed input, identifiers that are neither current nor retired, and records from a newer format version are errors, reported clearly and left unmodified in storage. A seed record that cannot be read is a bug to fix rather than something to approximate.

Bundled preset references are checked against their captured registry rather than only the current preset bundle's IDs. A captured built-in may have been removed from the current bundle and still be valid in the seed's bundle. Recognized historical value conversions may report an unavoidable approximation; unknown input must not be silently replaced by today's preferred defaults.

## Versioning and migration

The seed record format has its own format version. A commit changing the structure of the seed record or its generation results increments it and adds a structural migration step. Because the increment is made in the commit making the change, seed records from dev-site deployments are upgraded by the same steps as release seeds.

## To be written

- Generation results needed for viewing and patching.
- Spoiler log.
- Captured racing ruleset.
- Seed IDs and URL scheme.
- Re-patching policy: whether old seeds are patched by the current patcher, and which generation results that makes a permanent contract.
