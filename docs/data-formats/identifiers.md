# Identifiers

Status: draft. This document defines the identifiers that persisted documents use to refer to settings, options, preset scopes, and built-in presets. Every other permanent format depends on these, so their rules are the strictest.

## General rules

- **Identity is the identifier within its namespace.** Display text and position in the frontend layout never determine identity.
- **Independent of display layout.** No identifier encodes the section, group, or card under which a setting is displayed. A setting's persisted path follows its semantic scope, not its display position. UI-only reorganization never changes an identifier or settings path.
- **Permanent.** An identifier keeps its meaning for as long as it is current. Changing what a setting or option means requires a new identifier, except for a change of value kind whose old values are distinguishable from its new ones, or a return to an earlier kind with its earlier meaning (see [Changes of value kind](#changes-of-value-kind)).
- **Never reused after removal.** A removed setting key, option token, or preset ID is retired permanently, including identifiers that only ever appeared on the dev site. Vocabulary rules depend on this: a retired identifier found in any document can only carry its old meaning.
- **Lowercase ASCII.** Identifiers consist of segments of `[a-z0-9_]`, separated by `.` where a structured form is defined below. Case never distinguishes identifiers.
- **No informational names in documents.** Values are plain scalars, so documents carry no names beside them. Displays take names from trusted shared definitions.

## Kinds of identifiers

### Ordinary setting keys

A setting individually authored in the randomizer uses a single `snake_case` segment, such as `map_layout`. Where a Rust field corresponds to the setting, the key matches its Serde field name, but the key is the permanent identity and the Rust field must follow it, not the reverse.

A setting is identified by its path: the keys leading to it through nested objects, in the shape of [`data/settings.json`](../../data/settings.json). Paths follow semantic scopes. For example, technique 5 is at `skill` › `tech` › `5`, while the root-scope `map_layout` is at the document root. Paths are never joined into a single string key; each segment is its own JSON object key. A change of scope requires an explicit path migration even if the setting key itself stays the same. Cross-scope migration design is deferred until needed; no move-template language is defined here.

### Data-driven families

Settings generated from data, such as one setting per technique, are members of a family: an object whose keys identify the members. A family is handled generically, so adding a member requires no code. The families are:

| Family | Path | Member identity | Value |
| --- | --- | --- | --- |
| Technique | `skill` › `tech` › `<tech_id>` | Upstream numeric tech ID | Enabled flag |
| Proficiency | `skill` › `proficiencies` › `<tech_id>` | Upstream numeric tech ID | Level |
| Notable | `skill` › `notables` › `<world>` › `<room>` › `<notable_id>` | Upstream room identity and room-scoped notable ID | Enabled flag |
| Item count | `item_pool` › `<item>` | Item token | Count |

For example, in JSON: `{"skill": {"tech": {"5": true}, "notables": {"underworld": {"60": {"1": false}}}}, "item_pool": {"fire_rod": 1}}`.

Numeric IDs are written as decimal strings, since they are object keys. The world segment is `overworld` or `underworld`, matching `SourceRoomId`. Item tokens are the `name` fields of [`data/items.json`](../../data/items.json): a permanent `snake_case` vocabulary owned by the randomizer, mapped there to upstream item names. The Rust `Item` enum's Serde representation follows these tokens.

Member identities are only meaningful within their family object, so they can never collide with ordinary setting keys or with members of another family.

### Option tokens

Each choice setting defines its own vocabulary of option tokens, such as `random` and `vanilla` for `map_layout`. Tokens are single `snake_case` segments scoped to their setting: the same token may appear under different settings with unrelated meanings. Where a Rust enum corresponds to the choice, its Serde variant names match the tokens.

### Preset scope IDs

Each preset scope has a single-segment ID, such as `skill` for Skill assumptions. The ID `root` is reserved for the root scope, represented by the settings document itself. Other scope IDs identify nested settings objects under their declared parent. Scope IDs are independent of display section IDs, even where a section displays exactly one scope. Root presets cover all settings; a non-root preset covers its own scope subtree.

### Built-in preset IDs

Built-in preset IDs are single segments local to their scope, such as `default` in root scope or `hard` in skill scope. The same ID can be used in different scopes without a prefix. A `preset` selection is looked up in the scope of its containing settings object and the applicable generation/customization context. The bundle preserves that context rather than combining presets into one global ID namespace. Shared data preparation obtains a preset's scope from its source directory.

Removing or merging a built-in preset requires a [rule](#retired-identifiers-and-rules) for live references to it. The rule carries the historical replacement or conversion facts it needs, without loading a complete historical shared-data export. Pinned settings documents retain their bundled definitions; removing a preset from the current preset bundle does not replace that captured definition.

### Section and group IDs

Frontend sections and groups may carry IDs for page navigation. These are not permanent identifiers and must not appear in persisted settings or seed bundles.

## Vocabulary changes

Settings are expected to evolve faster than any other data, so their vocabulary is migrated without versioning. Vocabulary changes, such as renaming or removing a setting key, option token, or built-in preset, do not change the format version of settings documents or seed records. They are handled by unversioned rules attached to the retired identifiers. Because identifiers are never reused, finding a retired identifier in a document is unambiguous: it can only carry its old meaning, regardless of when the document was written. Each rule can therefore run whenever that identifier is encountered in its applicable context. These rules preserve meaning as closely as possible; adopting current preset/default contents is a separate live-settings operation.

### Retired identifiers and rules

When an identifier is renamed or removed, it moves from the current vocabulary to the retired list in the [committed vocabulary](#vocabulary-check), together with its rule. A rule is one of:

- **Renamed:** the identifier is replaced by another, such as `old_key` becoming `new_key`, or option token `a` of `some_key` becoming `b`. The value carries over unchanged.
- **Value map:** a declarative mapping from old values to new ones, optionally under a new key. This covers the common case of a toggle becoming a choice, as well as renaming or merging option tokens. For example:

  ```json
  { "key": "foo", "from_kind": "toggle", "values": { "false": "off", "true": "full" } }
  ```

  The mapping is usually exact: the old `true` meant one specific behavior, which becomes one of the new options, and the added options have no old equivalent. If an old value's behavior is split between several new options, the rule maps it to the closest one, and lenient readers report the conversion.
- **Converted:** a small function replaces the retired identifier and its value with current entries. This covers splitting or merging settings, value conversions for a change in meaning, and translating a removed live preset reference using explicit historical facts carried by the rule. Captured preset contents come from the seed's bundle when migrating a pinned settings document, rather than from historical shared-data exports.
- **Dropped:** the identifier has no equivalent. Its entries are removed, and lenient readers report it as a removed setting rather than as unreadable input.

Each retired identifier, and each retired kind of a key, has exactly one rule. A rule's output may itself contain identifiers retired later; rules are applied until none remain. If `a` was renamed to `b` and later `b` to `c`, an old document applies both rules in turn. Since identifiers are never reused, cycles cannot occur.

Retired identifiers also let readers distinguish a deliberately removed setting, which can produce a specific message, from input that is simply unrecognized.

### Where rules apply

Setting/value rules apply wherever current identifiers are expected: in live settings documents, inside pinned settings documents and their bundled preset definitions, and on copies of old seeds' resolved values before display or patching. See [settings document migration](settings.md#behavior-preserving-migration) and [seed settings](seed-data.md#settings). Keep explicit tweaks distinct from inherited/baseline values so migration does not accidentally pin defaults into live settings documents.

Preset-reference rules preserve the distinction between current built-in references and captured definitions. A retired live reference needs a supported replacement, while a pinned reference continues to use its migrated bundled target. If a migration renames a bundled preset ID, it updates the registry and references together. It must not discard the captured values in favor of the current preset with a matching ID. All migration operates on copies of seed data; original stored records remain unchanged.

### Changes of value kind

A rule keyed only by identifier cannot detect a change in the meaning of a key that still exists when old and new documents look the same. The permanent identity of a value is therefore its key together with its JSON type, and for a choice, its option token: each of these has one meaning forever.

- **In place, when old values are distinguishable.** A key may change its value kind in place only when every old value is distinguishable from every new value by JSON type, such as a toggle (`true`/`false`) becoming a choice (strings). The old kind is retired in the key's kind history with a value-map rule keyed by the key and its old kind, so it fires only on values of the old type, and applying it again changes nothing.
- **Otherwise, a new key.** Any other change in meaning, such as rescaling a count, requires retiring the key and introducing a new one, with a rule converting old values.
- **Returning to an earlier kind.** A key may return to an earlier kind only if values of that kind regain exactly their earlier meaning. For example, a toggle that became a choice may become a toggle again if `true` and `false` mean what they originally meant: documents from the first toggle period are then valid as they are, and a value-map rule converts documents from the choice period. The kind history marks the earlier kind as restored and its rule is deleted, since keeping it would make rule application loop. Marking a kind as restored is an explicit statement that its values mean exactly what they meant before; the check cannot verify meaning, so this is a deliberate decision recorded in the vocabulary. If the earlier kind would return with a different meaning, a new key is required instead.
- **Adding an option is not a migration.** Adding a token to an existing choice leaves existing tokens' meanings unchanged, so no rule is needed.

### Dev-only identifiers

Rules are written in the commit making the change, so documents from earlier dev-site deployments are covered without separate work. An identifier that has never appeared in a release, such as a key introduced since the last release, may be renamed or removed without a rule when one is not worth writing. It is still listed as retired, without a rule, so that it is never reused. Live settings documents affected by the omission are handled by [lenient loading](settings.md#lenient-loading).

Bundled dev-site seeds also need migration to current setting definitions. If a required rule was omitted, strict reading fails until it is supplied. Whether to permit dev-only omissions while promising indefinite compatibility remains an open question.

### Vocabulary check

The vocabulary is committed: every current key with its value kind, kind history, and option tokens, every current built-in preset ID with its settings context and scope, every retired identifier with its rule, and whether each identifier has appeared in a release. CI compares the prepared shared setting definitions and preset bundle against it and fails when:

- an identifier that has appeared in a release is neither current nor retired with a rule;
- a current key changes its value kind without a value-map rule for its old kind; or
- a key returns to an earlier kind without marking it as restored.

It reports, without failing, the same conditions for identifiers that have appeared only on the dev site. In a release, a lenient-loading fallback caused by a missing rule therefore indicates a bug.

## Upstream identifiers

Technique, proficiency, and notable identities come from [z3-json-data](https://github.com/vg-json-data/z3-json-data), which the randomizer does not control. The randomizer depends on the following properties of upstream data:

- Tech IDs are stable and never reused.
- Notable IDs are stable and never reused within a room.
- Room identities are stable.

These assumptions can be violated by upstream changes, such as a deleted technique, a merged pair of notables, or a restructured room. Because such changes arrive through data rather than randomizer code, they could rename or remove keys without anyone writing a rule. The [vocabulary check](#vocabulary-check) detects them in the same way as randomizer-side changes. In addition, shared data preparation fails when an upstream member lacks the randomizer-side classification it needs, such as a difficulty assignment, rather than silently defaulting it.

When upstream changes an identity, the randomizer retires the old key with a rule translating it to the new key, or with a "dropped" rule if there is no equivalent.

## Open questions

- Whether techniques and proficiencies should remain separate families. Upstream represents both as techs, distinguished by whether they have levels. Separate families mean that if upstream converts a tech into a proficiency, its key changes along with its value kind, which makes the change visible and requires a vocabulary rule.
- Whether item counts are the only item family needed (for example, starting items).
- Where the committed vocabulary lives, how it is updated when a commit intentionally adds or retires identifiers, and how converting rules (functions) are referenced from it.
