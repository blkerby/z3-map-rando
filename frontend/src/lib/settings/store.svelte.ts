import { untrack } from 'svelte';
import { SearchState } from './search.svelte';
import { cloneTree, getAt, isTree, leafPaths, pathKey, setAt, startsWith, type Path, type Tree, type Value } from './tree';

/**
 * Settings and presets, following docs/data-formats/settings.md:
 *
 * - The root scope has built-in presets (complete settings) and the player's own presets. Child scopes
 *   (e.g. `skill`) have built-in presets only, each complete over its subtree.
 * - A settings document holds the built-in root `preset`, a `preset` selection inside each child scope
 *   object that differs from what the root preset selects, and tweaks: values that differ from that
 *   baseline. A user preset is a name plus such a document.
 * - Two comparisons: unsaved changes are differences from the selected preset as applied (with its tweaks);
 *   tweaks are differences between that and the same baseline without the preset's tweaks.
 * - A child scope either takes what the selected root preset chooses for it ("Hard + 2 tweaks"), or has a
 *   preset picked by hand, which it takes plainly; the pick itself is an unsaved change.
 *
 * The store knows settings only by path. Labels and value formatting come from the controls on the page,
 * which register themselves while mounted (see `register`).
 */

export const FORMAT_VERSION = 1;

export interface ScopePreset {
	id: string;
	label: string;
	/** Complete values of the scope's subtree. */
	settings: Tree;
}

export interface RootPreset {
	id: string;
	label: string;
	/** Complete values of every setting; child scope subtrees are replaced by the selected scope presets. */
	settings: Tree;
	/** Scope id -> built-in preset of that scope. Scopes left out take their first preset. */
	scopes: Record<string, string>;
}

/** A child scope: the settings under `settings[id]`. */
export interface Scope {
	id: string;
	label: string;
	presets: ScopePreset[];
}

export interface SettingsModel {
	/** Built-in root presets; the first is the default. */
	rootPresets: RootPreset[];
	scopes: Scope[];
}

export interface UserPreset {
	id: string;
	name: string;
	/** A settings document (without `format_version`). */
	document: Tree;
}

/** What a mounted control tells the store about its setting. */
export interface ControlInfo {
	path: Path;
	label: string;
	notes?: string;
	/** The card it is shown in. */
	cardId?: string;
	/** Text for a value, for summaries and chips. */
	format: (value: Value) => string;
}

export interface CardInfo {
	id: string;
	title: string;
	icon?: string;
	collapsed?: boolean;
}

export class SettingsStore {
	readonly model: SettingsModel;
	/** Prefix for element ids, so two stores on one page don't collide. */
	readonly idPrefix: string;
	readonly search: SearchState;
	/** Every setting's path, in the default preset's order. */
	readonly leaves: Path[];

	values = $state<Tree>({});
	/** The selected root preset: built-in or one of the player's own. */
	selectedId = $state('');
	/** Child scope id -> preset picked by hand, or null when it takes what the root preset chooses. */
	scopePicks = $state<Record<string, string | null>>({});
	userPresets = $state<UserPreset[]>([]);
	/** When true the settings are read-only (a game has been generated from them). */
	locked = $state(false);
	/** Cards folded or unfolded by hand, by card id. */
	opened = $state<Record<string, boolean>>({});
	/** Set when the player's presets couldn't be saved in this browser. */
	storageFailed = $state(false);
	/** Mounted controls, by path key. */
	controls = $state.raw<Record<string, ControlInfo>>({});
	/** Mounted cards, in page order. */
	cards = $state.raw<CardInfo[]>([]);

	private readonly layers = $derived.by(() => {
		const doc = this.selectedDocument;
		return {
			applied: this.resolve(doc, this.scopePicks, true),
			plain: this.resolve(doc, this.scopePicks, false),
			// The same with no scope picked by hand, for what the scopes would inherit.
			inherited: this.resolve(doc, {}, true),
			inheritedPlain: this.resolve(doc, {}, false)
		};
	});

	constructor(model: SettingsModel, idPrefix: string, search: SearchState = new SearchState()) {
		this.model = model;
		this.idPrefix = idPrefix;
		this.search = search;
		this.leaves = leafPaths(model.rootPresets[0].settings);
		this.selectedId = model.rootPresets[0].id;
		for (const scope of model.scopes) this.scopePicks[scope.id] = null;
		this.values = this.resolve(this.selectedDocument, {}, true);
	}

	get query(): string {
		return this.search.query;
	}

	// ---- Presets and their baselines ---------------------------------------------

	get rootPresets(): RootPreset[] {
		return this.model.rootPresets;
	}

	builtIn(id: unknown): RootPreset | undefined {
		return this.model.rootPresets.find((p) => p.id === id);
	}

	userPreset(id: unknown): UserPreset | undefined {
		return this.userPresets.find((p) => p.id === id);
	}

	isUser(id: string): boolean {
		return this.userPreset(id) !== undefined;
	}

	/** The selected root preset's name. */
	get presetName(): string {
		return this.userPreset(this.selectedId)?.name ?? this.builtIn(this.selectedId)?.label ?? '';
	}

	/** The selected root preset as a settings document. */
	get selectedDocument(): Tree {
		return this.userPreset(this.selectedId)?.document ?? { preset: this.selectedId };
	}

	/** The built-in root preset behind a document. */
	private rootOf(doc: Tree): RootPreset {
		return this.builtIn(doc.preset) ?? this.model.rootPresets[0];
	}

	/** The built-in root preset behind the selected one: itself if built-in. */
	get basePreset(): RootPreset {
		return this.rootOf(this.selectedDocument);
	}

	scopePreset(scope: Scope, id: unknown): ScopePreset | undefined {
		return scope.presets.find((p) => p.id === id);
	}

	/** The preset a document chooses for a scope: its own selection, else its root preset's. */
	private chosenFor(doc: Tree, scope: Scope): string {
		const own = getAt(doc, [scope.id, 'preset']);
		if (this.scopePreset(scope, own)) return own as string;
		return this.rootOf(doc).scopes[scope.id] ?? scope.presets[0].id;
	}

	/** The preset the selected root preset chooses for a scope. */
	impliedScopePreset(scope: Scope): ScopePreset {
		return this.scopePreset(scope, this.chosenFor(this.selectedDocument, scope))!;
	}

	/** The preset a scope takes: picked by hand, otherwise the one chosen for it. */
	selectedScopePreset(scope: Scope): ScopePreset {
		return this.scopePreset(scope, this.scopePicks[scope.id]) ?? this.impliedScopePreset(scope);
	}

	/**
	 * The values a document gives: its root preset, the scope presets it chooses (or `picks`), and with
	 * `tweaks` its values on top, except inside picked scopes, which take their preset plainly.
	 */
	private resolve(doc: Tree, picks: Record<string, string | null>, tweaks: boolean): Tree {
		const out = cloneTree(this.rootOf(doc).settings);
		const picked: Path[] = [];
		for (const scope of this.model.scopes) {
			const pick = this.scopePreset(scope, picks[scope.id]);
			if (pick) picked.push([scope.id]);
			const preset = pick ?? this.scopePreset(scope, this.chosenFor(doc, scope))!;
			out[scope.id] = cloneTree(preset.settings);
		}
		if (tweaks) {
			for (const path of this.leaves) {
				if (picked.some((p) => startsWith(path, p))) continue;
				const v = getAt(doc, path);
				if (typeof v === typeof getAt(out, path)) setAt(out, path, v as Value);
			}
		}
		return out;
	}

	/** The values the selected presets give. Differences from these are unsaved changes. */
	get applied(): Tree {
		return this.layers.applied;
	}

	/** The values the selected presets give without the root preset's tweaks. */
	get plain(): Tree {
		return this.layers.plain;
	}

	// ---- Values -----------------------------------------------------------------

	get(path: Path): Value {
		return getAt(this.values, path) as Value;
	}

	set(path: Path, value: Value) {
		if (this.locked) return;
		setAt(this.values, path, value);
	}

	/** Does this setting differ from what the selected presets give? */
	isChanged(path: Path): boolean {
		return getAt(this.values, path) !== getAt(this.applied, path);
	}

	/** Is this setting one of the selected root preset's tweaks (and not changed since)? */
	isTweaked(path: Path): boolean {
		return !this.isChanged(path) && getAt(this.applied, path) !== getAt(this.plain, path);
	}

	get changedPaths(): Path[] {
		return this.leaves.filter((p) => this.isChanged(p));
	}

	get tweakedPaths(): Path[] {
		return this.leaves.filter((p) => this.isTweaked(p));
	}

	/** The child scope a setting is in, if any. */
	scopeOf(path: Path): Scope | undefined {
		return this.model.scopes.find((s) => s.id === path[0]);
	}

	isScopePicked(scope: Scope): boolean {
		return this.scopePicks[scope.id] !== null;
	}

	/** Scopes with a preset picked by hand (each is an unsaved change). */
	get pickedScopes(): Scope[] {
		return this.model.scopes.filter((s) => this.isScopePicked(s));
	}

	/** How many settings in a scope the root preset's tweaks change, when it isn't picked by hand. */
	inheritedTweakCount(scope: Scope): number {
		const { inherited, inheritedPlain } = this.layers;
		let n = 0;
		for (const p of this.leaves) if (p[0] === scope.id && getAt(inherited, p) !== getAt(inheritedPlain, p)) n++;
		return n;
	}

	/** How many settings in a scope the root preset's tweaks change (none when it is picked by hand). */
	scopeTweakCount(scope: Scope): number {
		return this.isScopePicked(scope) ? 0 : this.inheritedTweakCount(scope);
	}

	/** Does a scope that isn't picked by hand take something other than the built-in base would give it? */
	isScopeTweaked(scope: Scope): boolean {
		if (this.isScopePicked(scope)) return false;
		return this.scopeTweakCount(scope) > 0 || this.impliedScopePreset(scope).id !== this.chosenFor({ preset: this.basePreset.id }, scope);
	}

	/** What a scope takes when it isn't picked by hand, e.g. "Hard + 2 tweaks". */
	inheritedScopeLabel(scope: Scope): string {
		const n = this.inheritedTweakCount(scope);
		const name = this.impliedScopePreset(scope).label;
		return n ? `${name} + ${n} tweak${n === 1 ? '' : 's'}` : name;
	}

	/** What a scope takes: "Expert" when picked by hand, else e.g. "Hard + 2 tweaks". */
	scopeLabel(scope: Scope): string {
		return this.isScopePicked(scope) ? this.selectedScopePreset(scope).label : this.inheritedScopeLabel(scope);
	}

	/** Unsaved changes: changed settings and picked scopes. */
	get changeCount(): number {
		return this.changedPaths.length + this.pickedScopes.length;
	}

	reset(path: Path) {
		if (this.locked) return;
		setAt(this.values, path, getAt(this.applied, path) as Value);
	}

	/** Undo the unsaved changes in a card: its settings, and the pick of its scope. */
	resetCard(cardId: string, scope?: Scope) {
		if (this.locked) return;
		if (scope) this.scopePicks[scope.id] = null;
		for (const c of Object.values(this.controls)) if (c.cardId === cardId) this.reset(c.path);
	}

	/** How many unsaved changes a card holds (its scope's pick included). */
	countCardChanges(cardId: string, scope?: Scope): number {
		let n = scope && this.isScopePicked(scope) ? 1 : 0;
		for (const c of Object.values(this.controls)) if (c.cardId === cardId && this.isChanged(c.path)) n++;
		return n;
	}

	resetAll() {
		if (this.locked) return;
		for (const scope of this.model.scopes) this.scopePicks[scope.id] = null;
		this.values = cloneTree(this.applied);
	}

	/** Select a root preset and take its values. */
	applyPreset(id: string) {
		if (this.locked || (!this.builtIn(id) && !this.isUser(id))) return;
		this.selectedId = id;
		this.resetAll();
	}

	/** Select another root preset, keeping the current values: where they differ, they become unsaved changes. */
	switchPreset(id: string) {
		if (this.builtIn(id) || this.isUser(id)) this.selectedId = id;
	}

	/**
	 * Pick a preset for a scope and take its values. '' means taking what the root preset chooses, as does
	 * picking that same preset when the root preset changes nothing in it.
	 */
	pickScopePreset(scope: Scope, id: string) {
		if (this.locked || (id !== '' && !this.scopePreset(scope, id))) return;
		const inherit = id === '' || (id === this.impliedScopePreset(scope).id && this.inheritedTweakCount(scope) === 0);
		this.scopePicks[scope.id] = inherit ? null : id;
		const applied = this.applied;
		for (const p of this.leaves) if (p[0] === scope.id) setAt(this.values, p, getAt(applied, p) as Value);
	}

	// ---- Settings documents -------------------------------------------------------

	/**
	 * The current settings as a document based on the built-in behind the selected preset, so it never refers
	 * to the player's own presets: the scope selections that differ from that built-in's, and the values that
	 * differ from what those give.
	 */
	capture(): Tree {
		const base = this.basePreset;
		const doc: Tree = { preset: base.id };
		for (const scope of this.model.scopes) {
			const id = this.selectedScopePreset(scope).id;
			if (id !== this.chosenFor({ preset: base.id }, scope)) doc[scope.id] = { preset: id };
		}
		const given = this.resolve(doc, {}, false);
		for (const path of this.leaves) {
			const v = getAt(this.values, path) as Value;
			if (v !== getAt(given, path)) setAt(doc, path, v);
		}
		return doc;
	}

	/**
	 * Read a settings document from untrusted input, leniently: unknown presets fall back to the default, and
	 * unknown settings or values of the wrong type are dropped.
	 */
	readDocument(raw: unknown): Tree {
		const doc: Tree = { preset: this.builtIn(getAt(raw, ['preset']))?.id ?? this.model.rootPresets[0].id };
		for (const scope of this.model.scopes) {
			const preset = this.scopePreset(scope, getAt(raw, [scope.id, 'preset']));
			if (preset) doc[scope.id] = { preset: preset.id };
		}
		const defaults = this.model.rootPresets[0].settings;
		for (const path of this.leaves) {
			const v = getAt(raw, path);
			if (typeof v === typeof getAt(defaults, path) && (typeof v !== 'number' || Number.isFinite(v))) setAt(doc, path, v as Value);
		}
		return doc;
	}

	/**
	 * Take the values a document gives, with its built-in preset selected. Its scope selections become picks
	 * and its tweaks unsaved changes.
	 */
	loadDocument(doc: Tree) {
		this.selectedId = this.rootOf(doc).id;
		for (const scope of this.model.scopes) {
			const id = this.chosenFor(doc, scope);
			this.scopePicks[scope.id] = id === this.chosenFor({ preset: this.selectedId }, scope) ? null : id;
		}
		this.values = this.resolve(doc, {}, true);
	}

	/** The current settings for a share link. */
	serialize(): string {
		return toBase64Url(JSON.stringify({ format_version: FORMAT_VERSION, ...this.capture() }));
	}

	/** Load a share link's settings. (Not blocked by `locked`: used to open shared games.) */
	load(code: string): boolean {
		try {
			this.loadDocument(this.readDocument(JSON.parse(fromBase64Url(code))));
			return true;
		} catch {
			return false;
		}
	}

	/** The current settings and selections, for keeping a draft in this browser. */
	serializeDraft(): string {
		return JSON.stringify({ selected: this.selectedId, picks: this.scopePicks, document: this.capture() });
	}

	loadDraft(text: string) {
		try {
			const draft = JSON.parse(text);
			this.loadDocument(this.readDocument(draft.document));
			this.switchPreset(draft.selected);
			for (const scope of this.model.scopes) {
				this.scopePicks[scope.id] = this.scopePreset(scope, getAt(draft, ['picks', scope.id]))?.id ?? null;
			}
		} catch {
			/* unreadable draft: keep the defaults */
		}
	}

	// ---- User presets -----------------------------------------------------------

	/** Would saving into the selected preset (one of the player's own) change it? */
	get canUpdate(): boolean {
		const preset = this.userPreset(this.selectedId);
		return preset !== undefined && JSON.stringify(this.capture()) !== JSON.stringify(preset.document);
	}

	addPreset(name: string, document: Tree): UserPreset {
		const preset = { id: newPresetId(), name, document };
		this.userPresets.push(preset);
		return this.userPresets[this.userPresets.length - 1];
	}

	/** Save the current settings into one of the player's presets and select it. */
	updatePreset(id: string) {
		const preset = this.userPreset(id);
		if (!preset) return;
		preset.document = this.capture();
		this.selectedId = id;
		for (const scope of this.model.scopes) this.scopePicks[scope.id] = null;
	}

	renamePreset(id: string, name: string) {
		const preset = this.userPreset(id);
		if (preset) preset.name = name;
	}

	/** Delete one of the player's presets. If it is selected, its built-in base is selected, keeping the current values. */
	deletePreset(id: string) {
		if (this.selectedId === id) this.selectedId = this.basePreset.id;
		this.userPresets = this.userPresets.filter((p) => p.id !== id);
	}

	loadUserPresets(key: string) {
		let data: unknown;
		try {
			data = JSON.parse(localStorage.getItem(key) ?? '{}');
		} catch {
			/* storage unavailable or unreadable: start without saved presets */
		}
		this.userPresets = this.readPresets(getAt(data, ['presets']));
	}

	saveUserPresets(key: string) {
		try {
			localStorage.setItem(key, JSON.stringify({ format_version: FORMAT_VERSION, presets: this.userPresets }));
			this.storageFailed = false;
		} catch {
			this.storageFailed = true;
		}
	}

	/** Read stored presets back, discarding anything malformed. */
	private readPresets(raw: unknown): UserPreset[] {
		const out: UserPreset[] = [];
		if (!Array.isArray(raw)) return out;
		for (const item of raw) {
			if (!isTree(item) || typeof item.name !== 'string') continue;
			const id = typeof item.id === 'string' && /^user-[a-z0-9]{4,32}$/.test(item.id) && !out.some((p) => p.id === item.id) ? item.id : newPresetId();
			out.push({ id, name: item.name, document: this.readDocument(item.document) });
		}
		return out;
	}

	// ---- Controls, cards and search -----------------------------------------------

	/** Called by a control while it is mounted. Returns the function that unregisters it. */
	register(info: ControlInfo): () => void {
		const key = pathKey(info.path);
		this.controls = untrack(() => ({ ...this.controls, [key]: info }));
		return () => {
			if (this.controls[key] !== info) return;
			const { [key]: _, ...rest } = this.controls;
			void _;
			this.controls = rest;
		};
	}

	registerCard(card: CardInfo): () => void {
		this.cards = untrack(() => [...this.cards, card]);
		return () => (this.cards = this.cards.filter((c) => c !== card));
	}

	/** A setting's label, from its control. */
	label(path: Path): string {
		return this.controls[pathKey(path)]?.label ?? pathKey(path);
	}

	/** A setting's current value as text, from its control. */
	display(path: Path): string {
		const v = this.get(path);
		return this.controls[pathKey(path)]?.format(v) ?? String(v);
	}

	elementId(path: Path): string {
		return this.idPrefix + pathKey(path);
	}

	/** Does a control match the search box? */
	matches(info: ControlInfo): boolean {
		const q = this.query.trim().toLowerCase();
		if (!q) return true;
		return [info.label, info.notes ?? '', pathKey(info.path)].some((s) => s.toLowerCase().includes(q));
	}

	/** How many controls (in one card, if given) match the search box. */
	countMatches(cardId?: string): number {
		let n = 0;
		for (const c of Object.values(this.controls)) if ((cardId === undefined || c.cardId === cardId) && this.matches(c)) n++;
		return n;
	}

	/** Is this card unfolded (by hand, by default, or because the search box is in use)? */
	isOpen(card: CardInfo): boolean {
		if (this.query.trim()) return true;
		return this.opened[card.id] ?? !card.collapsed;
	}
}

function newPresetId(): string {
	const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789';
	const bytes = crypto.getRandomValues(new Uint8Array(10));
	return 'user-' + Array.from(bytes, (b) => alphabet[b % alphabet.length]).join('');
}

export function toBase64Url(text: string): string {
	let binary = '';
	for (const byte of new TextEncoder().encode(text)) binary += String.fromCharCode(byte);
	return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

export function fromBase64Url(code: string): string {
	const b64 = code.replace(/-/g, '+').replace(/_/g, '/');
	const binary = atob(b64 + '='.repeat((4 - (b64.length % 4)) % 4));
	return new TextDecoder('utf-8', { fatal: true }).decode(Uint8Array.from(binary, (c) => c.charCodeAt(0)));
}
