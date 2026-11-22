# Settings

Status: draft. A settings document organizes values and preset selections hierarchically by scope. Drafts, saved user presets, files, share links, and generated seeds use this format.

During pre-release development, backward compatibility is not required: settings, presets, share links, and saved seeds may change without migrations. The compatibility and migration requirements below describe intended post-release behavior.

## Live and pinned settings

Every settings document uses the same format but is used in one of two modes:

- A **live settings document** is migrated to the current setting definitions while preserving the meaning of its entries. A separate live-settings update then adopts current built-in presets and defaults for untweaked values. Drafts, saved user presets, files, and share links are live settings documents.
- A **pinned settings document** is bundled with the preset definitions needed to interpret it. It and its bundled definitions are migrated to the current setting definitions while preserving generation-time behavior as closely as possible, then resolved using those migrated definitions rather than current built-ins. A seed record's settings document is pinned (see [seed-data.md](seed-data.md)).

Resolving settings produces their **resolved values**: complete concrete values organized by the same scopes, without preset references. A **tweak** assigns a value differing from the selected presets' baseline. Internal UI stores and Rust types can use another representation where useful; they do not determine the persisted JSON shape.

## Format

A settings document has these fields. Field names are provisional:

| Field | Contents |
| --- | --- |
| `format_version` | Version of the persisted settings format |
| `preset` | The built-in root preset supplying the baseline |
| setting keys | Root-scope values |
| scope IDs | Nested objects containing the settings for those scopes |

The document itself represents root scope. A child scope such as `skill`, `item_pool`, or `starting_state` is a nested object; it can select its own built-in `preset` and contain values, family objects, and further child scopes. A value is a JSON scalar (boolean, number, or string) at its setting's [path](#identifiers); any object is a scope or family, never a value. Documents are sparse: only tweaks are present. Shared setting definitions identify each setting's scope and therefore its location in the document. UI section names do not determine that path.

There is one preset selection per scope object. Its ID is local to that scope: root `preset: "default"` selects the root default, while `preset: "hard"` inside `skill` selects the skill preset. The selected preset supplies a baseline, child scope objects customize their respective parts, and explicit values override the applicable baseline. JSON property order has no effect. A selected child preset replaces the values throughout that child scope, so values from a previously inherited preset cannot linger. A child object without a preset selection retains its inherited baseline and applies only its explicit changes.

User-defined presets remain root-only and based directly on a built-in root preset. A nested `skill` object is part of the root settings document, not a separately saved user preset. Built-in preset definitions use the same scoped settings objects. Metadata names such as `format_version` and `preset`, and child scope IDs, cannot also identify a setting at the same object level.

## Example settings

```json
{
  "format_version": 1,
  "preset": "default",
  "skill": {
    "preset": "hard",
    "tech": {
      "5": false
    }
  },
  "starting_state": {
    "equipment": { "fire_rod": true },
    "resources": { "heart_capacity": 2 }
  },
  "map_layout": "vanilla"
}
```

This starts with the default root preset, applies Hard skill assumptions, then selects vanilla map layout disables bomb boosts, and starts with the Fire Rod and two hearts. The default item pool deducts the starting Fire Rod from placement. The preset definitions come from the separate bundle, not this settings object. For live settings, the untouched values follow current built-ins; a seed's pinned settings use its migrated captured definitions. Documents carry no informational names; displays take names from the shared data.

## Identifiers

A setting is identified by its path: the object keys leading to it, in the shape of the resolved [default preset](../../data/presets/generation/root/default.json) and its selected child presets, including the [default item pool](../../data/presets/generation/item_pool/default.json), such as `map_layout` or `skill` › `tech` › `5`. Paths follow semantic scopes, never the UI layout. Keys are lowercase `snake_case`; where a Rust field or enum corresponds to a setting or option token, its Serde name follows the key, not the reverse. Option tokens are scoped to their setting, and built-in preset IDs to their scope.

Settings generated from data are members of a family object, keyed by stable IDs written as decimal strings where numeric:

| Family | Path | Member identity |
| --- | --- | --- |
| Technique | `skill` › `tech` › `<tech_id>` | Upstream tech ID |
| Proficiency | `skill` › `proficiencies` › `<tech_id>` | Upstream tech ID |
| Notable | `skill` › `notables` › `<world>` › `<room>` › `<notable_id>` | Upstream room identity and room-scoped notable ID |
| Item count | `item_pool` › `items` › `<item>` | Item token from [`data/items.json`](../../data/items.json) |

`starting_state` contains `equipment` and `resources` objects. Equipment uses booleans for unique items and named tiers for progressive items and the Ocarina. Resources contain whole-heart capacity, bomb/arrow capacities, starting bomb/arrow ammunition, rupees, and the contents of each of the four bottle slots `bottle_1`–`bottle_4` (`none`, `empty`, `red_potion`, `green_potion`, `blue_potion`, `fairy`, `bee`, or `gold_bee`). `item_pool` contains `items` pickup counts, `deduct_starting_items` (equipment only), and `bomb_upgrade_size`/`arrow_upgrade_size` (1–99 each). Resolved values retain these authored choices; generation derives the remaining pickup pool and fills unused locations with Nothing.

Identifiers are permanent: a key, option token, or built-in preset ID keeps one meaning while current and is never reused after removal, including identifiers that only appeared on the dev site. A retired identifier found in any document can therefore only carry its old meaning (see [vocabulary rules](#vocabulary-rules)).

Technique, notable, and room identities come from [z3-json-data](https://github.com/vg-json-data/z3-json-data), which the randomizer relies on to keep them stable and never reuse them. When upstream changes one anyway, the randomizer retires the old identifier with a vocabulary rule like any other.

## Versioning and migration

The settings format has its own format version, recorded in every document, including browser drafts, files, and share links. A commit changing the document structure increments it and adds a structural migration step, such as `migrate_v2_to_v3`. Vocabulary changes, such as renamed keys, option tokens, or built-in presets, do not change the format version; they are handled by [vocabulary rules](#vocabulary-rules).

Moving a setting to another semantic scope changes its persisted path and will require an explicit migration. Such moves are expected to be rare. The operation format, conflict handling, and other cross-scope migration details are deferred until a concrete need arises. Moving a control between UI sections without changing its scope does not require a settings migration.

### Vocabulary rules

Vocabulary changes are migrated without versioning, by a rule attached to each retired identifier. Since identifiers are never reused, a rule can run whenever its identifier is found, in live documents, pinned documents and their bundled presets, and seeds' resolved values. A rule is one of:

- **Renamed:** the identifier is replaced by another, keeping the value.
- **Value map:** old values map to new ones, optionally under a new path, e.g. a toggle becoming a choice: `{ "false": "off", "true": "full" }`.
- **Converted:** a small function replaces the entry with current ones, for splits, merges, and changes of meaning.
- **Dropped:** the setting has no equivalent; lenient readers report it as removed rather than unreadable.

Rules are applied until no retired identifiers remain. A setting's value kind may change in place only when old values are distinguishable from new ones by JSON type (such as a toggle becoming a choice), with a value-map rule for the old type; any other change of meaning, such as rescaling a count, needs a new key. Adding an option token needs no rule. An identifier that never appeared in a release may be retired without a rule, relying on [lenient loading](#lenient-loading).

The current and retired vocabulary is committed, and CI checks the shared setting definitions and presets against it, so that a release cannot drop an identifier without a rule.

### Behavior-preserving migration

Structural migrations and [vocabulary rules](#vocabulary-rules) bring persisted settings documents and values into the current settings representation while preserving their behavior as closely as possible. They do not adopt new built-in defaults as a side effect. Migration always precedes validation against current definitions: sanitizing first would discard obsolete identifiers and values before they could be translated.

Preserve the distinction between inherited values and explicit tweaks. Do not resolve a live settings document into a historical value map and turn all those values into tweaks. When a pinned bundle needs a compatibility value for a newly introduced setting, place that historical baseline in its bundled definitions or other concrete snapshot data rather than inventing a user tweak. The exact completion representation remains to be specified.

Pinned settings documents receive structural and semantic migration, including the setting/value entries inside their bundled presets. The backend can therefore return the same current settings representation as for live settings. Captured preset definitions remain authoritative for pinned references even when a preset with the same ID is retuned or removed from the current preset bundle. Migrate references and their bundled targets consistently; do not replace a captured target with a current built-in merely because its identifier is retired.

The original pinned settings document, bundled presets, and resolved values remain unchanged in storage; migration produces an upgraded copy. Migrations that cannot preserve meaning exactly report the translation or limitation. Pinned input remains subject to [strict reading](seed-data.md#reading-seed-records), rather than the live editor's lenient fallback.

### Bringing live settings up to date

A live settings document is processed in this order:

1. Apply the settings format's structural migrations, from the settings document's recorded format version to the current one.
2. Apply vocabulary rules until no retired identifiers remain.
3. Apply the separate live-settings update: resolve preset applications against current built-ins, allowing inherited values and newly introduced settings to follow their current defaults.
4. Validate the complete resolved candidate against the JSON schemas and semantic rules, with optional [recovery](#lenient-loading) for what remains unreadable. Strict consumers reject invalid input.
5. Resolve and normalize tweaks against the current selected baseline (see below).

The update in step 3 preserves explicit tweaks; it does not replace their values with new defaults. Changes of value kind and identifiers have already been handled by migration.

A pinned settings document omits the live-settings update. It resolves against its migrated bundle and compares tweaks against that bundle's baseline. No current built-in contents are substituted.

### Normalization

A tweak equal to its selected baseline is redundant and is removed. For live settings, compare after adopting current built-in presets/defaults. For pinned settings, compare against the migrated captured baseline so normalization cannot change the frozen behavior. The browser stores the normalized document in place of the original, as described below.

This does not replay every intermediate release's defaults. Enforcing a rule that a tweak disappears if it matched any intervening baseline would require additional historical baseline facts or explicit change rules. Whether that stronger normalization guarantee is worth retaining is an open question, rather than a dependency on storing complete historical shared-data exports.

## Lenient loading

Recovery allows a live settings document to be loaded without discarding it when some submitted entries cannot be interpreted. This can happen when a dev-site change omitted a vocabulary rule, when a rule is missing by mistake, or when the settings document comes from a newer deployment, such as a dev-site share link opened on the release site. Loading a live settings document is therefore lenient:

- **Unreadable entries revert to their baseline.** An entry with an unknown key, an invalid option token, an out-of-range value, or a reference to an unknown built-in preset is dropped. The affected setting takes the value given by the settings document's presets, not the raw setting default. An unknown preset reference falls back to its scope's default preset, or to the default root preset.
- **Newer format versions are loaded leniently too.** A settings document from a newer format version is read with the current code's understanding, and anything that cannot be interpreted is dropped in the same way.
- **Recovery always yields a loadable document.** If dropping and restoring individual entries does not make a document readable, the changes in each affected child scope are dropped, keeping its preset selection, and then all changes, keeping the root preset; each step is reported. A document that still cannot be processed, or stored text that is not valid JSON, indicates a bug: the editor names it and lets the player copy it, then discards it.
- **Fallback is never silent.** Loading returns the usable settings document together with a list of what was dropped or replaced, and the editor shows that list. Entries dropped by an explicit "dropped" rule are reported as removed settings rather than as unreadable input.
- **The cleaned form replaces the original.** A stored draft or preset is overwritten by its cleaned form as soon as it is loaded, so its notices are shown once. Input dropped because of a missing rule is not recovered if the rule is added later.
- **One processing implementation, explicit recovery policy.** The generation endpoint is strict. The CLI is strict by default and enables recovery with `--recover-settings` alongside `--settings`. Browser loading and import use recovery through the backend's [settings processing endpoint](backend-api.md#settings-processing). The browser stores each document with the [settings revision](settings-data.md#repository-and-build) it was processed under, and loads documents stored under the current revision without processing them again; saved presets are processed when first selected. A page whose bundled data differs from the server's asks the player to reload rather than use the results. The pinned settings document stored with the seed is the cleaned document actually used.
- **Validity across settings is not repaired.** If the resolved values violate a cross-setting constraint, such as an item pool that does not fit the available locations, the editor or generation reports it and generation is blocked; values are never adjusted automatically to make the settings document valid.

In a release, a fallback caused by a missing vocabulary rule indicates a bug, since the vocabulary check should have prevented it. On the dev site it is expected.

Validation uses separately maintained [generation](../../schema/settings/generation.json), [skill](../../schema/settings/skill.json), and [customization](../../schema/settings/customization.json) JSON schemas for complete resolved values. Fixed objects reject unexpected properties; dynamic maps validate values generically while Rust checks their keys, permitted proficiency levels, and item minimums against prepared data. Missing document entries inherit recursively from the selected baseline. A required field still missing after resolution is a blocking preparation inconsistency in either mode.

Strict mode reports invalid values, unexpected keys, unknown dynamic IDs, and unknown presets as blocking errors. Recovery selects fallback presets before resolution, removes unexpected submitted entries, and restores invalid known entries or malformed containers from the selected baseline while preserving siblings. It applies repairs in one batch and revalidates once, then deserializes into Rust settings once. Unattributable findings and remaining failures stay blocking. Reports retain string-segment paths, reasons, and replacement values where applicable. An invalid value beneath an unknown key produces only the removal finding.

Whole-valued JSON numbers such as `10`, `10.0`, and `1e1` are accepted equivalently and normalized to integers without a recovery notice. Fractions are not rounded, bounds are not clamped, and strings are not converted. The processor retains the original input independently of this working copy. Format version 1 is the only authored format and has no historical migrations; unsupported versions are blocking in strict mode and interpreted through recognized fields in recovery mode.

Pinned settings documents are read strictly, as part of the [seed record](seed-data.md#reading-seed-records), without live recovery or substitution of current built-in presets.

## Generation requests

The frontend submits the settings document through the [generation endpoint](backend-api.md#generation). The backend migrates and updates it through the same live-settings path, including when the frontend bundled older data. The resulting seed records the settings actually used. The frontend shows any upgrade notices and lets the user inspect the authoritative result through the normal seed view, using its captured presets rather than the frontend's bundled built-ins.

## Share links

To be written: URL scheme and encoding of settings documents in share links.

## Customization presets

To be written: saved presentation/control preferences and their relationship to the generation settings format. Temporary per-seed gameplay adjustments are proposed in the [customization plan](../../plans/customization.md).

## Open questions

- Whether live normalization must replay intermediate baseline changes, and if so, which compact change rules provide the necessary history.
- Where the committed vocabulary lives and how converting rules are referenced from it.
