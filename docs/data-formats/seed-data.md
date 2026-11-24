# Seed data

Status: draft, partially written. A seed record is everything stored for a generated seed. During pre-release development, backward compatibility is not required: seed formats may change without migrations, and existing prototype seeds may become unreadable or unplayable. The long-term compatibility requirements below describe intended post-release behavior.

## Contents

The JSON record stores resolved values in `settings` and their provenance in `settings_document`, the cleaned settings document. Generation results are stored per room in `rooms`, each identified by its `room_id`, with optional fields: `position` (the top-left map cell `[x, y]` of an overworld room in a randomized layout), `theme` (the retiling theme ID of a room drawn as an overworld area, applying to both worlds), and `items` (the `item_id` and `item` of each of the room's item locations). `borders` lists the open borders of a randomized layout, each by the `[x, y]` cell `position` on its west or north side, its `side` (`east` or `south`), and its edge `variant`; unlisted borders are closed. The CLI writes plain JSON; the backend's object store transparently applies Zstd compression to stored objects.

A seed record contains at least:

- the release that generated it;
- the **pinned settings document** used for generation (see [settings.md](settings.md#live-and-pinned-settings));
- the **resolved values** generation actually used;
- the generation results (to be written).

The pinned settings document is the public form of the creator's settings document: it includes unsaved edits as tweaks and carries no user preset name or other user metadata. References to built-in presets are not private and are kept as the creator selected them.

## Settings

The resolved values are authoritative for what generation used. They retain the scope hierarchy with concrete scalar values in place of preset references and sparse value entries. A behavior-preserving migration of these values supplies the seed baseline for customizable patch settings and racing rules, without needing current built-in preset resolution at patch time.

The pinned settings document explains those values. The preset definitions it references are not stored, so the seed does not record the defaults that its tweaks replaced; a baseline for tweak highlighting could be added later as an optional field.

To display a seed, the backend migrates a copy of its settings document and resolved values to the current setting definitions, preserving behavior as closely as possible.

The frontend displays the result with current Svelte components and [bundled display data](settings-data.md). New settings receive historical compatibility values from migration; renamed or removed options receive the closest current representation, with any limitation reported. No historical UI description or renderer is needed.

Newly generated seeds use this same inspection path, including seeds requested by an outdated frontend. The displayed values come from the backend's seed data, not from re-resolving the request against the frontend's bundled presets. Generation can also return notices explaining that older settings were upgraded.

The current patcher reads customizable gameplay values by their current keys. The same behavior-preserving migrations, including [vocabulary rules](settings.md#vocabulary-rules), therefore apply before its inputs are constructed. The stored seed is never rewritten. Importing its settings document for future generation additionally performs the separate live-settings update to current built-in presets/defaults; historical viewing and patching omit that update.

## Reading seed records

Seed records, including their pinned settings documents, are read strictly. Malformed input and identifiers that are neither current nor retired are errors, reported clearly and left unmodified in storage. A seed record that cannot be read is a bug to fix rather than something to approximate.

Recognized historical value conversions may report an unavoidable approximation; unknown input must not be silently replaced by today's preferred defaults.

## Versioning and migration

Seed records carry no format version (see [common principles](README.md#common-principles)). Compatible extensions, such as optional fields whose absence keeps the old meaning, need no migration.

Identifiers saved in generation results, including room and item IDs, Small Key door IDs, and border variant names, are permanent. Seeds are interpreted against current catalogs: the patcher selects border graphics from the current retiling catalog by variant name, and the [spoiler](../../plans/spoiler.md) builds overworld connections from the current logic catalog's edge variants. Such an identifier may therefore be removed or renamed only together with a migration that maps it to a current equivalent preserving connectivity and gameplay meaning. The form of these migrations is not yet designed.

## To be written

- Generation results needed for viewing and patching.
- Spoiler log.
- Captured racing ruleset (see the [customization plan](../../plans/customization.md#racing-rulesets)).
- Seed IDs and URL scheme.
- Re-patching policy: whether old seeds are patched by the current patcher, and which generation results that makes a permanent contract.
