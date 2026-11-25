# Pending work

An index of work that is planned or in progress. The design of implemented systems is in [`docs/`](../docs/), starting from the [architecture](../docs/architecture.md). Details belong in the linked documents; keep each entry short and remove it when the work is complete.

## In progress

- [Settings processing](settings-processing.md): milestones 1–3 are complete; milestones 4 onward move browser loading onto the backend.

## Planned

- [Splitting area $80](splitting_area_80.md): give Sacred Grove and Under the Bridge their own themes within area `$80`.
- [Seed corpus test](seed-corpus.md): read committed seeds from every release with current code and catalogs.

## Deferred

- Door-specific Small Keys in the ROM: keys still use vanilla behavior, while logic assigns each key one door (see [door-specific keys](../docs/generation/keys.md)).
- Replacement English text for equipment receipt messages.
- Repeatable refill sources: source-data annotations distinguishing sources safe to repeat during a route from those that reset only between routes. Adding route refills must preserve valid cached states and reconsider capacity-dependent refill representations (see [traversal](../docs/generation/traversal.md#magic-ammunition-and-bottles)).
- Generation refinements: placement balancing and progression pacing, alternative restart destinations, reduced graphs for the Small Key pass, Thieves' Town's asymmetric reverse entrance lock, and timing runs of the incremental traverser.
- Retired identifiers in the [seed vocabulary](../crates/catalog_builder/tests/seed_vocabulary.rs): the retired list and migration form, designed when an identifier is first retired after release.
- Fine-grained CGRAM uploads for ordinary scrolling transitions, only if NMI measurements show the full 512-byte `$15` upload is too expensive. `$15=$80` would upload only the background palette rows set in an 8-bit mask: the main loop loads a row into `$7EC300`, copies it to the matching `$7EC500` row so later full uploads stay consistent, and sets its bit; NMI uploads each marked row from `$7EC300` and clears the mask. A pending full upload supersedes the mask, including `$81` from legacy increments, and clears it. This mode stays independent of `$17=$03`. See [overworld asset loading](../docs/engine/overworld_assets.md).
- Spoiler refinements (see [spoiler](../docs/generation/spoiler.md)): a filter for which items to list, adjustable logic settings (techs, proficiencies) to explore how they change the spheres, route queries for the flags and doors a route depends on, and route starts from other spawn points, which needs tracking the discoveries that unlock them.
- [Customization](customization.md): proposal, not implemented.
- [Dungeon layouts](dungeon.md) and [text](text.md): deferred until needed.
