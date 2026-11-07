import type { Item, Preset, Section, Setting, Value } from './types';
import { PresetList, type PresetContent } from './presetList.svelte';
import { readStoredPresets } from './presetIO';
import { SearchState } from './search.svelte';

/** Flatten settings and their nested `details` into one list. */
export function flatten(settings: Setting[]): Setting[] {
	return settings.flatMap((s) => [s, ...('details' in s && s.details ? flatten(s.details) : [])]);
}

/** A section's own settings (with their details), leaving out nested sections. */
export function ownSettings(section: Section): Setting[] {
	const out: Setting[] = [];
	for (const item of section.items) if (item.kind !== 'section') out.push(...flatten([item]));
	return out;
}

/** Every setting in a list of items (with their details), nested sections included. */
export function settingsIn(items: Item[]): Setting[] {
	const out: Setting[] = [];
	for (const item of items) out.push(...(item.kind === 'section' ? settingsIn(item.items) : flatten([item])));
	return out;
}

/** Every section in a list of items, nested ones included, parents first. */
function sectionsIn(items: Item[]): Section[] {
	const out: Section[] = [];
	for (const item of items) if (item.kind === 'section') out.push(item, ...sectionsIn(item.items));
	return out;
}

/** The sections directly inside a section. */
function childSections(section: Section): Section[] {
	const out: Section[] = [];
	for (const item of section.items) if (item.kind === 'section') out.push(item);
	return out;
}

/** Item locations used by a section's counts (see Section.pool). */
export interface PoolStatus {
	total: number;
	used: number;
}

/** What applying a preset gives: every setting's value and the preset selected in every section. */
interface Resolution {
	values: Record<string, Value>;
	selections: Record<string, string>;
	/** For each section inside: the preset the presets around it choose for it. */
	implied: Record<string, string>;
	/** A section to work out as if it took what the presets around it choose (its pick and any picks inside it ignored). */
	inherit?: string;
	/** Work out every preset as its built-in base (user presets' choices and tweaks left out). */
	builtInOnly?: boolean;
}

/** Id of the root section: everything, with the presets for all the settings at once. */
const ROOT_ID = 'root';

export class SettingsStore {
	/** Everything, as one section: its items are the top-level settings and sections, its presets are for all the settings. */
	readonly root: Section;
	/** The top-level list: loose settings and sections. */
	readonly items: Item[];
	/** Every section below the root, nested ones included, parents first. */
	readonly sections: Section[];
	/** The presets for each section (the root included), by section id. */
	readonly lists: Record<string, PresetList> = {};
	readonly all: Setting[];
	/** Lookup by id. Null-prototype so ids from untrusted links like "constructor" can't match. */
	readonly byId: Record<string, Setting | undefined>;
	readonly defaults: Record<string, Value>;
	/** The section each setting belongs to directly (the root for settings outside any section), by setting id. */
	readonly ownerOf: Record<string, Section> = {};

	values = $state<Record<string, Value>>({});
	/** Shared search box state (the same object may be shared with other stores). */
	readonly search: SearchState;
	/** When true the settings are read-only (a game has been generated from them). */
	locked = $state(false);
	/**
	 * Cards the player has opened or folded by hand, by section id. Cards not in here use their default:
	 * open, unless the section says `collapsed`. Searching opens every card with a match.
	 */
	opened = $state<Record<string, boolean>>({});
	/** Set when the player's presets couldn't be saved in this browser. */
	storageFailed = $state(false);

	/** The values the selected presets give, and what they choose for each section (see `resolve`). */
	private readonly layers = $derived.by(() => {
		const applied = this.emptyResolution();
		this.resolve(this.root, this.rootPreset, {}, true, true, applied);
		const plain = this.emptyResolution();
		this.resolve(this.root, this.rootPreset, {}, true, false, plain);
		const builtIn = { ...this.emptyResolution(), builtInOnly: true };
		this.resolve(this.root, this.rootPreset, {}, true, true, builtIn);
		return { applied: applied.values, implied: applied.implied, builtInImplied: builtIn.implied, plain: plain.values };
	});

	/**
	 * For each section: the settings inside it that the presets around it change from the preset they
	 * choose for it (with their tweaks, and their picks for sections inside it), had it not been picked by hand.
	 */
	private readonly inheritedChanges = $derived.by(() => {
		const out: Record<string, string[]> = {};
		for (const sec of this.sections) {
			const inherited = this.emptyResolution(sec.id);
			this.resolve(this.root, this.rootPreset, {}, true, true, inherited);
			const plain = this.resolveFresh(sec, this.impliedPreset(sec)).values;
			const ids: string[] = [];
			for (const s of settingsIn(sec.items)) if (inherited.values[s.id] !== plain[s.id]) ids.push(s.id);
			out[sec.id] = ids;
		}
		return out;
	});

	/**
	 * Settings the selected presets set differently from the preset shown for their section: the tweaks
	 * behind "Hard + 2 tweaks" in sections that aren't picked by hand, and the top-level preset's tweaks of
	 * settings outside any section.
	 */
	private readonly tweaked = $derived.by(() => {
		const out: Record<string, boolean> = {};
		for (const sec of this.sections) {
			if (this.isSelectionChanged(sec)) continue;
			for (const id of this.inheritedChanges[sec.id]) out[id] = true;
		}
		for (const s of ownSettings(this.root)) if (this.applied[s.id] !== this.plain[s.id]) out[s.id] = true;
		return out;
	});

	constructor(items: Item[], presets: Preset[], search: SearchState = new SearchState()) {
		this.search = search;
		this.items = items;
		this.root = { kind: 'section', id: ROOT_ID, title: 'All settings', presets, items };
		this.sections = sectionsIn(items);
		for (const sec of [this.root, ...this.sections]) this.lists[sec.id] = new PresetList(sec.presets, sec.defaultPreset);
		this.lists[ROOT_ID].selectedId = this.lists[ROOT_ID].defaultId;
		this.all = settingsIn(items);
		for (const sec of [this.root, ...this.sections]) for (const s of ownSettings(sec)) this.ownerOf[s.id] = sec;
		this.byId = Object.assign(Object.create(null), Object.fromEntries(this.all.map((s) => [s.id, s])));
		this.defaults = Object.fromEntries(this.all.map((s) => [s.id, s.default]));
		this.values = this.resolveFresh(this.root, this.rootPreset).values;
	}

	get query(): string {
		return this.search.query;
	}

	set query(value: string) {
		this.search.query = value;
	}

	listFor(section: Section): PresetList {
		return this.lists[section.id];
	}

	/** The selected presets, e.g. "Default · Logic: Hard + 2 tweaks". */
	get presetSummary(): string {
		let out = this.rootPreset.name;
		for (const sec of this.sections) out += ` · ${sec.title}: ${this.selectionLabel(sec)}`;
		return out;
	}

	/** The selected top-level preset. */
	get rootPreset(): Preset {
		const list = this.lists[ROOT_ID];
		return list.find(list.selectedId) ?? list.defaultPreset;
	}

	/** The preset the presets around a section choose for it. */
	impliedPreset(section: Section): Preset {
		const list = this.lists[section.id];
		return list.find(this.layers.implied[section.id]) ?? list.defaultPreset;
	}

	/** The preset a section takes: the one picked for it by hand, otherwise the one chosen for it. */
	selectedPreset(section: Section): Preset {
		if (section === this.root) return this.rootPreset;
		return this.lists[section.id].find(this.lists[section.id].selectedId) ?? this.impliedPreset(section);
	}

	/** How many settings inside a section the presets around it change (see `inheritedChanges`). */
	tweakCount(section: Section): number {
		return this.inheritedChanges[section.id]?.length ?? 0;
	}

	/**
	 * Does a section that isn't picked by hand take something other than what the built-in presets behind
	 * the selected ones would give it: a different preset chosen for it, or tweaks inside it?
	 */
	isSelectionTweaked(section: Section): boolean {
		if (section === this.root || this.isSelectionChanged(section)) return false;
		return this.tweakCount(section) > 0 || this.layers.implied[section.id] !== this.layers.builtInImplied[section.id];
	}

	/** Is this setting one of the selected presets' tweaks (see `tweaked`)? */
	isTweaked(id: string): boolean {
		return this.tweaked[id] === true;
	}

	/** Ids of the selected presets' tweaks. */
	get tweakedIds(): string[] {
		return this.all.filter((s) => this.isTweaked(s.id)).map((s) => s.id);
	}

	/** What a section takes, e.g. "Hard + 2 tweaks", or "Expert" when picked by hand. */
	selectionLabel(section: Section): string {
		if (section === this.root || this.isSelectionChanged(section)) return this.selectedPreset(section).name;
		return this.inheritedLabel(section);
	}

	/** The label of a section's choice when it isn't picked by hand, e.g. "Hard + 2 tweaks". */
	inheritedLabel(section: Section): string {
		const name = this.impliedPreset(section).name;
		const n = this.tweakCount(section);
		return n ? `${name} + ${n} tweak${n === 1 ? '' : 's'}` : name;
	}

	get(id: string): Value {
		return this.values[id];
	}

	set(id: string, value: Value) {
		if (this.locked) return;
		this.values[id] = value;
	}

	// ---- Working out what presets give ------------------------------------------

	private emptyResolution(inherit?: string): Resolution {
		return { values: { ...this.defaults }, selections: {}, implied: {}, inherit };
	}

	/**
	 * Work out a section with `preset` applied, into `out`: its own settings come from the preset (or its
	 * base); each section inside takes the preset chosen for it (by `outer`, the choices of the presets
	 * around this one, which win; then by this preset; then by its base; otherwise the section's default);
	 * then this preset's tweaks go on top.
	 *
	 * With `follow`, a section inside that has a preset picked by hand takes that preset instead, plainly:
	 * this preset's choices and tweaks don't reach into it. Without `follow`, picks are ignored. With `tweaks` false, this preset's own tweaks are left
	 * out (not those of presets inside it).
	 *
	 * Returns the ids of the settings this preset decides (where its tweaks apply).
	 */
	private resolve(
		section: Section,
		preset: Preset,
		outer: Record<string, string>,
		follow: boolean,
		tweaks: boolean,
		out: Resolution
	): string[] {
		const base = this.lists[section.id].builtInBehind(preset);
		if (out.builtInOnly) preset = base;
		out.selections[section.id] = preset.id;
		const decided: string[] = [];
		for (const s of ownSettings(section)) {
			out.values[s.id] = base.values[s.id] ?? this.defaults[s.id];
			decided.push(s.id);
		}
		const chosen = { ...base.sections, ...(preset === base ? undefined : preset.sections), ...outer };
		for (const child of childSections(section)) {
			const list = this.lists[child.id];
			const implied = list.find(chosen[child.id]) ?? list.defaultPreset;
			out.implied[child.id] = implied.id;
			const followChild = follow && child.id !== out.inherit;
			const picked = followChild ? list.find(list.selectedId) : undefined;
			if (picked) {
				this.resolve(child, picked, {}, true, true, out);
				continue;
			}
			decided.push(...this.resolve(child, implied, chosen, followChild, true, out));
		}
		if (tweaks && preset !== base) {
			for (const [id, v] of Object.entries(preset.values)) if (decided.includes(id)) out.values[id] = v;
		}
		return decided;
	}

	/** A section with a preset applied as if just selected (current selections inside it ignored). */
	private resolveFresh(section: Section, preset: Preset): Resolution {
		const out = this.emptyResolution();
		this.resolve(section, preset, {}, false, true, out);
		return out;
	}

	/** The values the selected presets give. Differences from these are unsaved changes. */
	get applied(): Record<string, Value> {
		return this.layers.applied;
	}

	/** The values the selected presets give without the top-level preset's own tweaks. */
	get plain(): Record<string, Value> {
		return this.layers.plain;
	}

	// ---- Item locations -----------------------------------------------------------

	poolOf(section: Section): PoolStatus | null {
		if (!section.pool) return null;
		let used = 0;
		for (const s of ownSettings(section)) if (s.kind === 'number') used += Number(this.values[s.id]) || 0;
		return { total: section.pool.total, used };
	}

	/** Reasons the current settings can't be generated. Empty when everything is fine. */
	get problems(): string[] {
		const out: string[] = [];
		for (const sec of this.sections) {
			const p = this.poolOf(sec);
			if (p && p.used > p.total) {
				out.push(`${sec.title}: ${p.used} items is more than the ${p.total} item locations. Lower some counts.`);
			}
		}
		for (const s of this.all) {
			if (s.kind !== 'number' || !s.poolItem) continue;
			const pooled = Number(this.values[s.poolItem]) || 0;
			if (Number(this.values[s.id]) > pooled) {
				out.push(`${s.label}: starting with ${this.values[s.id]} is more than the item pool has (${pooled}).`);
			}
		}
		return out;
	}

	// ---- Unsaved changes ------------------------------------------------------------

	/** Does this setting differ from what the selected presets give? */
	isChanged(id: string): boolean {
		return this.values[id] !== this.applied[id];
	}

	/** Was this section's preset picked by hand (rather than taking what the presets around it choose)? */
	isSelectionChanged(section: Section): boolean {
		if (section === this.root) return false;
		return this.lists[section.id].find(this.lists[section.id].selectedId) !== undefined;
	}

	/** Ids that differ from the selected presets. */
	get changedIds(): string[] {
		return this.all.filter((s) => this.isChanged(s.id)).map((s) => s.id);
	}

	/** Sections picked by hand (their selection is an unsaved change). */
	get changedSections(): Section[] {
		return this.sections.filter((sec) => this.isSelectionChanged(sec));
	}

	/** How many unsaved changes there are in this section: changed settings and sections picked by hand, inside it included. */
	countChanges(section: Section): number {
		let n = 0;
		for (const sec of [section, ...sectionsIn(section.items)]) if (this.isSelectionChanged(sec)) n++;
		for (const s of settingsIn(section.items)) if (this.isChanged(s.id)) n++;
		return n;
	}

	/** Has anything in this section (its selection, settings and sections inside included) changed? */
	sectionChanged(section: Section): boolean {
		return this.countChanges(section) > 0;
	}

	reset(id: string) {
		if (this.locked) return;
		this.values[id] = this.applied[id];
	}

	/** Undo the unsaved changes in one section (selections inside it included), leaving everything else alone. */
	resetSection(section: Section) {
		if (this.locked) return;
		for (const sec of [section, ...sectionsIn(section.items)]) if (sec !== this.root) this.lists[sec.id].selectedId = null;
		const applied = this.applied;
		for (const s of settingsIn(section.items)) this.values[s.id] = applied[s.id];
	}

	resetAll() {
		this.resetSection(this.root);
	}

	/**
	 * Pick a preset for a section and take its values (and the presets it chooses for sections inside
	 * it). An empty id, or the preset the presets around it choose when they change nothing in it, means
	 * taking what they choose.
	 */
	applyPreset(id: string, section: Section) {
		if (this.locked) return;
		const list = this.lists[section.id];
		if (id !== '' && !list.find(id)) return;
		const inherit = section !== this.root && (id === '' || (id === this.impliedPreset(section).id && this.tweakCount(section) === 0));
		this.adoptPreset(section, inherit ? null : id);
		const applied = this.applied;
		for (const s of settingsIn(section.items)) this.values[s.id] = applied[s.id];
	}

	/** Make `id` (null: what the presets around it choose) a section's selection, with the sections inside it taking what it chooses. */
	private adoptPreset(section: Section, id: string | null) {
		this.lists[section.id].selectedId = id;
		for (const sec of sectionsIn(section.items)) this.lists[sec.id].selectedId = null;
	}

	/** After saving or updating a preset for a section from the current settings: make it the section's selection. */
	adoptSaved(section: Section, id: string) {
		const inherit = section !== this.root && !this.isSelectionChanged(section) && id === this.impliedPreset(section).id;
		this.adoptPreset(section, inherit ? null : id);
	}

	// ---- User presets -----------------------------------------------------------

	/**
	 * What a user preset for this section saved now would hold: the built-in preset behind the selected
	 * one as its base, the selections inside the section that differ from what would otherwise be chosen,
	 * and the settings that differ from what those give.
	 */
	capture(section: Section): PresetContent {
		const base = this.lists[section.id].builtInBehind(this.selectedPreset(section));
		const sections: Record<string, string> = {};
		this.recordSelections(section, base, {}, sections);
		const given = this.resolveFresh(section, { id: '', name: '', base: base.id, sections, values: {} }).values;
		const values: Record<string, Value> = {};
		for (const s of settingsIn(section.items)) if (this.values[s.id] !== given[s.id]) values[s.id] = this.values[s.id];
		return { base: base.id, sections, values };
	}

	/** Record the current selections inside a section where they differ from what `preset` and `outer` choose. */
	private recordSelections(section: Section, preset: Preset, outer: Record<string, string>, out: Record<string, string>) {
		const base = this.lists[section.id].builtInBehind(preset);
		const chosen = { ...base.sections, ...(preset === base ? undefined : preset.sections), ...outer };
		for (const child of childSections(section)) {
			const list = this.lists[child.id];
			const implied = list.find(chosen[child.id]) ?? list.defaultPreset;
			const current = list.find(list.selectedId) ?? this.impliedPreset(child);
			if (current.id !== implied.id) out[child.id] = current.id;
			this.recordSelections(child, current, { ...chosen, ...out }, out);
		}
	}

	/** Would updating the selected preset (a user preset) change it? */
	canUpdate(section: Section): boolean {
		const saved = this.selectedPreset(section);
		if (!this.lists[section.id].isUser(saved.id)) return false;
		const now = this.capture(section);
		return (
			now.base !== saved.base ||
			!sameEntries(now.sections ?? {}, saved.sections ?? {}) ||
			!sameEntries(now.values, saved.values)
		);
	}

	/**
	 * Delete a user preset. Presets that refer to it switch to its base and take over its choices and
	 * tweaks (unless they have their own), so their values don't change.
	 */
	deletePreset(section: Section, id: string) {
		const list = this.lists[section.id];
		const gone = list.userPresets.find((p) => p.id === id);
		if (!gone) return;
		for (const other of Object.values(this.lists)) {
			for (const p of other.userPresets) {
				if (p.sections?.[section.id] !== id) continue;
				p.sections = { ...gone.sections, ...p.sections, [section.id]: gone.base! };
				p.values = { ...gone.values, ...p.values };
			}
		}
		list.userPresets = list.userPresets.filter((p) => p.id !== id);
		if (list.selectedId === id) list.selectedId = list.builtInOrDefault(gone.base).id; // the current settings are kept
	}

	/** Load the player's presets from this browser, moving tweaks of settings that changed section (see `migrate`). */
	loadUserPresets(key: string, version: number) {
		let data: { version?: unknown; lists?: Record<string, unknown> } = {};
		try {
			data = JSON.parse(localStorage.getItem(key) ?? '{}') ?? {};
		} catch {
			/* storage unavailable or unreadable: start without saved presets */
		}
		for (const [secId, list] of Object.entries(this.lists)) {
			list.userPresets = readStoredPresets(data.lists?.[secId], (v) => this.sanitize(v));
		}
		if (typeof data.version === 'number' && data.version < version) this.migrate();
	}

	/** Save the player's presets in this browser. */
	saveUserPresets(key: string, version: number) {
		const lists: Record<string, Preset[]> = {};
		for (const [secId, list] of Object.entries(this.lists)) if (list.userPresets.length) lists[secId] = list.userPresets;
		try {
			localStorage.setItem(key, JSON.stringify({ version, lists }));
			this.storageFailed = false;
		} catch {
			this.storageFailed = true;
		}
	}

	/**
	 * After settings have moved between sections: a section preset's tweak of a setting that is no longer
	 * inside the section moves up into the presets that refer to it (unless they have their own), so they
	 * keep the same values. With nothing referring to it, it stays, unused, in case the setting moves back.
	 */
	private migrate() {
		for (const section of this.sections) {
			const inside = settingsIn(section.items).map((s) => s.id);
			for (const preset of this.lists[section.id].userPresets) {
				for (const [id, v] of Object.entries(preset.values)) {
					if (inside.includes(id)) continue;
					let moved = false;
					for (const other of Object.values(this.lists)) {
						for (const p of other.userPresets) {
							if (p.sections?.[section.id] !== preset.id) continue;
							if (!(id in p.values)) p.values[id] = v;
							moved = true;
						}
					}
					if (moved) delete preset.values[id];
				}
			}
		}
	}

	/** Keep only known, correctly-typed values from untrusted input (only those given; nothing is filled in). */
	sanitize(raw: unknown): Record<string, Value> {
		const out: Record<string, Value> = {};
		if (typeof raw !== 'object' || raw === null) return out;
		for (const [id, val] of Object.entries(raw)) {
			const s = this.byId[id];
			if (!s) continue;
			if (s.kind === 'choice' && typeof val === 'string' && s.options.some((o) => o.value === val)) {
				out[id] = val;
			} else if (s.kind === 'toggle' && typeof val === 'boolean') {
				out[id] = val;
			} else if (s.kind === 'level' && typeof val === 'number' && s.levels.some((l) => l.value === val)) {
				out[id] = val;
			} else if (s.kind === 'number' && typeof val === 'number' && Number.isFinite(val)) {
				out[id] = Math.min(s.max, Math.max(s.min, val));
			}
		}
		return out;
	}

	// ---- Display and search -------------------------------------------------------

	/** Human-readable label for a value, for summaries and chips. */
	display(id: string): string {
		const s = this.byId[id];
		const v = this.values[id];
		if (!s) return String(v);
		if (s.kind === 'choice') return s.options.find((o) => o.value === v)?.label ?? String(v);
		if (s.kind === 'toggle') return v ? 'On' : 'Off';
		if (s.kind === 'level') return s.levels.find((l) => l.value === v)?.label ?? String(v);
		return String(v);
	}

	/** Does this setting (or any of its details), or anything in this section, match the search box? */
	matches(item: Item): boolean {
		const q = this.query.trim().toLowerCase();
		if (!q) return true;
		const settings = item.kind === 'section' ? settingsIn(item.items) : flatten([item]);
		return settings.some(
			(x) => x.label.toLowerCase().includes(q) || x.id.toLowerCase().includes(q) || (x.notes ?? '').toLowerCase().includes(q)
		);
	}

	/** How many rows (settings, not counting their details) match the search box, nested sections included. */
	countMatches(items: Item[] = this.items): number {
		let n = 0;
		for (const item of items) {
			if (item.kind === 'section') n += this.countMatches(item.items);
			else if (this.matches(item)) n++;
		}
		return n;
	}

	/** The sections holding an item (a setting or section, by id), outermost first. Empty for a top-level item. */
	pathTo(id: string, items: Item[] = this.items): Section[] {
		for (const item of items) {
			if (item.kind !== 'section') continue;
			if (item.id === id) return [item];
			const inner = this.pathTo(id, item.items);
			if (inner.length || ownSettings(item).some((s) => s.id === id)) return [item, ...inner];
		}
		return [];
	}

	/** Is this card unfolded (by hand, by default, or because the search box has a match in it)? */
	isOpen(section: Section): boolean {
		if (!section.collapsible || this.query.trim()) return true;
		return this.opened[section.id] ?? !section.collapsed;
	}

	// ---- Links and drafts -----------------------------------------------------------

	/**
	 * Compact string of the current settings: the selected presets and the values that differ from the
	 * schema defaults. Share links can't use the player's own presets, since other people don't have them,
	 * so they name the built-in preset behind each; pass `keepPreset` for local storage, where the
	 * selected presets should be remembered.
	 */
	serialize(opts: { keepPreset?: boolean } = {}): string {
		const s: Record<string, string | null> = {};
		for (const [secId, list] of Object.entries(this.lists)) {
			const picked = list.find(list.selectedId);
			s[secId] = !picked ? null : opts.keepPreset ? picked.id : list.builtInBehind(picked).id;
		}
		const v: Record<string, Value> = {};
		for (const x of this.all) if (this.values[x.id] !== x.default) v[x.id] = this.values[x.id];
		const json = JSON.stringify({ s, v });
		return btoa(unescape(encodeURIComponent(json)))
			.replace(/\+/g, '-')
			.replace(/\//g, '_')
			.replace(/=+$/, '');
	}

	/** Load a settings string. Unknown ids and wrong types are ignored. (Not blocked by `locked`: used to open shared games.) */
	load(code: string): boolean {
		try {
			const b64 = code.replace(/-/g, '+').replace(/_/g, '/');
			const json = decodeURIComponent(escape(atob(b64)));
			const { s, v } = JSON.parse(json) as { s?: Record<string, unknown>; v: unknown };
			for (const [secId, list] of Object.entries(this.lists)) list.selectedId = list.find(s?.[secId])?.id ?? null;
			this.lists[ROOT_ID].selectedId ??= this.lists[ROOT_ID].defaultId;
			this.values = { ...this.defaults, ...this.sanitize(v) };
			return true;
		} catch {
			return false;
		}
	}
}

/** Do two records hold the same entries? */
function sameEntries(a: Record<string, unknown>, b: Record<string, unknown>): boolean {
	const keys = Object.keys(a);
	if (keys.length !== Object.keys(b).length) return false;
	for (const k of keys) if (!(k in b) || a[k] !== b[k]) return false;
	return true;
}
