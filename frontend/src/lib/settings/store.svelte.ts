import type { CalculatedSetting, Group, NumberSetting, Preset, Setting, Value, ValueSetting } from './types';
import { PresetList, type PresetContent } from './presetList.svelte';
import { adjustSplit, expectedSplit, splitFiller, type FillerItem } from '../master/pool';
import { SearchState } from './search.svelte';

/** Flatten settings and their nested `details` into one list. */
export function flatten(settings: Setting[]): Setting[] {
	return settings.flatMap((s) => [s, ...('details' in s && s.details ? flatten(s.details) : [])]);
}

/** Slot accounting for a group with a fixed total (see Group.pool). */
export interface PoolStatus {
	total: number;
	/** Slots taken by the player's own counts. */
	used: number;
	/** Slots left for the calculated items (total - used). */
	filler: number;
	/** How many more slots the player can still take before calculated items run out of their minimums. */
	free: number;
}

function toFiller(c: CalculatedSetting): FillerItem {
	return { name: c.id, weight: c.weight, min: c.min, max: c.max };
}

export class SettingsStore {
	readonly groups: Group[];
	/** Presets for all the settings at once. */
	readonly presets: PresetList;
	/** Presets for a single card, by card id (only for cards that have their own). */
	readonly groupPresets: Record<string, PresetList | undefined> = {};
	readonly all: Setting[];
	/** Lookup by id. Null-prototype so ids from untrusted links like "constructor" can't match. */
	readonly byId: Record<string, Setting | undefined>;
	readonly defaults: Record<string, Value>;
	/** Settings that hold a value (everything except calculated ones). */
	private readonly tracked: ValueSetting[];
	/** For number settings inside a pool group: the group they share slots in. */
	private readonly poolGroupById: Record<string, Group | undefined>;

	values = $state<Record<string, Value>>({});
	/** Shared search box state (the same object may be shared with other stores). */
	readonly search: SearchState;
	/** When true the settings are read-only (a game has been generated from them). */
	locked = $state(false);

	constructor(groups: Group[], presets: Preset[], search: SearchState = new SearchState()) {
		this.search = search;
		this.groups = groups;
		this.presets = new PresetList(presets);
		for (const g of groups) if (g.presets) this.groupPresets[g.id] = new PresetList(g.presets);
		this.all = groups.flatMap((g) => flatten(g.settings));
		this.byId = Object.assign(Object.create(null), Object.fromEntries(this.all.map((s) => [s.id, s])));
		this.tracked = this.all.filter((s): s is ValueSetting => s.kind !== 'calculated');
		this.defaults = Object.fromEntries(this.tracked.map((s) => [s.id, s.default]));
		// Calculated items start at their expected amounts (same every time, so server and browser agree).
		for (const g of groups) {
			const calcs = this.fillersOf(g);
			if (!g.pool || !calcs.length) continue;
			const used = flatten(g.settings).reduce((n, s) => n + (s.kind === 'number' ? s.default : 0), 0);
			expectedSplit(g.pool.total - used, calcs.map(toFiller)).forEach((v, i) => (this.defaults[calcs[i].id] = v));
		}
		this.values = { ...this.defaults };
		this.poolGroupById = Object.assign(Object.create(null), {});
		for (const g of groups) {
			if (!g.pool) continue;
			for (const s of flatten(g.settings)) if (s.kind === 'number') this.poolGroupById[s.id] = g;
		}
	}

	get query(): string {
		return this.search.query;
	}

	set query(value: string) {
		this.search.query = value;
	}

	/** The presets for one card if it has its own, otherwise the presets for all the settings. */
	listFor(group?: Group): PresetList {
		return (group && this.groupPresets[group.id]) || this.presets;
	}

	/** The selected presets, e.g. "Default · Logic: Expert". */
	get presetSummary(): string {
		let out = this.presets.selected.name;
		for (const g of this.groups) {
			const list = this.groupPresets[g.id];
			if (list) out += ` · ${g.title}: ${list.selected.name}`;
		}
		return out;
	}

	/**
	 * Values the selected presets want: a card with its own presets takes its values from the preset
	 * selected for it, everything else from the selected full-settings preset.
	 */
	get baseline(): Record<string, Value> {
		const out = { ...this.defaults, ...this.presets.selected.values };
		for (const g of this.groups) {
			const list = this.groupPresets[g.id];
			if (!list) continue;
			const values = list.selected.values;
			for (const s of flatten(g.settings)) out[s.id] = values[s.id] ?? this.defaults[s.id];
		}
		return out;
	}

	get(id: string): Value {
		return this.values[id];
	}

	set(id: string, value: Value) {
		if (this.locked) return;
		const s = this.byId[id];
		// Counts that share a pool can't take more slots than are free.
		if (s?.kind === 'number' && typeof value === 'number') value = Math.min(value, this.limitOf(s));
		this.values[id] = value;
		if (s?.kind === 'number') this.rebalanceFor(id);
	}

	// ---- Calculated items ----------------------------------------------------------

	private fillersOf(group: Group): CalculatedSetting[] {
		return flatten(group.settings).filter((x): x is CalculatedSetting => x.kind === 'calculated');
	}

	/** Make the calculated items add up to the slots left, changing as little as possible (random where it must). */
	private rebalance(group: Group) {
		const calcs = this.fillersOf(group);
		const status = this.poolOf(group);
		if (!calcs.length || !status) return;
		const next = adjustSplit(
			calcs.map((c) => Number(this.values[c.id]) || 0),
			status.filler,
			calcs.map(toFiller),
			Math.random
		);
		calcs.forEach((c, i) => (this.values[c.id] = next[i]));
	}

	private rebalanceFor(id: string) {
		const g = this.poolGroupById[id];
		if (g) this.rebalance(g);
	}

	private rebalanceAll() {
		for (const g of this.groups) if (g.pool) this.rebalance(g);
	}

	/** Draw the calculated items again from scratch. */
	reroll(group: Group) {
		if (this.locked) return;
		const calcs = this.fillersOf(group);
		const status = this.poolOf(group);
		if (!calcs.length || !status || status.free < 0) return;
		const next = splitFiller(status.filler, calcs.map(toFiller), Math.random);
		calcs.forEach((c, i) => (this.values[c.id] = next[i]));
	}

	// ---- Pools (a fixed number of slots shared by several counts) ----------------

	poolOf(group: Group): PoolStatus | null {
		if (!group.pool) return null;
		let used = 0;
		let minFiller = 0;
		for (const s of flatten(group.settings)) {
			if (s.kind === 'number') used += Number(this.values[s.id]) || 0;
			else if (s.kind === 'calculated') minFiller += s.min;
		}
		const filler = group.pool.total - used;
		return { total: group.pool.total, used, filler, free: filler - minFiller };
	}

	/** Largest value a count can take right now: its own max, and never more than the pool has room for. */
	limitOf(s: NumberSetting): number {
		const group = this.poolGroupById[s.id];
		if (!group) return s.max;
		const status = this.poolOf(group);
		if (!status) return s.max;
		return Math.min(s.max, (Number(this.values[s.id]) || 0) + Math.max(0, status.free));
	}

	/** Reasons the current settings can't be generated. Empty when everything is fine. */
	get problems(): string[] {
		const out: string[] = [];
		for (const g of this.groups) {
			const p = this.poolOf(g);
			if (p && p.free < 0) {
				out.push(`${g.title}: ${-p.free} too many items for the ${p.total} slots. Lower some counts.`);
			}
		}
		return out;
	}

	isChanged(id: string): boolean {
		if (this.byId[id]?.kind === 'calculated') return false; // drawn, not chosen
		return this.values[id] !== this.baseline[id];
	}

	/** Ids that differ from the selected preset. */
	get changedIds(): string[] {
		return this.tracked.filter((s) => this.isChanged(s.id)).map((s) => s.id);
	}

	reset(id: string) {
		if (this.locked) return;
		this.values[id] = this.baseline[id];
		this.rebalanceFor(id);
	}

	/** Has anything in this card moved from the selected preset (including the drawn calculated items)? */
	groupChanged(group: Group): boolean {
		return flatten(group.settings).some((s) => (s.kind === 'calculated' ? this.values[s.id] !== this.baseline[s.id] : this.isChanged(s.id)));
	}

	/** Put everything in one card back to the selected preset, leaving the other cards alone. */
	resetGroup(group: Group) {
		if (this.locked) return;
		const base = this.baseline;
		for (const s of flatten(group.settings)) this.values[s.id] = base[s.id];
		this.rebalance(group); // a no-op when the preset's amounts already add up
	}

	resetAll() {
		if (this.locked) return;
		this.values = { ...this.baseline };
		this.rebalanceAll();
	}

	/**
	 * Select a preset (for one card, or for all the settings) and take its values. A full-settings
	 * preset also selects the card presets it was saved with.
	 */
	applyPreset(id: string, group?: Group) {
		if (this.locked) return;
		const list = this.listFor(group);
		const preset = list.find(id);
		if (!preset) return;
		list.selectedId = preset.id;
		if (group) {
			this.resetGroup(group);
			return;
		}
		for (const g of this.groups) this.groupPresets[g.id]?.select(preset.groupPresets?.[g.id]);
		this.values = { ...this.defaults, ...preset.values };
		this.rebalanceAll();
	}

	// ---- User presets -----------------------------------------------------------

	/**
	 * What a preset saved now would hold: a full snapshot of the values (one card's, or all of them, so
	 * later default changes can't silently alter it) and, for a full-settings preset, the selected card presets.
	 */
	capture(group?: Group): PresetContent {
		if (group) {
			const values: Record<string, Value> = {};
			for (const s of flatten(group.settings)) values[s.id] = this.values[s.id];
			return { values };
		}
		const groupPresets: Record<string, string> = {};
		for (const g of this.groups) {
			const list = this.groupPresets[g.id];
			if (list) groupPresets[g.id] = list.selectedId;
		}
		return { values: { ...this.values }, groupPresets };
	}

	/** Would saving now change the selected preset (for one card, or for all the settings)? */
	hasChanges(group?: Group): boolean {
		if (group) return this.groupChanged(group);
		if (this.changedIds.length) return true;
		const recorded = this.presets.selected.groupPresets;
		for (const g of this.groups) {
			const list = this.groupPresets[g.id];
			if (list && list.selectedId !== (recorded?.[g.id] ?? list.defaultId)) return true;
		}
		return false;
	}

	/**
	 * Keep only known, correctly-typed values from untrusted input; everything else gets its default.
	 * With a group, only that card's values are kept.
	 */
	sanitize(raw: unknown, group?: Group): Record<string, Value> {
		const next = this.sanitizeAll(raw);
		if (!group) return next;
		const out: Record<string, Value> = {};
		for (const s of flatten(group.settings)) out[s.id] = next[s.id];
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
			} else if (s.kind === 'calculated' && typeof val === 'number' && Number.isFinite(val)) {
				next[id] = Math.min(s.max ?? Infinity, Math.max(s.min, Math.round(val)));
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

	/** Does this setting (or any of its details) match the search box? */
	matches(s: Setting): boolean {
		const q = this.query.trim().toLowerCase();
		if (!q) return true;
		return flatten([s]).some(
			(x) => x.label.toLowerCase().includes(q) || x.id.toLowerCase().includes(q) || (x.notes ?? '').toLowerCase().includes(q)
		);
	}

	/** Groups with at least one setting matching the search box, each with its matching settings. */
	get visibleGroups(): { group: Group; rows: Setting[] }[] {
		return this.groups
			.map((group) => ({ group, rows: group.settings.filter((s) => this.matches(s)) }))
			.filter((x) => x.rows.length > 0);
	}

	changedIn(s: Setting): number {
		return flatten([s]).filter((x) => this.isChanged(x.id)).length;
	}

	/** Only the values that differ from the schema defaults (in one card, or in all of them). */
	diff(group?: Group): Record<string, Value> {
		const out: Record<string, Value> = {};
		for (const s of group ? flatten(group.settings) : this.tracked) {
			if (s.kind === 'calculated') continue;
			if (this.values[s.id] !== s.default) out[s.id] = this.values[s.id];
		}
		return out;
	}

	/** The drawn amounts of the calculated items. Always put in share links (even when they equal the defaults). */
	fillerValues(): Record<string, Value> {
		const out: Record<string, Value> = {};
		for (const s of this.all) if (s.kind === 'calculated') out[s.id] = this.values[s.id];
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
		for (const group of this.groups) {
			const list = this.groupPresets[group.id];
			if (list) g[group.id] = idOf(list);
		}
		const json = JSON.stringify({ p: idOf(this.presets), g, v: { ...this.diff(), ...this.fillerValues() } });
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
			for (const group of this.groups) this.groupPresets[group.id]?.select(g?.[group.id]);
			this.values = this.sanitize(v);
			this.rebalanceAll(); // a link that doesn't add up (edited by hand) is repaired
			return true;
		} catch {
			return false;
		}
	}
}
