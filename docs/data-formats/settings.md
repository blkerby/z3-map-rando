# Settings

Status: draft. A settings document organizes values and preset selections hierarchically by scope. Drafts, saved user presets, files, share links, and generated seeds use this format. Its source revision identifies the shared setting definitions and preset data used by the corresponding build (see [shared settings data](settings-data.md)).

## Live and pinned settings

Every settings document uses the same format but is used in one of two modes:

- A **live settings document** is migrated to the current setting definitions while preserving the meaning of its entries. A separate live-settings update then adopts current built-in presets and defaults for untweaked values. Drafts, saved user presets, files, and share links are live settings documents.
- A **pinned settings document** is bundled with the preset definitions needed to interpret it. It and its bundled definitions are migrated to the current setting definitions while preserving generation-time behavior as closely as possible, then resolved using those migrated definitions rather than current built-ins. A seed record's settings document is pinned (see [seed-data.md](seed-data.md)); its recorded source settings revision remains provenance, not a dependency on a historical shared-data export.

Resolving settings produces their **resolved values**: complete concrete values organized by the same scopes, without preset references. A **tweak** assigns a value differing from the selected presets' baseline. Internal UI stores and Rust types can use another representation where useful; they do not determine the persisted JSON shape.

## Format

A settings document has these fields. Field names are provisional:

| Field | Contents |
| --- | --- |
| `format_version` | Version of the persisted settings format |
| `revision` | Shared settings data revision against which these choices were authored or saved |
| `preset` | The built-in root preset supplying the baseline |
| setting keys | Root-scope values |
| scope IDs | Nested objects containing the settings for those scopes |

The document itself represents root scope. A child scope such as `skill` is a nested object; it can select its own built-in `preset` and contain values, family objects, and further child scopes. A value is a JSON scalar (boolean, number, or string) at its setting's [path](identifiers.md#ordinary-setting-keys); any object is a scope or family, never a value. Documents are sparse: only tweaks are present. Shared setting definitions identify each setting's scope and therefore its location in the document. UI section names do not determine that path.

There is one preset selection per scope object. Its ID is local to that scope: root `preset: "default"` selects the root default, while `preset: "hard"` inside `skill` selects the skill preset. The selected preset supplies a baseline, child scope objects customize their respective parts, and explicit values override the applicable baseline. JSON property order has no effect. A selected child preset replaces the values throughout that child scope, so values from a previously inherited preset cannot linger. A child object without a preset selection retains its inherited baseline and applies only its explicit changes.

User-defined presets remain root-only and based directly on a built-in root preset. A nested `skill` object is part of the root settings document, not a separately saved user preset. Built-in preset definitions use the same scoped settings objects. Metadata names such as `format_version`, `revision`, and `preset`, and child scope IDs, cannot also identify a setting at the same object level.

## Example settings

```json
{
  "format_version": 1,
  "revision": "<settings data revision>",
  "preset": "default",
  "skill": {
    "preset": "hard",
    "tech": {
      "5": false
    }
  },
  "map_layout": "vanilla"
}
```

This starts with the default root preset, applies Hard skill assumptions, then selects vanilla map layout and disables bomb boosts. The preset definitions come from the separate bundle, not this settings object. For live settings, the untouched values follow current built-ins; a seed's pinned settings use its migrated captured definitions. The revision above is a placeholder for the shared settings data revision. Documents carry no informational names; displays take names from the shared data.

## Versioning and migration

The settings format has its own format version, recorded in every document, including browser drafts, files, and share links. A commit changing the document structure increments it and adds a structural migration step, such as `migrate_v2_to_v3`. Vocabulary changes, such as renamed keys, option tokens, or built-in presets, do not change the format version; they are handled by [vocabulary rules](identifiers.md#vocabulary-changes).

Moving a setting to another semantic scope changes its persisted path and will require an explicit migration. Such moves are expected to be rare. The operation format, conflict handling, and other cross-scope migration details are deferred until a concrete need arises. Moving a control between UI sections without changing its scope does not require a settings migration.

### Behavior-preserving migration

Structural migrations and [vocabulary rules](identifiers.md#vocabulary-changes) bring persisted settings documents and values into the current settings representation while preserving their behavior as closely as possible. They do not adopt new built-in defaults as a side effect. Migration always precedes validation against current definitions: sanitizing first would discard obsolete identifiers and values before they could be translated.

Preserve the distinction between inherited values and explicit tweaks. Do not resolve a live settings document into a historical value map and turn all those values into tweaks. When a pinned bundle needs a compatibility value for a newly introduced setting, place that historical baseline in its bundled definitions or other concrete snapshot data rather than inventing a user tweak. The exact completion representation remains to be specified.

Pinned settings documents receive structural and semantic migration, including the setting/value entries inside their bundled presets. The backend can therefore return the same current settings representation as for live settings. Captured preset definitions remain authoritative for pinned references even when a preset with the same ID is retuned or removed from the current preset bundle. Migrate references and their bundled targets consistently; do not replace a captured target with a current built-in merely because its identifier is retired.

The original pinned settings document, bundled presets, and resolved values remain unchanged in storage; migration produces an upgraded copy. Migrations that cannot preserve meaning exactly report the translation or limitation. Pinned input remains subject to [strict reading](seed-data.md#reading-seed-records), rather than the live editor's lenient fallback.

### Bringing live settings up to date

A live settings document is processed in this order:

1. Apply the settings format's structural migrations, from the settings document's recorded format version to the current one.
2. Apply vocabulary rules until no retired identifiers remain.
3. Apply the separate live-settings update: resolve preset applications against current built-ins, allowing inherited values and newly introduced settings to follow their current defaults.
4. Validate against current setting definitions, with [lenient fallback](#lenient-loading) for what remains unreadable.
5. Resolve and normalize tweaks against the current selected baseline (see below).

The update in step 3 preserves explicit tweaks; it does not replace their values with new defaults. Changes of value kind and identifiers have already been handled by migration. The completed result records the current interpretation separately from the source revision used as migration provenance.

A pinned settings document omits the live-settings update. It resolves against its migrated bundle and compares tweaks against that bundle's baseline. No current built-in contents are substituted.

### Normalization

A tweak equal to its selected baseline is redundant and is removed. For live settings, compare after adopting current built-in presets/defaults. For pinned settings, compare against the migrated captured baseline so normalization cannot change the frozen behavior. The original stored document is retained until an explicit save, as described below.

This does not replay every intermediate release's defaults. Enforcing a rule that a tweak disappears if it matched any intervening baseline would require additional historical baseline facts or explicit change rules; a source revision alone cannot reconstruct them. Whether that stronger normalization guarantee is worth retaining is an open question, rather than a dependency on storing complete historical shared-data exports.

## Lenient loading

A user's live settings document should never block generation or need to be discarded because part of it cannot be interpreted. This can happen when a dev-site change omitted a vocabulary rule, when a rule is missing by mistake, or when the settings document comes from a newer deployment, such as a dev-site share link opened on the release site. Loading a live settings document is therefore lenient:

- **Unreadable entries revert to their baseline.** An entry with an unknown key, an invalid option token, an out-of-range value, or a reference to an unknown built-in preset is dropped. The affected setting takes the value given by the settings document's presets, not the raw setting default. An unknown preset reference falls back to its scope's default preset, or to the default root preset.
- **Newer format versions are loaded leniently too.** A settings document from a newer format version is read with the current code's understanding, and anything that cannot be interpreted is dropped in the same way. If the settings document cannot be parsed at all, the default settings document is used.
- **Fallback is never silent.** Loading returns the usable settings document together with a list of what was dropped or replaced, and the editor shows that list. Entries dropped by an explicit "dropped" rule are reported as removed settings rather than as unreadable input.
- **The original is kept until the user saves.** The stored draft or preset is not overwritten by its cleaned form until the user explicitly saves. If a rule is added later, the untouched original can still be migrated properly.
- **One load path.** The backend applies the same lenient loading to generation requests. The pinned settings document stored with the seed is the cleaned settings document actually used.
- **Validity across settings is not repaired.** If the resolved values violate a cross-setting constraint, such as an item pool that does not fit the available locations, the editor reports it and generation is blocked; values are never adjusted automatically to make the settings document valid.

In a release, a fallback caused by a missing vocabulary rule indicates a bug, since the vocabulary check should have prevented it. On the dev site it is expected.

Pinned settings documents are read strictly, as part of the [seed record](seed-data.md#reading-seed-records).

## Generation requests

The frontend submits the settings document through the [generation endpoint](backend-api.md#generation). The backend migrates and updates it through the same live-settings path, including when the frontend bundled older data. The resulting seed records the settings actually used. The frontend shows any upgrade notices and lets the user inspect the authoritative result through the normal seed view, using its captured presets rather than the frontend's bundled built-ins.

## Share links

To be written: URL scheme and encoding of settings documents in share links.

## Customization presets

To be written: saved presentation/control preferences, their relationship to the generation settings format, and the temporary per-seed Gameplay adjustment draft.

## Open questions

- Where migrations are implemented. The proposal is to implement structural migrations and vocabulary rules only in Rust, exposed to the frontend through a backend endpoint or WebAssembly, so they are never maintained in two languages.
- Whether live normalization must replay intermediate baseline changes, and if so, which compact change rules provide the necessary history.
