# Settings and presets

## Status and goals

This document records the settings model agreed during design discussion. JSON field names and examples are provisional. The UI and Rust boundary approaches below are proposals for further discussion, not implementation decisions. This plan supersedes the more elaborate user-preset hierarchy proposed in the frontend's `docs/presets-plan.md`.

Use a common JSON interpretation for browser storage, import/export, and generation requests. Every released version should accept settings from any previous released version through explicit migration. The primary behavior is to follow evolving built-in presets and defaults while retaining the user's differences. Reproducing historical generator behavior from an editable settings document is not the primary goal.

Generated seeds have a different requirement: their settings must remain inspectable as they were interpreted at generation time. Preserve those concrete settings and enough frozen preset definitions to explain them. Gameplay adjustments made while patching are temporary and specific to that seed; saved customization presets contain presentation/control preferences. Future racing rulesets should be generic JSON constraints captured with the seed, independent of generation compatibility checks. Public seed records must not expose private preset names or other arbitrary user metadata. No account system is planned, so creators and other viewers receive the same public representation.

## Preset semantics

- User-defined presets exist only at root scope.
- Every user preset is based directly on one built-in root preset. User presets cannot inherit from other user presets, and parentless user presets are deferred.
- A user preset can choose built-in presets for sections such as Skill assumptions and override individual settings anywhere in the configuration.
- Built-in presets retain their section hierarchy and composition. Their definitions and the master description drive resolution; particular presets such as Hard are not hard-coded in the resolver.
- Saving a new preset from an existing user preset copies its built-in base, effective section selections, and tweaks. It creates an independently editable sibling, without a reference to the original user preset.
- Inherited values follow current built-in definitions. New settings inherit values from the applicable current presets and defaults.
- An explicit tweak represents a difference from its current inherited baseline. If migration or a preset update makes it equal to that baseline, remove it. There is no hidden per-setting pinning behavior.
- Compare tweaks against the baseline after applying section preset selections, rather than against raw schema defaults.

The section containing techniques, notables, and proficiencies is named "Skill assumptions", with stable section identifier `skill` and preset selector `skill_preset`. Logic is a broader concept: item progression rules, such as which locations may contain high-tier items and how many high-tier items may appear per stem, belong in their own section rather than Skill assumptions.

The deliberate limitation is that users cannot independently reuse private skill presets or share a private baseline across several configurations. Repeated personal tweaks must be maintained separately. Built-in section presets provide the supported composition mechanism.

The distinction between a saved tweak and an unsaved edit remains useful. Patch-time Gameplay adjustments are excluded from saved customization presets and have the seed as their baseline instead. Other exact save, reset, section-selection, and UI operations are deferred; they should not introduce additional inheritance semantics implicitly.

## Setting identity and JSON values

Store settings as a flat map of stable identifiers to entries. Concrete setting entries have a `value` and may have informational metadata such as `name`:

```json
{
  "map_layout": {
    "value": "random"
  },
  "tech.5": {
    "name": "canBombBoost",
    "value": true
  }
}
```

The identifier determines identity. `name` is ignored when interpreting values and can be omitted. Regenerate informational names from trusted definitions on export and when constructing a public seed record. A historical name can remain readable without becoming a permanent operational identifier.

Use stable authored IDs for techniques, proficiencies, and room-scoped notables. Ordinary setting keys and preset selector keys use `snake_case`, matching Rust field naming where applicable. Structured identifiers such as `tech.5` are reserved for dynamic families handled generically. The exact family key vocabulary, including item identities and prefixes, remains to be settled. Keys do not track the UI's current layout.

Use the same entry shape for all serialized values rather than accepting both scalars and wrapped entries. Internal UI state can still store scalars. Labels, grouping, control types, option descriptions, section membership, and validation information remain in the master description; do not reproduce the section tree in settings documents.

Preset selector entries contain `preset` instead of `value`. For example, `"root_preset": { "preset": "root_default" }` selects the built-in root preset, and `"skill_preset": { "preset": "skill_hard" }` selects a built-in Skill assumptions preset. This is the single reference mechanism for both root and child selections. Each entry supplies either a concrete value or a preset reference, not both. Ordinary string values never become references implicitly. Reference identifiers are stable; their display names are informational.

The master identifies selector slots and their relationships. The resolver uses those definitions rather than hard-coded knowledge of particular selector names. Preset selectors are consumed during resolution and do not become fields in `GenerationSettings` or `CustomizationSettings`; only the resulting concrete values proceed to typed settings deserialization.

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

An editable settings document records a version, the release or master revision it was saved against, a flat recipe containing its root preset selection, child preset selections, and setting tweaks. Built-in references resolve against the current definitions after migration. Do not embed historical built-in contents in ordinary saved settings merely to preserve history.

A frozen seed bundle records one anonymous root recipe and a registry of the built-in definitions actually reached after applying its selection overrides. Followed references resolve exclusively within that bundle. An overridden selection can remain in a captured definition without its unused target being included. Capture defaults as concrete values in the frozen built-in definitions and make otherwise implicit child selections explicit. Include required hidden or fixed generation values as well. This makes the effective values self-contained without embedding the master's section tree. The bundle describes the generated configuration, not a complete library for evaluating alternative selections.

The following miniature frozen bundle has only two settings. Real bundles include every definition and value needed to resolve the actual generation inputs. Envelope names and the distinction between live and frozen document contexts remain provisional.

```json
{
  "version": 1,
  "root": {
    "root_preset": {
      "preset": "root_default"
    },
    "skill_preset": {
      "preset": "skill_hard"
    },
    "map_layout": {
      "value": "vanilla"
    }
  },
  "presets": {
    "root_default": {
      "skill_preset": {
        "preset": "skill_basic"
      },
      "map_layout": {
        "value": "random"
      }
    },
    "skill_hard": {
      "tech.5": {
        "name": "canBombBoost",
        "value": true
      }
    }
  }
}
```

For future generation, resolve generically before deserializing concrete values into the internal Rust settings structs:

1. Parse and migrate the generic document, separating explicit preset selections from concrete tweaks.
2. Start expanding the selected built-in root preset, carrying the user's explicit selections through the traversal.
3. At each child selector, choose the user's explicit selection if present; otherwise use the selection supplied by the enclosing preset or the applicable default. Enclosing preset selections take precedence over defaults supplied inside their descendants.
4. Resolve the chosen child presets and combine their concrete values with the enclosing preset's values.
5. Apply the user's concrete tweaks over the selected built-in configuration, then normalize redundant tweaks and deserialize the complete scalar values into typed settings.

In the example, `skill_hard` replaces the inherited `skill_basic` selection before that child is expanded. Basic contributes no concrete values and its definition is therefore omitted from the bundle. JSON property order has no effect: the explicit `skill_preset` selection wins whether it appears before or after `root_preset`. Root tweaks can override values supplied by any child. A merge of independently resolved preset value maps is insufficient because values from a replaced preset could otherwise linger.

Live resolution uses the master to supply omitted defaults and selections. Frozen resolution uses the captured, explicit definitions. A missing reference is an error if resolution actually follows it, rather than an opportunity to substitute current built-ins; an overridden reference need not have a bundled target. Importing a seed's settings for a new generation is a separate migration and live-resolution operation; inspecting the old seed must not update it.

The backend currently stores concrete `Settings` in `SeedData.settings`. How that field and the frozen explanatory bundle coexist, or whether one becomes derived from the other, remains an implementation decision. Both must describe exactly the inputs actually used. An exact settings record does not itself promise identical output from a different generator release.

## Public seed representation

Construct the anonymous public recipe on the server at generation time. Use resolved settings, including unsaved edits and fixed values, rather than the creator's private preset structure. Obtain names and built-in definitions from trusted project data. Construct the public record from supported fields instead of copying arbitrary submitted metadata.

Choose explanatory built-in bases using a bottom-up process:

1. Choose bases for child sections first.
2. For a section, compare its directly owned concrete values with candidate built-ins.
3. Prefer the candidate requiring the fewest setting tweaks; among equally good candidates, prefer fewer explicit child-selection overrides.
4. Use a stable catalog-defined tie-break, without preferring the creator's private base.
5. Work upward to the root, collect all tweaks into the anonymous root recipe, and capture only the built-in definitions reached when resolving that recipe with its selection overrides.

This produces one anonymous custom root, not anonymous custom presets throughout the hierarchy. The same effective settings and built-in revision should produce the same public settings representation regardless of how the creator reached them. The chosen bases explain the result and do not claim to record the creator's selections. They never alter the generated values or modify the creator's saved recipe.

Accept that bottom-up local choices may not give a globally smallest representation, particularly when child matches tie. Start with counts of differing settings rather than weighted notions of importance. Freeze the chosen explanation at generation time so later matching-rule changes do not reinterpret existing seeds.

## Compatibility and migration

Use explicit, ordered migrations from old settings versions to the current interpretation. Keep version information on every persisted or portable recipe, including browser drafts, files, and copied codes. The settings interpretation version and generator release serve different purposes; record the source release or master revision separately when identifying historical built-in definitions matters.

For editable settings intended for future generation, the read path is: parse the old envelope, migrate the raw recipe, validate against current definitions, resolve current built-ins, and remove redundant tweaks. Migration must run before current-schema sanitization; otherwise obsolete keys or option values can disappear before they can be translated. Malformed input and recognized historical input are different cases, and unrecognized values should not silently become current defaults.

- Added settings inherit the current selected presets and defaults.
- Changes to built-in defaults or presets affect inherited values automatically.
- Renamed identifiers and option tokens receive explicit translations; renamed display text does not change identity.
- Moving a setting between sections preserves root-level tweaks under the same stable key. It does not require moving private section presets, since those do not exist.
- Removed, split, or renamed section/preset references need deliberate migration rules.
- Removed settings and options need deliberate treatment when no exact equivalent exists; do not silently invent compatibility behavior.

Simple structural migration notices can be produced by the migration itself. Reporting all effective changes since a live preset was saved would additionally require historical definitions or a resolved snapshot. That richer history and notice system is deferred. A source revision alone cannot reconstruct definitions that have not been retained.

### Historical seed inspection

Use the current master as a presentation guide for old seeds, with a separate migration workflow preserving the concrete meaning of the generation-time settings. The frontend should not need to consume every historical master schema. The current master supplies grouping, labels, and familiar display components where appropriate; it does not supply current preset values or determine which historical entries are allowed to survive.

| Behavior | Editable settings for future generation | Historical seed inspection |
| --- | --- | --- |
| Built-in definitions | Current definitions | Captured definitions |
| Inherited values | Follow current presets/defaults | Preserve generation-time meaning |
| Newly introduced setting | Use the current selected baseline | Supply a known historical equivalent or indicate absence |
| Obsolete setting or option | Translate for current generation where possible | Retain an explanation if no exact equivalent exists |
| Tweak comparison | Current selected baseline | Captured baseline |

The inspection path is:

```text
original frozen bundle
  -> upgrade document/reference format without changing its interpretation
  -> resolve captured preset definitions and concrete values
  -> migrate concrete historical meanings into the current inspection format
  -> present using the current master, with historical fallbacks
```

Bundle format upgrades can translate old entry or reference syntax but must not rebind references to current built-ins or insert current defaults. Resolve frozen references using their captured selection structure and values. Historical value migrations then translate identifiers, option tokens, and representations while preserving the original behavior. Share appropriate rename/conversion operations with live migration, without reusing live default-filling or sanitization unchanged.

A missing new setting is not automatically equivalent to today's default. If a feature was always disabled before becoming configurable, a migration can accurately supply `false`. A newly introduced generation constraint may instead have no meaningful historical value; show that it was not applicable or recorded in that version. Removed options without an exact modern equivalent should remain inspectable with their historical label/value rather than being silently approximated or discarded.

The read-only inspection model must support missing, legacy, and currently unsupported values. It should not require deserialization into today's `GenerationSettings`, whose mandatory fields and enum variants may be inappropriate for an old seed. Nor should editor sanitization erase entries simply because they are absent from the current master. Provide a simple read-only fallback using captured informational metadata and values for historical settings that no longer have a matching definition. Exact visual treatment remains a UI decision.

Preserve the historical preset explanation as well as the final values. A display such as "Hard + two tweaks" refers to captured Hard, not the current built-in with the same ID. Migrate captured preset comparison values through the corresponding historical transformations and compute tweak highlighting against that baseline. Current section layout may reorganize the display without changing the values or rebasing the preset explanation against today's definitions.

The backend should expose a current-format inspection response derived from the frozen record, so the frontend consumes one presentation contract rather than historical master formats. Keep the original bundle intact; this response is a projection, not a rewrite of seed history. Importing settings from the seed for a new generation starts the separate live migration/resolution workflow. The inspection projection does not itself promise identical output from a newer generator or replace the seed's authoritative patching/racing interpretation.

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

The customization annotation on generation settings can also drive the narrow patch-input fields and their projection from seed settings. This is a mechanical subset selection from the shared definitions, rather than a hand-written assignment for every gameplay feature. Presentation/control preset values and temporary Gameplay adjustments remain separate sources even when they ultimately populate one resolved patch-input struct.

The generator needs a small, explicit mapping from semantic value kinds to Rust types, including integer versus fractional values. It must map stable JSON keys and choice tokens to valid, unambiguous Rust field/variant identifiers while preserving the wire spelling through Serde. Choose straightforward naming conventions and report source keys when generation fails. Documentation comments can come from authored help text. Defaults, numeric constraints, and built-in preset resolution remain owned by the definitions/resolver rather than independently maintained generated policy.

Run generation before compiling the types' crate. A small Rust `build.rs` can read local authored definitions, write generated modules under Cargo's `OUT_DIR`, declare its input files with `rerun-if-changed`, and include those modules in the crate. This follows [Cargo's code-generation pattern](https://doc.rust-lang.org/cargo/reference/build-script-examples.html#code-generation) and [build-script output rules](https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script). The generator must not depend on the backend executable or the types it is generating. There should be no network access or running service requirement for Rust type generation, and no dependency on compiling full logic/retiling assets merely to declare ordinary fields.

Generate fields and enum variants in a deterministic order based on stable identities, not UI traversal order. Moving a setting between cards or changing a label should not change its Rust API. Adding or removing a field/choice can intentionally produce compiler errors at consumers that need semantic updates. Code generation does not implement the setting's behavior or migrate historical inputs.

The main drawbacks are owning a small generator, defining value kinds precisely enough for both runtime and Rust, diagnosing errors in generated source, less convenient source inspection/navigation, extra build work and build-dependencies, and supporting new type shapes when the simple setting vocabulary is insufficient. Keep this generator limited to declarations and serialization attributes. Do not generate the frontend application, migrations, arbitrary conversion logic, or an open-ended Rust binding language.

Generated Rust files should preferably not be checked in. Commit the authored definitions, generator, and inclusion code; regenerate into `OUT_DIR` during normal builds. This prevents stale generated files and merge conflicts and makes the JSON authoritative. The alternative is an explicit regeneration command producing checked-in source, with a freshness check to ensure it matches the definitions. That makes generated Rust diffs visible in review and easier to browse, but adds duplicated source, regeneration discipline, and noisy diffs. Do not have `build.rs` rewrite tracked source files. The choice to use generation itself remains open; untracked build output is the tentative recommendation if it is adopted.

Perform type, allowed-value, and cross-setting validation at ingestion, retaining generator checks that depend on the actual catalog or prevent invalid generation. Once resolved, omitted inherited values are no longer missing data, and the internal conversion should not choose a second set of defaults. Informational names cannot affect identity or override trusted public names.

The typed internal model need not have the same shape as the wire format. Keep one public settings contract while allowing Rust structs and indexed vectors to serve their internal consumers. If the saved-seed concrete settings field continues using typed `Settings`, document whether that is an internal seed field or migrate it to the shared flat encoding; do not accidentally establish another independently evolving editable-settings contract.

## Customization settings

Customization should use a master description supplied by the randomizer, with the same definition/entry conventions and generic frontend controls as generation settings. Replace `src/lib/settings/customizationSchema.ts` as an authoritative setting list; it currently contains sample cosmetic settings and option lists directly in TypeScript. Keep generation and patching as distinct consumers with narrow resolved Rust inputs. Their descriptions may be served together, and a setting may participate in both contexts without duplicating its authored definition.

### Ownership and customization annotation

Settings changing gameplay rules belong to generation from the outset, even if the current logic does not yet model their consequences. Record their chosen values with the seed as its intended gameplay behavior. Movement abilities, combat rules, and resource behavior are examples. Customization-only settings concern presentation and personal control preferences that do not change those game rules.

A feature such as preserving dash speed through transitions has one definition but two possible concrete values: the seed's recorded generation value and the effective patch-time value after an optional adjustment. They may differ legitimately. This is separate from Skill assumptions: whether the game provides an ability and whether the player can execute a strat using it are different questions.

Initially use one annotation on a generation setting to indicate that it is exposed for temporary customization and included in the patcher's effective settings. A possible spelling is `customizable: true`; the exact syntax remains open, and it may instead be an object containing its compatibility relationship. Do not introduce independent "consumed by patcher" and "overrideable" flags without a concrete setting requiring that distinction. Customization-only presentation/control settings are already patch inputs.

The annotation is semantic metadata, independent of UI section membership. A Gameplay section in generation and a parallel customization Gameplay section can be derived from the same definitions. Reorganizing the UI must not change the setting's ownership, identity, or inclusion in saved presets.

Not everything consumed by the patcher is an adjustable setting. World layout, item placements, and other fixed generation results remain seed facts. A setting whose change requires regeneration should not be exposed through the patch-only adjustment mechanism merely because its effects ultimately involve ROM writes. If a genuine configuration field is needed by the patcher but should never be adjustable, reconsider whether separate annotations earn their weight then.

### Effective patch inputs and generation compatibility

A shared coordinator, used by CLI and backend callers, projects only the relevant generation values from the seed, combines them with temporary Gameplay adjustments and presentation/control preferences, and evaluates compatibility. The patcher receives the resulting concrete patch settings and required seed artifacts, without needing arbitrary generation settings such as tech/notable assumptions. Interpretation and compatibility policy belong outside the low-level patching implementation.

For a feature whose enabled behavior only adds capabilities, the directional relationship can be:

| Recorded generation value | Effective patch value | Interpretation |
| --- | --- | --- |
| Disabled | Disabled | Matches generation |
| Disabled | Enabled | Adds a capability without invalidating the assumed abilities |
| Enabled | Enabled | Honors the generation assumption |
| Enabled | Disabled | May invalidate the generation assumptions |

Compatibility is feature-specific; do not infer that every gameplay modification only adds capabilities. Other features may require equality or another defined relationship. A potentially incompatible adjustment does not prove the seed is unbeatable, since generation may not actually have needed the ability. Whether to reject it or permit it with a warning is an open product decision.

Recorded gameplay behavior and modeled logic assumptions are not identical. When a later release begins modeling an existing feature, compatibility handling for old seeds must respect the original generation semantics rather than retroactively claiming that an older generator assumed a capability it never modeled. Define any conservative warnings or historical compatibility translations deliberately.

### Gameplay adjustments and persistence

Patch-time Gameplay adjustments are excluded from saved customization presets and their imports/exports. Someone generating their own seed selects the intended gameplay settings during generation; someone opening another person's seed initially follows that seed's recorded gameplay settings. Loading or saving a presentation/control preset must not capture, replace, or propagate Gameplay adjustments.

Adjustments apply only to the current seed and should survive repeated patching and page refreshes. Use one active-seed draft in `sessionStorage`, tagged with the seed's identity and containing only values differing from that seed's settings. Restore it only when the seed identity matches. Switching seeds discards the previous draft and starts from the new seed's recorded values; switching A to B and back to A does not restore a history of old adjustments. `sessionStorage` survives page reloads and is scoped to a tab, but the seed check is application behavior rather than a browser guarantee. See [MDN's sessionStorage documentation](https://developer.mozilla.org/en-US/docs/Web/API/Window/sessionStorage).

This is temporary draft storage rather than a reusable preference or a change to the shared seed record. Restored adjustments remain visibly marked and receive the same compatibility checks as new edits. Persist only the normal versioned override data needed for that seed, not a collection of saved per-seed presets.

With no reusable Gameplay preferences, the earlier Follow seed/Enabled/Disabled preference model is unnecessary. A control can show the effective value, highlight a difference from the seed, and reset to the seed's value. An override equal to the seed value is redundant and can be removed. Exact controls remain a UI decision.

### Existing implementation and presentation/control presets

The existing patcher `PatchSettings` is a small patch-time configuration containing `transition_asset_phase`, rather than an implementation of the frontend's sample cosmetic list. Decide which settings are actually user-facing before replacing that list; moving placeholder definitions into JSON does not implement their effects. Asset-derived choices, such as supported sprites, should come from the corresponding supported asset definitions when available rather than a second hard-coded frontend list.

Use the same migration and preset mechanisms for saved presentation/control preferences where applicable, with domain identity carried by the document/request context. Their persistence is separate from the temporary active-seed Gameplay draft. Historical seed settings remain frozen while the user's patch-time customization is interpreted for the patcher being used; do not automatically bind the customization vocabulary to the seed's old generation master. Exact preset UI operations and whether customization later needs special value types remain open.

## Future racing rulesets

Keep racing conformance separate from generation compatibility. A patch-time change can preserve the ability to complete a seed yet violate the race's restrictions. Rulesets can constrain any applicable patch-time setting, including presentation choices, rather than only Gameplay adjustments.

Define rulesets as versioned JSON constraints keyed by stable setting IDs, evaluated by generic code. A particular tournament's rules should require data rather than a special branch in Rust. The following is a provisional example, not a requirement to implement rulesets immediately:

```json
{
  "version": 1,
  "constraints": {
    "preserve_dash_speed": {
      "match_seed": true
    },
    "link_sprite": {
      "allowed_values": ["link"]
    },
    "menu_speed": {
      "disallowed_values": ["instant"]
    }
  }
}
```

`match_seed: true` requires the effective patch value to equal the seed's recorded generation value. `allowed_values` requires membership in the given set. `disallowed_values` forbids membership in the given set. All constraints present on a setting apply together: a value listed in both sets is forbidden. A setting without a constraint has no additional restriction from this ruleset, while ordinary validation and generation compatibility still apply. Validate rule keys and operands against the relevant definitions; matching a seed value is meaningful only where such a value was recorded.

Start with a small constraint vocabulary. Numeric bounds, conditions, and relationships between settings can be added when an actual ruleset requires them. There is no need for a general expression language initially. New operators require evaluator code; a new ruleset using existing operators does not.

Evaluate the final concrete patch configuration, after inherited seed values, temporary Gameplay adjustments, and presentation/control preferences are resolved. A customization operation resulting in the same effective value is not itself a violation. Produce structured findings identifying the setting, constraint, expected value or allowed/disallowed set, and actual value. Preserve generation compatibility findings and racing findings as distinct results.

Capture the applied ruleset definition and interpretation version with the seed, rather than referring only to the latest definition of a named ruleset. Later changes to a race's rules must not reinterpret existing seeds. Public ruleset metadata follows the same restrictions on arbitrary user text as the rest of the public seed representation.

Conformance evaluation does not choose enforcement. Future behavior could reject nonconforming customization, permit it with an in-game indication on the title screen and/or menu, or use another explicitly selected policy. No enforcement policy is selected yet. Keep the final status available to whichever layer implements that policy; the patcher need not implement the generic evaluator or consume the full generation configuration merely to apply an indication. Full ruleset enforcement and in-game marking are deferred.

```text
seed's relevant gameplay values + temporary adjustments + presentation/control preferences
  -> effective patch settings
  -> generation compatibility findings and racing conformance findings
  -> enforcement decision
  -> narrow patch inputs and any required status indication
```

## Scope of future implementation

Implementation must reach the master definitions and stable key adapter; the root-only preset model; draft/browser storage and preset import/export; generation requests and responses; shared Rust ingestion and CLI defaults; typed settings conversion and generator consumers; and frozen seed construction/inspection. Historical inspection needs a current-format read-only projection preserving old concrete meanings and captured preset baselines, distinct from live recipe migration. Customization work must distinguish saved presentation/control preferences from temporary Gameplay adjustments, project narrow patch inputs through the shared annotation, and retain compatibility with the seed's original interpretation. The design must leave room for captured declarative racing constraints without requiring their full implementation now.

The immediate next design work is to settle authored master assembly, handwritten versus generated Rust types, the two mapping boundaries, the location and sharing of migration/resolution code, the customization annotation and compatibility relationships, the remaining key vocabulary, and live/frozen envelope versioning. Other exact UI operations, parentless presets, private sub-presets, user inheritance chains, recursive freezing operations, richer historical notices, and racing enforcement/marking are deferred. Do not implement those extensions merely to preserve flexibility.
