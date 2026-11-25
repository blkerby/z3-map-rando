# Pending work

An index of work that is planned or in progress. Details belong in the linked documents; keep each entry short and remove it when the work is complete.

## In progress

- [Spoiler](spoiler.md): seed-page spoiler computed by replaying traversal in the browser. Work item 1 is done; items 2–5 remain, after the room theme refactor below.
- [Overworld asset loading](asset_loading.md): submilestones 9B.1–9B.4 are implemented; gameplay validation is pending.
- [BG1 streamer](bg1_streamer.md): milestone 4 is implemented; gameplay validation is pending before milestone 5.
- [Settings processing](settings-processing.md): milestones 1–3 are complete; milestones 4 onward move browser loading onto the backend.

## Planned

- Room themes in the world layout: move each room's theme into `OverworldRoomPlacement` and delete `AreaLayout`. `WorldLayout` lists rooms for the vanilla layout too, gains `vanilla: bool`, and keeps `borders` empty for vanilla. Saved seeds gain a top-level `vanilla_map` flag, omitted when false, and save every overworld room's position. Special rooms take their theme from a parent overworld room, recorded next to `common_types::SPECIAL_ROOM_AREAS`; the patcher derives area themes from the room placements. Needed before spoiler work item 2.
- [Splitting area $80](splitting_area_80.md): give Sacred Grove and Under the Bridge their own themes within area `$80`.
- [Seed corpus test](seed-corpus.md): read committed seeds from every release with current code and catalogs.

## Deferred

- [Incremental traversal](incremental-traversal.md): additional generation timing runs.
- [Prototype implementation plan](../plan.md#development-verification-and-deferred-work): deferred gameplay and generation work.
- Retired identifiers in the [seed vocabulary](../crates/catalog_builder/tests/seed_vocabulary.rs): the retired list and migration form, designed when an identifier is first retired after release.
- [Customization](customization.md): proposal, not implemented.
- [Dungeon layouts](dungeon.md) and [text](text.md): deferred until needed.
