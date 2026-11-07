import type { Item, Preset, Section, Setting, Value } from './types';
import { PresetList, type PresetContent } from './presetList.svelte';
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

/** Item locations used by a section's counts (see Section.pool). */
export interface PoolStatus {
	total: number;
	used: number;
}

export class SettingsStore {
	/** The top-level list: loose settings and sections. */
	readonly items: Item[];
	/** Every section, nested ones included, parents first. */
	readonly sections: Section[];
	/** Presets for all the settings at once. */
	readonly presets: PresetList;
	/** Presets for a single card, by section id (only for cards that have their own). */
	readonly sectionPresets: Record<string, PresetList | undefined> = {};
	readonly all: Setting[];
	/** Lookup by id. Null-prototype so ids from untrusted links like "constructor" can't match. */
	readonly byId: Record<string, Setting | undefined>;
	readonly defaults: Record<string, Value>;

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

	constructor(items: Item[], presets: Preset[], search: SearchState = new SearchState()) {
		this.search = search;
		this.items = items;
		this.sections = sectionsIn(items);
		this.presets = new PresetList(presets);
		for (const sec of this.sections) if (sec.presets) this.sectionPresets[sec.id] = new PresetList(sec.presets);
		this.all = settingsIn(items);
		this.byId = Object.assign(Object.create(null), Object.fromEntries(this.all.map((s) => [s.id, s])));
		this.defaults = Object.fromEntries(this.all.map((s) => [s.id, s.default]));
		this.values = { ...this.defaults };
	}

	get query(): string {
		return this.search.query;
	}

	set query(value: string) {
		this.search.query = value;
	}

	/** The presets for one card if it has its own, otherwise the presets for all the settings. */
	listFor(section?: Section): PresetList {
		return (section && this.sectionPresets[section.id]) || this.presets;
	}

	/** The selected presets, e.g. "Default · Logic: Expert". */
	get presetSummary(): string {
		let out = this.presets.selected.name;
		for (const sec of this.sections) {
			const list = this.sectionPresets[sec.id];
			if (list) out += ` · ${sec.title}: ${list.selected.name}`;
		}
		return out;
	}

	/**
	 * Values the selected presets want: a card with its own presets takes its own settings' values from
	 * the preset selected for it, everything else from the selected full-settings preset.
	 */
	get baseline(): Record<string, Value> {
		const out = { ...this.defaults, ...this.presets.selected.values };
		for (const sec of this.sections) {
			const list = this.sectionPresets[sec.id];
			if (!list) continue;
			const values = list.selected.values;
			for (const s of ownSettings(sec)) out[s.id] = values[s.id] ?? this.defaults[s.id];
		}
		return out;
	}

	get(id: string): Value {
		return this.values[id];
	}

	set(id: string, value: Value) {
		if (this.locked) return;
		this.values[id] = value;
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

	isChanged(id: string): boolean {
		return this.values[id] !== this.baseline[id];
	}

	/** Ids that differ from the selected preset. */
	get changedIds(): string[] {
		return this.all.filter((s) => this.isChanged(s.id)).map((s) => s.id);
	}

	reset(id: string) {
		if (this.locked) return;
		this.values[id] = this.baseline[id];
	}

	/** Has any of this card's own settings moved from the selected preset? */
	sectionChanged(section: Section): boolean {
		return ownSettings(section).some((s) => this.isChanged(s.id));
	}

	/** Put this card's own settings back to the selected preset, leaving everything else alone. */
	resetSection(section: Section) {
		if (this.locked) return;
		const base = this.baseline;
		for (const s of ownSettings(section)) this.values[s.id] = base[s.id];
	}

	resetAll() {
		if (this.locked) return;
		this.values = { ...this.baseline };
	}

	/**
	 * Select a preset (for one card, or for all the settings) and take its values. A full-settings
	 * preset also selects the card presets it was saved with.
	 */
	applyPreset(id: string, section?: Section) {
		if (this.locked) return;
		const list = this.listFor(section);
		const preset = list.find(id);
		if (!preset) return;
		list.selectedId = preset.id;
		if (section) {
			this.resetSection(section);
			return;
		}
		for (const sec of this.sections) this.sectionPresets[sec.id]?.select(preset.groupPresets?.[sec.id]);
		this.values = { ...this.defaults, ...preset.values };
	}

	// ---- User presets -----------------------------------------------------------

	/**
	 * What a preset saved now would hold: a full snapshot of the values (one card's, or all of them, so
	 * later default changes can't silently alter it) and, for a full-settings preset, the selected card presets.
	 */
	capture(section?: Section): PresetContent {
		if (section) {
			const values: Record<string, Value> = {};
			for (const s of ownSettings(section)) values[s.id] = this.values[s.id];
			return { values };
		}
		const groupPresets: Record<string, string> = {};
		for (const sec of this.sections) {
			const list = this.sectionPresets[sec.id];
			if (list) groupPresets[sec.id] = list.selectedId;
		}
		return { values: { ...this.values }, groupPresets };
	}

	/** Would saving now change the selected preset (for one card, or for all the settings)? */
	hasChanges(section?: Section): boolean {
		if (section) return this.sectionChanged(section);
		if (this.changedIds.length) return true;
		const recorded = this.presets.selected.groupPresets;
		for (const sec of this.sections) {
			const list = this.sectionPresets[sec.id];
			if (list && list.selectedId !== (recorded?.[sec.id] ?? list.defaultId)) return true;
		}
		return false;
	}

	/**
	 * Keep only known, correctly-typed values from untrusted input; everything else gets its default.
	 * With a section, only that card's own values are kept.
	 */
	sanitize(raw: unknown, section?: Section): Record<string, Value> {
		const next = this.sanitizeAll(raw);
		if (!section) return next;
		const out: Record<string, Value> = {};
		for (const s of ownSettings(section)) out[s.id] = next[s.id];
		return out;
	}

	private sanitizeAll(raw: unknown): Record<string, Value> {
		const next = { ...this.defaults };
		if (typeof raw !== 'object' || raw === null) return next;
		for (const [id, val] of Object.entries(raw)) {
			const s = this.byId[id];
			if (!s) continue;
			if (s.kind === 'choice' && typeof val === 'string' && s.options.some((o) => o.value === val)) {
				next[id] = val;
			} else if (s.kind === 'toggle' && typeof val === 'boolean') {
				next[id] = val;
			} else if (s.kind === 'level' && typeof val === 'number' && s.levels.some((l) => l.value === val)) {
				next[id] = val;
			} else if (s.kind === 'number' && typeof val === 'number' && Number.isFinite(val)) {
				next[id] = Math.min(s.max, Math.max(s.min, val));
			}
		}
		return next;
	}

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

	/** Only the values that differ from the schema defaults (in one card, or in all of them). */
	diff(section?: Section): Record<string, Value> {
		const out: Record<string, Value> = {};
		for (const s of section ? ownSettings(section) : this.all) {
			if (this.values[s.id] !== s.default) out[s.id] = this.values[s.id];
		}
		return out;
	}

	/**
	 * Compact string of the current settings (only values that differ from defaults).
	 * Share links leave out the player's own presets, since other people don't have them;
	 * pass `keepPreset` for local storage, where the selected preset should be remembered.
	 */
	serialize(opts: { keepPreset?: boolean } = {}): string {
		const idOf = (list: PresetList) => (opts.keepPreset || !list.isUser(list.selectedId) ? list.selectedId : list.defaultId);
		const g: Record<string, string> = {};
		for (const sec of this.sections) {
			const list = this.sectionPresets[sec.id];
			if (list) g[sec.id] = idOf(list);
		}
		const json = JSON.stringify({ p: idOf(this.presets), g, v: this.diff() });
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
			const { p, g, v } = JSON.parse(json) as { p: unknown; g?: Record<string, unknown>; v: unknown };
			this.presets.select(p);
			for (const sec of this.sections) this.sectionPresets[sec.id]?.select(g?.[sec.id]);
			this.values = this.sanitize(v);
			return true;
		} catch {
			return false;
		}
	}
}
