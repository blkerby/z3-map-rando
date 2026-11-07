# Settings and presets redesign: plan

Working plan for reorganizing the Generate settings into a section hierarchy with layered presets. Phases are meant to be done in order; each should leave the site working.

## Concepts

### Sections

- Sections form a tree (e.g. top level › Items › Item Pool / Starting Items). A section may hold settings, child sections, or both.
- Every section has presets: at least a built-in "Default". Each section designates one built-in as its default.
- Settings have a stable identity (their id) independent of which section they are in, so a setting can move to a different section across randomizer versions.

### Presets

- A **built-in preset** of a section stands on its own: values for the section's own settings, plus a choice of built-in preset for each child section with presets.
- A **user preset** of a section has:
  - a **base**: a built-in preset of the same section (never another user preset of the same section: the base relationship is flat);
  - a **selection for each child section** with presets: any preset of that child, built-in or user. The user preset follows that child preset when it changes (it is a reference, not a copy);
  - **tweaks**: a flat map of setting id -> absolute value, across the section's whole subtree. Which section a tweak belongs to is determined from the current schema when the preset is applied.
- Saving a user preset: the base is the built-in behind the current selection; child selections are the current ones; tweaks are the values that differ from what base + child selections give. Tweaks equal to the baseline are dropped.
- Saving while a user preset of the same section is selected (e.g. "Mine" selected, then "Save as new") flattens: the new preset takes Mine's base, with Mine's own tweaks folded in.

### Section selections

A section (other than the top level) either takes what the presets around it choose for it, or has a preset picked by hand:

- **Inherited** (the state right after selecting a top-level preset): the section takes the preset chosen for it, plus whatever the presets around it change inside it (their tweaks, and their picks for sections inside it). Its dropdown shows e.g. "Hard + 2 tweaks", or just "Hard" when they change nothing.
- **Picked by hand**: the section takes the picked preset plainly; the presets around it don't reach into it. The selection itself is the unsaved change, covering everything under it, so the rows inside are clean. Picking plain "Hard" is how tweaks are removed. The dropdown still lists "Hard + 2 tweaks" to go back. Picking the preset it would inherit, when nothing is changed inside, counts as inheriting.

### Two comparisons

- **Unsaved changes** (strong shade): differences from the selected presets, as applied; a section picked by hand is one unsaved change (its selection).
- **Tweaks** (weak shade): differences from the selected preset with its own tweaks removed. For a section's own settings that means the preset's base; for a child section it means the selected child preset.
- A setting that is both is shown with the strong shade.

### Reset

Resets (per row, per card, top-level) only undo unsaved changes: values go back, and sections picked by hand inside go back to inheriting (e.g. back to "Hard + 2 tweaks"). Tweaks are removed by picking the plain preset instead (see Section selections).

### Lifecycle

- Updating a user preset that others refer to changes them too (they hold references).
- Deleting a user preset that others refer to: each referring preset switches to the deleted preset's base, with the deleted preset's tweaks folded in (except settings it already tweaks). Values do not change.
- A missing built-in preset (removed in a newer version) falls back to that section's default built-in.
- No UI for changing a user preset's base for now. To rebase: select the other built-in, redo the tweaks, save over. Each section's preset row shows a muted "based on X" when a user preset is selected.

### Migration when settings move between sections

- A top-level preset's tweaks follow automatically (they are keyed by setting id and hold absolute values).
- For a section user preset (e.g. Logic "Hard tweaked"): a tweak whose setting moves within the section's subtree still applies. If it moves out of the subtree, the tweak moves up into every user preset that refers to this one (unless that preset already tweaks the setting), and is removed from this one. If nothing refers to it, it is kept unused, in case the setting moves back.
- Presets record the schema version they were last migrated to; migration runs when they are loaded.

### Sharing

- Share links and preset codes cannot refer to the sender's user presets, so they are flattened: built-in bases plus tweaks.
- On opening a link, a temporary unsaved top-level preset "Shared settings" is built: for every section (top level included), the recipient's closest preset for that section, built-in or user, scored by the number of differing settings among the settings that section holds directly (each section scored independently; ties go to the built-in named in the link, then the default built-in). What is left over becomes its tweaks, so a shared seed shows tweaks but no unsaved changes.
- Saving "Shared settings" flattens it like any other save. Before saving, the top-level row shows e.g. "Shared settings · based on Mine".

### UI

- Summary panel: unsaved changes on top as now ("Reset all" applies only to them), including changed section selections (e.g. a chip "Logic preset: Expert"). Below, a collapsible "Tweaks" list grouped by section ("Logic, based on Hard: 3 tweaks"), with muted chips that jump to the setting. Starts expanded when there are no unsaved changes.
- A changed section selection gets the strong shade on its dropdown, with a tooltip naming the preset's selection (e.g. "Mine uses Hard"). A folded card's "N changed" count includes it.

## Phase 1: schema and master data

- Unified structure: the top level is a list of items, each either a setting or a section; a section in turn holds a list of items. Settings outside any section are shown loose (no card), and nested sections are shown as nested cards. The customization settings are loose items too.
- Rename the "Map" section to "General" (id `general`).
- Split "Items" into an "Items" section containing two sub-sections:
  - "Item Pool": the current item counts, with built-in preset "Full" (the current defaults).
  - "Starting Items": a count for every pool item (id `start.<Item>`, linked to its pool item by `poolItem`), with built-in presets "None" (all 0, the default) and "All" (the Full pool counts).
  - "Items" itself gets just a "Default" preset (Full + None). This needs composite presets, so it lands in Phase 2; until then Items has no preset picker.
- Section presets are listed in the master (`presets` on a section). Logic keeps its derived difficulty presets. A section without presets gets a built-in "Default".
- Remove calculated counts (and the random filler split, Reroll). Every item is a plain count, with max = `itemLocations`; no more No/Yes display for 0–1 counts.
- Validation (blocks generation): the Item Pool total must not exceed `itemLocations` (fewer is allowed: the randomizer leaves locations empty); each starting count must not exceed its pool count.
- Keep the customization options but no longer group them into sections (they are loose items; sections can be added later).

## Phase 2: preset model

Store, types, and preset storage implementing the concepts above: section tree, built-in and user preset shapes, the two baselines, selection changes, reset of unsaved changes, deletion folding, built-in fallback, migration with schema versions. Items gets its "Default" composite preset.

Notes on how it was done:

- The root is a section like any other (id `root`), with the top-level presets. Every section has a preset picker.
- A user preset records only the selections that differ from what its base (and the presets selected inside it) would choose, so it follows changes to those presets too.
- All of a store's user presets are kept under one localStorage key with the schema version; the migration of tweaks runs when the stored version is older.
- Share links name the built-in preset behind each selection (flattening the tweaks is Phase 4).
- Preset codes and exported files still carry references to the player's other presets; flattening them is Phase 4.

## Phase 3: UI

Strong and weak shading, the changed-selection marker on dropdowns, "based on X", and the summary panel's Tweaks list.

Notes on how it was done:

- A setting is shaded as a tweak when it is behind the "+ N tweaks" of a section that isn't picked by hand (or, for settings outside any section, when the top-level preset's tweaks set it). An unsaved change takes the stronger shade instead.
- A dropdown picked by hand gets the strong tint, with a tooltip naming what Reset goes back to. Folded cards count picked dropdowns in their "N changed".
- The summary panel counts picked dropdowns as changes ("Logic preset: Hard" chips, which jump to the card). The Tweaks list ("N tweaks in Mine") is grouped by section, headed e.g. "Logic: Hard + 2 tweaks". It starts open only while there are no unsaved changes; once folded or unfolded by hand it stays that way.
- The customization panel has no Tweaks list yet.

## Phase 4: share links

Flattened links and preset codes; the "Shared settings" preset built from closest matches.
