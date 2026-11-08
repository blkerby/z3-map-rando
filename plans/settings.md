# Settings and presets

## Status and goals

This document records the settings model agreed during design discussion. JSON field names and examples are provisional. The UI and Rust boundary approaches below are proposals for further discussion, not implementation decisions. This plan supersedes the more elaborate user-preset hierarchy proposed in the frontend's `docs/presets-plan.md`.

Use a common JSON interpretation for browser storage, import/export, and generation requests. Every released version should accept settings from any previous released version through explicit migration. The primary behavior is to follow evolving built-in presets and defaults while retaining the user's differences. Reproducing historical generator behavior from an editable settings document is not the primary goal.

Generated seeds have a different requirement: their settings must remain inspectable as they were interpreted at generation time. Preserve those concrete settings and enough frozen preset definitions to explain them. Public seed records must not expose private preset names or other arbitrary user metadata. No account system is planned, so creators and other viewers receive the same public representation.

## Preset semantics

- User-defined presets exist only at root scope.
- Every user preset is based directly on one built-in root preset. User presets cannot inherit from other user presets, and parentless user presets are deferred.
- A user preset can choose built-in presets for sections such as Logic and override individual settings anywhere in the configuration.
- Built-in presets retain their section hierarchy and composition. Their definitions and the master description drive resolution; particular presets such as Hard are not hard-coded in the resolver.
- Saving a new preset from an existing user preset copies its built-in base, effective section selections, and tweaks. It creates an independently editable sibling, without a reference to the original user preset.
- Inherited values follow current built-in definitions. New settings inherit values from the applicable current presets and defaults.
- An explicit tweak represents a difference from its current inherited baseline. If migration or a preset update makes it equal to that baseline, remove it. There is no hidden per-setting pinning behavior.
- Compare tweaks against the baseline after applying section preset selections, rather than against raw schema defaults.

The deliberate limitation is that users cannot independently reuse private Logic presets or share a private baseline across several configurations. Repeated personal tweaks must be maintained separately. Built-in section presets provide the supported composition mechanism.

The distinction between a saved tweak and an unsaved edit remains useful. Exact save, reset, section-selection, and other UI operations are deferred; they should not introduce additional inheritance semantics implicitly.

## Setting identity and JSON values

Store settings as a flat map of stable identifiers to entries. Each entry has a `value` and may have informational metadata such as `name`:

```json
{
  "mapLayout": {
    "value": "randomized"
  },
  "tech.5": {
    "name": "canBombBoost",
    "value": true
  }
}
```

The identifier determines identity. `name` is ignored when interpreting values and can be omitted. Regenerate informational names from trusted definitions on export and when constructing a public seed record. A historical name can remain readable without becoming a permanent operational identifier.

Use stable authored IDs for techniques, proficiencies, and room-scoped notables. Ordinary settings can have descriptive stable identifiers. The exact key vocabulary, including item identities and prefixes, remains to be settled. Dotted keys denote identities or semantic namespaces, not paths that must track the UI's current layout.

Use the same entry shape for all serialized values rather than accepting both scalars and wrapped entries. Internal UI state can still store scalars. Labels, grouping, control types, option descriptions, section membership, and validation information remain in the master description; do not reproduce the section tree in settings documents.

Reserved keys identify composition explicitly. A provisional convention is `$base` for a built-in root base, `$sections` for built-in selections keyed by section ID, and `{ "$ref": "..." }` for a reference. Ordinary string values never become references implicitly. Reference identifiers are stable; their display names are informational.

## Definition ownership and assembled masters

Define randomizer-owned settings in the randomizer project, rather than maintaining frontend-specific copies. An authored master JSON, or generation/customization documents using the same format, should own stable keys, value kinds, defaults, limits, option tokens and labels, help text, layout, and built-in presets. Exact filenames and whether these are two files or two roots of one file remain open.

Populate techniques, proficiency tiers, and notables automatically from `z3-json-data`, with stable IDs as the join keys. Keep randomizer-owned metadata such as difficulty assignments and presentation overrides in one local definition/overlay. Do not maintain a second authoritative copy of upstream names, room identities, or tier values. Newly introduced techniques should require no new Rust field, widget binding, or per-tech conversion branch; randomizer-specific classification may still require authoring.

The assembled master is a derived artifact owned and served by the randomizer backend. The frontend requests it at runtime instead of importing a maintained or regenerated frontend JSON copy. Its construction can run during asset/catalog preparation or backend initialization from packaged local data; ordinary application startup should not require fetching upstream data. Reuse the existing upstream readers where suitable, without requiring the UI to consume the full logic catalog or requiring build tooling to run in the browser.

```text
randomizer-owned definitions + upstream identities/tiers + local classifications
  -> assembled generation and customization masters owned by the backend
  -> master JSON API -> frontend controls
  -> backend defaults, presets, validation, and settings interpretation
```

Today the frontend master was built by a one-time bootstrap script and is edited directly. The randomizer also has `data/tech_data.json` and `data/notable_data.json` with difficulty assignments. The logic catalog already retains technique IDs, names and tiers, and room-scoped notable IDs and names; it does not retain all UI descriptions and classifications. The new assembly path should replace copied definitions rather than add another independently maintained version. CLI default settings should be derived from the same definitions and default preset instead of keeping an independent default-values document.

Defining a setting once can automate its storage, rendering, resolution, and conversion, but implementing its actual generation or patching behavior still requires consumer code. Historical migrations also remain explicit when meaning changes. Do not confuse those semantic changes with plumbing.

### Master API and development

Serve the assembled generation/customization description through an endpoint such as `GET /api/master`. Serialize or otherwise prepare the representation once per backend data revision rather than rebuilding it on every request. Use an ETag derived from the served master content and an initial cache policy such as `Cache-Control: no-cache`: caches may store the response but must validate it before reuse. An unchanged conditional request can return `304 Not Modified`. Prefer ordinary HTTP caching over a separate persistent application cache initially. This is the standard validation model described by [RFC 9111](https://www.rfc-editor.org/rfc/rfc9111.html#section-4.3).

Browser caching and server-side frontend fetch caching are separate concerns. The current SvelteKit page uses universal load, which may execute on the server as well as in the browser. Use the fetch supplied by the load context and settle API routing/proxying accordingly. Do not assume that a browser's cached master automatically serves a server-side load request. See [SvelteKit's fetch documentation](https://svelte.dev/docs/kit/load#Making-fetch-requests).

A running backend is an acceptable requirement for normal interactive frontend development and integration work. Frontend compilation, static checks, and isolated component work need not all require it. If isolated previews later need a master fixture, derive that fixture from the backend; do not establish another hand-maintained settings source or a mandatory frontend bootstrap workflow. Error presentation for an unavailable master is ordinary loading behavior, not a reason to invent an offline subsystem now.

HTTP cache validation does not settle the separate question of an editor remaining open across a backend release. Retain a master revision in requests or an equivalent mechanism so accepted generation inputs and the resulting seed display can be reconciled deliberately.

## Live settings and frozen seed bundles

An editable settings document records a version, the release or master revision it was saved against, its built-in root base, section selections, and setting tweaks. Built-in references resolve against the current definitions after migration. Do not embed historical built-in contents in ordinary saved settings merely to preserve history.

A frozen seed bundle records one anonymous root recipe and a registry of the built-in definitions needed to resolve it. References resolve exclusively within that bundle. Capture defaults as concrete values in the frozen built-in definitions and make otherwise implicit child selections explicit. Include required hidden or fixed generation values as well. This makes the effective values self-contained without embedding the master's section tree.

The following miniature frozen bundle has only two settings. Real bundles include every definition and value needed to resolve the actual generation inputs. Envelope names and the distinction between live and frozen document contexts remain provisional.

```json
{
  "version": 1,
  "root": {
    "$base": {
      "$ref": "builtin/root/default"
    },
    "$sections": {
      "logic": {
        "$ref": "builtin/logic/hard"
      }
    },
    "mapLayout": {
      "value": "vanilla"
    }
  },
  "presets": {
    "builtin/root/default": {
      "$sections": {
        "logic": {
          "$ref": "builtin/logic/basic"
        }
      },
      "mapLayout": {
        "value": "randomized"
      }
    },
    "builtin/logic/basic": {
      "tech.5": {
        "name": "canBombBoost",
        "value": false
      }
    },
    "builtin/logic/hard": {
      "tech.5": {
        "name": "canBombBoost",
        "value": true
      }
    }
  }
}
```

Generic resolution combines inherited section selections and explicit values, resolves the selected children, and applies explicit values over the child results. More-derived selections and values take precedence. Root tweaks can therefore override values supplied by any child. Changing a child selection must replace the inherited selection before expansion, so values from the previous child do not leak into the result. A merge of already resolved value maps is insufficient to preserve these semantics.

Live resolution uses the master to supply omitted defaults and selections. Frozen resolution uses the captured, explicit definitions. Missing references in a frozen bundle are errors, rather than opportunities to substitute current built-ins. Importing a seed's settings for a new generation is a separate migration and live-resolution operation; inspecting the old seed must not update it.

The backend currently stores concrete `Settings` in `SeedData.settings`. How that field and the frozen explanatory bundle coexist, or whether one becomes derived from the other, remains an implementation decision. Both must describe exactly the inputs actually used. An exact settings record does not itself promise identical output from a different generator release.

## Public seed representation

Construct the anonymous public recipe on the server at generation time. Use resolved settings, including unsaved edits and fixed values, rather than the creator's private preset structure. Obtain names and built-in definitions from trusted project data. Construct the public record from supported fields instead of copying arbitrary submitted metadata.

Choose explanatory built-in bases using a bottom-up process:

1. Choose bases for child sections first.
2. For a section, compare its directly owned concrete values with candidate built-ins.
3. Prefer the candidate requiring the fewest setting tweaks; among equally good candidates, prefer fewer explicit child-selection overrides.
4. Use a stable catalog-defined tie-break, without preferring the creator's private base.
5. Work upward to the root, collect all tweaks into the anonymous root recipe, and capture the required built-in definitions.

This produces one anonymous custom root, not anonymous custom presets throughout the hierarchy. The same effective settings and built-in revision should produce the same public settings representation regardless of how the creator reached them. The chosen bases explain the result and do not claim to record the creator's selections. They never alter the generated values or modify the creator's saved recipe.

Accept that bottom-up local choices may not give a globally smallest representation, particularly when child matches tie. Start with counts of differing settings rather than weighted notions of importance. Freeze the chosen explanation at generation time so later matching-rule changes do not reinterpret existing seeds.

## Compatibility and migration

Use explicit, ordered migrations from old settings versions to the current interpretation. Keep version information on every persisted or portable recipe, including browser drafts, files, and copied codes. The settings interpretation version and generator release serve different purposes; record the source release or master revision separately when identifying historical built-in definitions matters.

The read path is: parse the old envelope, migrate the raw recipe, validate against current definitions, resolve current built-ins, and remove redundant tweaks. Migration must run before current-schema sanitization; otherwise obsolete keys or option values can disappear before they can be translated. Malformed input and recognized historical input are different cases, and unrecognized values should not silently become current defaults.

- Added settings inherit the current selected presets and defaults.
- Changes to built-in defaults or presets affect inherited values automatically.
- Renamed identifiers and option tokens receive explicit translations; renamed display text does not change identity.
- Moving a setting between sections preserves root-level tweaks under the same stable key. It does not require moving private section presets, since those do not exist.
- Removed, split, or renamed section/preset references need deliberate migration rules.
- Removed settings and options need deliberate treatment when no exact equivalent exists; do not silently invent compatibility behavior.

Simple structural migration notices can be produced by the migration itself. Reporting all effective changes since a live preset was saved would additionally require historical definitions or a resolved snapshot. That richer history and notice system is deferred. A source revision alone cannot reconstruct definitions that have not been retained.

Frozen seed inspection follows the captured interpretation. Later schema changes must not erase historical values merely because the current editor no longer offers them. The appropriate historical UI description or a simpler historical inspection view is an open presentation question, separate from capturing the effective values.

## Boundary 1: UI definitions and bidirectional editing

### Current code

The frontend already has a useful path: `master_data.json` is loaded by `src/lib/master/load.ts`; `buildSchema` in `src/lib/master/adapter.ts` builds section and control descriptions; `SettingsStore` maintains a flat scalar map; `SettingRow` reads `store.get(setting.id)` and sends changes through `store.set(setting.id, value)`. Primitive controls receive a value and emit a typed change. `Level` already translates slider positions into actual level values rather than storing display indexes.

The current `nodeKey` generally uses community names, so it must change to support stable IDs independent of names. The current store supports user presets at all sections and user inheritance chains, which exceeds the simplified model. Links and drafts omit schema-default values without a settings version, and preset import sanitizes against current definitions before migration. These paths must converge on the shared contract.

### Mapping approaches

| Approach | Advantages | Costs |
| --- | --- | --- |
| Resolve to a scalar map and keep the current controlled-component pattern | Reuses the existing store and controls; widgets share one source of values | Saving must calculate recipe differences against the correct selected baseline |
| Make the recipe primary state and derive resolved values reactively | Reference choices and explicit tweaks are represented directly | Every edit, reset, and baseline change must maintain recipe normalization; changes more of the store |
| Maintain a separate nested editable UI model | Convenient for bespoke composite forms | Requires synchronization in both directions and creates another representation that can drift |

Tentative preference: keep the scalar map and explicit store updates. Put stable setting IDs on master leaves, map those IDs directly into control descriptors, and keep names and serialized wrappers outside widget state. Section layout determines where a control is rendered, not its identity.

Master-driven generic controls remain the default; hand-written components are an alternative for settings needing a distinct interaction. Both can use the same ID-based store interface. Preset selectors edit composition references separately from scalar values; a reference is not an ordinary string setting. Do not maintain a second editable tree mirroring the rendered layout.

The conceptual flow is:

```text
JSON recipe -> migration and resolution -> scalar working values -> controls
controls -> store.set(id, value) -> working values
working values + selected built-ins -> difference capture -> JSON recipe
```

Maintain the saved preset definition, its resolved baseline, and current working values as distinct concepts so existing tweak and unsaved-edit highlighting remains explainable. Saving compares current values with the chosen built-in baseline after section selections; it must not turn every resolved non-default value into a permanent tweak. Metadata is restored from the applicable definitions during serialization.

With the same master revision, saving and reloading should preserve the effective values and relevant built-in selections. Byte-for-byte JSON preservation is not required: names may be refreshed and redundant tweaks removed.

Controls should continue converting browser input into semantic booleans, numbers, and stable option values. Special compound controls can read or update several explicit setting IDs when necessary; do not introduce a generic binding language without a concrete need. Hidden values must still participate in generation even though they have no editable control.

### Resolution authority

The backend must authoritatively migrate, validate, and resolve generation requests. Local UI resolution is useful for immediate feedback. Options are a small TypeScript resolver matching the Rust rules, a shared Rust resolver exposed through WebAssembly, or server resolution for editing operations. The first has the least integration work but duplicates an algorithm; WebAssembly shares behavior but adds a build/runtime boundary; server-only resolution adds round trips and complicates offline editing.

This choice remains open. Keep preset definitions and interpretation rules shared regardless of implementation language. Decide how an editor using an older master revision interacts with an updated generation server, and return or otherwise expose the concrete accepted settings so the displayed seed always matches what generation used.

## Boundary 2: flat values and internal Rust structs

### Current code

`crates/seed_data/src/settings.rs` defines typed `Settings` with `map_layout`, proficiency/tech/notable lists, and an item pool. Technique identity is numeric; names are informational. `generator::prepare_inputs` already maps those identities into catalog-indexed vectors. `generator::generate` and the CLI also read typed settings directly. `cli::load_settings` currently deserializes JSON directly into `Settings`, and `SeedData` persists that type.

The frontend currently sends a flat scalar object to a stub `generateGame`; it is not connected to this Rust contract. Existing values and capabilities also differ: frontend `mapLayout = "randomized"` versus Rust `map_layout = Random` serialized as `"random"`; combat/boss defaults of 1 versus 0; frontend starting items without a corresponding Rust setting; and frontend underfilled item pools versus the generator's exact-size requirement. These need explicit reconciliation, rather than serialization silently masking them.

### Mapping approaches

| Approach | Advantages | Costs |
| --- | --- | --- |
| Generic entry unwrapping and Serde into flat typed structs | Reuses typed consumers without a custom converter per ordinary setting | Requires compatible stable keys and option tokens; Rust fields and enum variants are still declared |
| Explicit conversion into the existing grouped `Settings` | Reuses the current generator interface and exposes conversions | Fixed settings need explicit mappings, and UI-shaped grouping can cause unnecessary churn |
| Custom deserialization directly into `Settings` | Gives callers a typed result in one operation | Couples wire interpretation to the internal type and can obscure migration and preset resolution |
| Consume flat values directly throughout generation | Avoids an intermediate struct | Spreads key lookups and conversions into generator code and still needs catalog-indexed preparation |
| Generate flat Rust structs and choice enums from the authored JSON | Eliminates repeated field/type and option declarations | Adds a generator and requires the definition format to specify supported value types precisely |

Updated preference: expose ordinary generation settings through one flat typed Rust struct and customization through a separate struct. Organize both by stable semantic fields, not UI sections. Use one shared ingestion path for CLI and web callers, resolve before constructing these types, and deserialize ordinary settings generically instead of writing a conversion for each field. Keep authored defaults in the master rather than repeating them as independent Rust defaults. Do not put migration or preset interpretation into the generation algorithm.

The conceptual flow is:

```text
JSON recipe
  -> migrate, validate, resolve, normalize
  -> complete flat semantic values
  -> unwrap ordinary entries and deserialize flat GenerationSettings
     + generically collect dynamic setting families
  -> prepare_inputs with catalog lookups
  -> GenerationInputs
```

For ordinary settings, use canonical JSON keys matching the struct's Serde field names and stable choice tokens matching enum serialization. For example, one `map_layout` setting with `random`/`vanilla` tokens can deserialize into `map_layout: MapLayoutSetting` without a custom assignment. Prototype names currently differ; settle a canonical vocabulary rather than permanently maintaining frontend/backend translations. UI labels remain free to change.

Repeated families such as techniques, proficiencies, notables, and item counts use stable IDs and catalog definitions, rather than a hand-written field or branch for every individual entry. Handle each family once, obtaining identity from canonical keys or definition metadata. Flat ordinary fields can coexist with typed dynamic collections; the struct must not mirror the UI section tree. Domain-specific preparation such as mapping IDs into catalog indices remains a genuine conversion.

There are two degrees of single-source ownership. With a handwritten flat Serde struct, adding an ordinary setting requires its JSON definition and a Rust field/type, plus any actual behavior. Choice enum variants may also repeat the allowed token vocabulary. No per-setting renderer, getter/setter, default, or conversion branch is needed. If eliminating the remaining declarations is important, generate the struct and choice enums from the authored definitions. That is a justified use of generation for this requirement, but the handwritten-versus-generated decision remains open. A generator should support the small setting vocabulary, not arbitrary Rust destination paths, and produce deterministic field ordering independent of UI layout.

### Generating Rust structs and enums

If chosen, generate flat `GenerationSettings` and `CustomizationSettings` types, plus enums for closed, statically defined choices, from the randomizer-owned definitions. Generated types derive the appropriate Serde and ordinary Rust traits. Keep dynamic tech/notable/proficiency families as typed collections with one generic family adapter; do not generate fields or enum variants per upstream technique. Asset-populated choices may likewise need stable ID/string types validated against the assembled master rather than compile-time variants per asset.

The generator needs a small, explicit mapping from semantic value kinds to Rust types, including integer versus fractional values. It must map stable JSON keys and choice tokens to valid, unambiguous Rust field/variant identifiers while preserving the wire spelling through Serde. Choose straightforward naming conventions and report source keys when generation fails. Documentation comments can come from authored help text. Defaults, numeric constraints, and built-in preset resolution remain owned by the definitions/resolver rather than independently maintained generated policy.

Run generation before compiling the types' crate. A small Rust `build.rs` can read local authored definitions, write generated modules under Cargo's `OUT_DIR`, declare its input files with `rerun-if-changed`, and include those modules in the crate. This follows [Cargo's code-generation pattern](https://doc.rust-lang.org/cargo/reference/build-script-examples.html#code-generation) and [build-script output rules](https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script). The generator must not depend on the backend executable or the types it is generating. There should be no network access or running service requirement for Rust type generation, and no dependency on compiling full logic/retiling assets merely to declare ordinary fields.

Generate fields and enum variants in a deterministic order based on stable identities, not UI traversal order. Moving a setting between cards or changing a label should not change its Rust API. Adding or removing a field/choice can intentionally produce compiler errors at consumers that need semantic updates. Code generation does not implement the setting's behavior or migrate historical inputs.

The main drawbacks are owning a small generator, defining value kinds precisely enough for both runtime and Rust, diagnosing errors in generated source, less convenient source inspection/navigation, extra build work and build-dependencies, and supporting new type shapes when the simple setting vocabulary is insufficient. Keep this generator limited to declarations and serialization attributes. Do not generate the frontend application, migrations, arbitrary conversion logic, or an open-ended Rust binding language.

Generated Rust files should preferably not be checked in. Commit the authored definitions, generator, and inclusion code; regenerate into `OUT_DIR` during normal builds. This prevents stale generated files and merge conflicts and makes the JSON authoritative. The alternative is an explicit regeneration command producing checked-in source, with a freshness check to ensure it matches the definitions. That makes generated Rust diffs visible in review and easier to browse, but adds duplicated source, regeneration discipline, and noisy diffs. Do not have `build.rs` rewrite tracked source files. The choice to use generation itself remains open; untracked build output is the tentative recommendation if it is adopted.

Perform type, allowed-value, and cross-setting validation at ingestion, retaining generator checks that depend on the actual catalog or prevent invalid generation. Once resolved, omitted inherited values are no longer missing data, and the internal conversion should not choose a second set of defaults. Informational names cannot affect identity or override trusted public names.

The typed internal model need not have the same shape as the wire format. Keep one public settings contract while allowing Rust structs and indexed vectors to serve their internal consumers. If the saved-seed concrete settings field continues using typed `Settings`, document whether that is an internal seed field or migrate it to the shared flat encoding; do not accidentally establish another independently evolving editable-settings contract.

## Customization settings

Customization should use a master description supplied by the randomizer, with the same definition/entry conventions and generic frontend controls as generation settings. Replace `src/lib/settings/customizationSchema.ts` as an authoritative setting list; it currently contains sample cosmetic settings and option lists directly in TypeScript. Keep generation and customization as separate logical settings domains, with separate resolved Rust structs and requests appropriate to their consumers. They may be served together while retaining that separation.

Generation settings determine the saved game and its logic assumptions. Customization settings apply while producing the player's ROM and can be selected independently for an existing seed. A setting belongs to generation if it changes assumptions used to produce the game, even if its implementation ultimately writes ROM bytes. Physical implementation in the patcher is not sufficient reason to classify it as customization.

The existing patcher `PatchSettings` is a small patch-time configuration containing `transition_asset_phase`, rather than an implementation of the frontend's sample cosmetic list. Decide which settings are actually user-facing before replacing that list; moving placeholder definitions into JSON does not implement their effects. Asset-derived choices, such as supported sprites, should come from the corresponding supported asset definitions when available rather than a second hard-coded frontend list.

Use the same migration and preset mechanisms where applicable, with domain identity carried by the document/request context. Settle customization persistence, versioning, and supported patcher capabilities separately from generation. Historical seed settings remain frozen while the user's patch-time customization is interpreted for the patcher being used; do not automatically bind the customization vocabulary to the seed's old generation master. Exact preset UI operations and whether customization later needs special value types remain open.

## Scope of future implementation

Implementation must reach the master definitions and stable key adapter; the root-only preset model; draft/browser storage and preset import/export; generation requests and responses; shared Rust ingestion and CLI defaults; typed settings conversion and generator consumers; and frozen seed construction/inspection. Cosmetic settings need the same boundary decisions where applicable, while their separation from generation settings remains intact.

The immediate next design work is to settle authored master assembly, handwritten versus generated Rust types, the two mapping boundaries, the location and sharing of migration/resolution code, the reserved JSON syntax, and live/frozen envelope versioning. Exact UI operations, parentless presets, private sub-presets, user inheritance chains, recursive freezing operations, and richer historical notices are deferred. Do not implement those extensions merely to preserve flexibility.
