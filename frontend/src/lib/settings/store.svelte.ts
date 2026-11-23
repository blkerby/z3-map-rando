import { untrack } from 'svelte';
import { FORMAT_VERSION, REVISION, processDocument, type SettingsIssue, type SettingsKind } from './processing';
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
 *
 * Every document the store holds is either a result of server processing or was stored by this browser
 * under the current settings revision (see `processing.ts`), so it is read without further checks.
 */

export interface ScopePreset {
	id: string;
	label: string;
	/** Complete values of the scope's subtree. */
	settings: Tree;
}

export interface RootPreset {
	id: string;
	label: string;
	/** Complete values of every setting, including authored tweaks to selected child presets. */
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
	kind: SettingsKind;
	/** Built-in root presets; the first is the default. */
	rootPresets: RootPreset[];
	scopes: Scope[];
}

export interface UserPreset {
	id: string;
	name: string;
	/**
	 * A settings document (without `format_version` once processed). Stored, it records its format version;
	 * until processed, it is kept as stored.
	 */
	document: Tree;
	/** The settings revision the document was processed under; another one needs processing before use. */
	revision: string | null;
}

/** The stored editor state: the current settings and selections. */
export interface Draft {
	selected: string;
	picks: Record<string, string | null>;
	/** A settings document. As stored, it records its `format_version`, which loading ignores. */
	document: Tree;
}

/** What server processing changed in one document, shown to the player once. */
export interface NoticeGroup {
	/** The document, e.g. "Preset “Mine”". */
	source: string;
	issues: SettingsIssue[];
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
	/** A generated game's settings document, which then takes the place of the selected preset. */
	seedDocument = $state<Tree | null>(null);
	/** Child scope id -> preset picked by hand, or null when it takes what the root preset chooses. */
	scopePicks = $state<Record<string, string | null>>({});
	userPresets = $state<UserPreset[]>([]);
	/** Why the settings are read-only (shown when a locked control is pressed), or null when editable. */
	lockReason = $state<string | null>(null);
	get locked() {
		return this.lockReason !== null;
	}
	/** Cards folded or unfolded by hand, by card id. */
	opened = $state<Record<string, boolean>>({});
	/** Set when the player's presets couldn't be saved in this browser. */
	storageFailed = $state(false);
	/** Changes server processing made to loaded documents, until dismissed. */
	notices = $state<NoticeGroup[]>([]);
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

	/** The selected root preset as a settings document (the seed's document, if one is loaded). */
	get selectedDocument(): Tree {
		if (this.seedDocument) return this.seedDocument;
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
			if (pick || getAt(doc, [scope.id, 'preset']) !== undefined || !tweaks) {
				out[scope.id] = cloneTree(preset.settings);
			}
		}
		if (tweaks) {
			for (const path of this.leaves) {
				if (picked.some((p) => startsWith(path, p))) continue;
				const v = getAt(doc, path);
				if (v !== undefined) setAt(out, path, v as Value);
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

	/** Does the selected document choose a different scope preset from its built-in base? */
	isScopePresetTweaked(scope: Scope): boolean {
		return this.impliedScopePreset(scope).id !== this.chosenFor({ preset: this.basePreset.id }, scope);
	}

	/** Does a scope that isn't picked by hand take something other than the built-in base would give it? */
	isScopeTweaked(scope: Scope): boolean {
		if (this.isScopePicked(scope)) return false;
		if (this.scopeTweakCount(scope) > 0 || this.isScopePresetTweaked(scope)) return true;
		for (const path of this.leaves) {
			if (path[0] === scope.id && getAt(this.layers.inherited, path) !== getAt(this.basePreset.settings, path)) return true;
		}
		return false;
	}

	/** What a scope takes when it isn't picked by hand, e.g. "Hard + 2 tweaks" (short: "Hard + 2"). */
	inheritedScopeLabel(scope: Scope, short = false): string {
		const n = this.inheritedTweakCount(scope);
		const name = this.impliedScopePreset(scope).label;
		if (!n) return name;
		return short ? `${name} + ${n}` : `${name} + ${n} tweak${n === 1 ? '' : 's'}`;
	}

	/** What a scope takes: "Expert" when picked by hand, else e.g. "Hard + 2 tweaks" (short: "Hard + 2"). */
	scopeLabel(scope: Scope, short = false): string {
		return this.isScopePicked(scope) ? this.selectedScopePreset(scope).label : this.inheritedScopeLabel(scope, short);
	}

	/** How many settings in a scope differ from what the selected presets give (not counting its pick). */
	countScopeChanges(scope: Scope): number {
		let n = 0;
		for (const p of this.leaves) if (p[0] === scope.id && this.isChanged(p)) n++;
		return n;
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
			if (id !== this.chosenFor({ preset: base.id }, scope) || this.isScopePicked(scope)
				|| getAt(this.selectedDocument, [scope.id, 'preset']) !== undefined) {
				doc[scope.id] = { preset: id };
			}
		}
		const given = this.resolve(doc, {}, true);
		for (const path of this.leaves) {
			const v = getAt(this.values, path) as Value;
			if (v !== getAt(given, path)) setAt(doc, path, v);
		}
		return doc;
	}

	/**
	 * Read a processed settings document into the form the store holds: its preset selections and the values
	 * at the settings' paths.
	 */
	readDocument(raw: unknown): Tree {
		const doc: Tree = { preset: this.builtIn(getAt(raw, ['preset']))?.id ?? this.model.rootPresets[0].id };
		for (const scope of this.model.scopes) {
			const preset = this.scopePreset(scope, getAt(raw, [scope.id, 'preset']));
			if (preset) doc[scope.id] = { preset: preset.id };
		}
		for (const path of this.leaves) {
			const v = getAt(raw, path);
			if (v !== undefined) setAt(doc, path, v as Value);
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
			const own = getAt(doc, [scope.id, 'preset']) !== undefined;
			this.scopePicks[scope.id] = id !== this.chosenFor({ preset: this.selectedId }, scope)
				|| (own && this.inheritedTweakCount(scope) > 0) ? id : null;
		}
		this.values = this.resolve(doc, {}, true);
	}

	/**
	 * Take a generated game's settings document as the selected preset, so its differences from its built-in
	 * preset show as tweaks rather than unsaved changes.
	 */
	loadSeed(doc: Tree) {
		this.seedDocument = doc;
		this.selectedId = this.rootOf(doc).id;
		for (const scope of this.model.scopes) this.scopePicks[scope.id] = null;
		this.values = this.resolve(doc, {}, true);
	}

	/** Stop treating the seed's document as the selected preset: its scope selections become picks and its tweaks unsaved changes. */
	releaseSeed() {
		const doc = this.capture();
		this.seedDocument = null;
		this.loadDocument(doc);
	}

	/** Capture the public settings document. */
	captureDocument(): Tree {
		return { format_version: FORMAT_VERSION, ...this.capture() };
	}

	/**
	 * The current settings and selections, for keeping a draft in this browser, with the revision its
	 * document was processed under (null when it needs processing before use). The stored document records
	 * its format version, so it can be migrated later.
	 */
	serializeDraft(revision: string | null = REVISION): string {
		const draft: Draft = { selected: this.selectedId, picks: this.scopePicks, document: this.captureDocument() };
		return JSON.stringify({ revision, ...draft });
	}

	/** Take a draft's settings and selections. Its document and its selected user preset must be processed. */
	loadDraft(draft: Draft) {
		this.loadDocument(this.readDocument(draft.document));
		this.switchPreset(draft.selected);
		for (const scope of this.model.scopes) {
			this.scopePicks[scope.id] = this.scopePreset(scope, draft.picks[scope.id])?.id ?? null;
		}
	}

	// ---- User presets -----------------------------------------------------------

	/** Would saving into the selected preset (one of the player's own) change it? */
	get canUpdate(): boolean {
		const preset = this.userPreset(this.selectedId);
		return preset !== undefined && JSON.stringify(this.capture()) !== JSON.stringify(preset.document);
	}

	/** Add one of the player's presets, holding a processed document. */
	addPreset(name: string, document: Tree): UserPreset {
		const preset = { id: newPresetId(), name, document, revision: REVISION };
		this.userPresets.push(preset);
		return this.userPresets[this.userPresets.length - 1];
	}

	/** Save the current settings into one of the player's presets and select it. */
	updatePreset(id: string) {
		const preset = this.userPreset(id);
		if (!preset) return;
		preset.document = this.capture();
		preset.revision = REVISION;
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

	/** Does one of the player's presets need processing before use? */
	isStale(id: string): boolean {
		const preset = this.userPreset(id);
		return preset !== undefined && preset.revision !== REVISION;
	}

	/**
	 * Process one of the player's presets on the server and keep the result, so it can be used. Its notices
	 * are shown with the others. Throws when the request fails (see `processDocument`).
	 */
	async processPreset(id: string): Promise<'processed' | 'unrecoverable'> {
		const preset = this.userPreset(id);
		if (!preset) return 'processed';
		const result = await processDocument(this.model.kind, preset.document);
		if (result.status === 'unrecoverable') return 'unrecoverable';
		preset.document = this.readDocument(result.document);
		preset.revision = REVISION;
		if (result.notices.length) this.notices.push({ source: `Preset “${preset.name}”`, issues: result.notices });
		return 'processed';
	}

	/** Take stored presets, as saved by `saveUserPresets`. */
	loadUserPresets(data: unknown) {
		this.userPresets = this.readPresets(getAt(data, ['presets']));
	}

	/**
	 * Store the player's presets. Processed documents record the current format version, so they can be
	 * migrated later; presets not yet processed are kept as they were stored.
	 */
	saveUserPresets(key: string) {
		const presets: UserPreset[] = [];
		for (const preset of this.userPresets) {
			if (preset.revision === REVISION) presets.push({ ...preset, document: { format_version: FORMAT_VERSION, ...preset.document } });
			else presets.push(preset);
		}
		try {
			localStorage.setItem(key, JSON.stringify({ presets }));
			this.storageFailed = false;
		} catch {
			this.storageFailed = true;
		}
	}

	/** Read stored presets back, discarding anything malformed. Documents are kept as stored until processed. */
	private readPresets(raw: unknown): UserPreset[] {
		const out: UserPreset[] = [];
		if (!Array.isArray(raw)) return out;
		for (const item of raw) {
			if (!isTree(item) || typeof item.name !== 'string' || !isTree(item.document)) continue;
			const id = typeof item.id === 'string' && /^user-[a-z0-9]{4,32}$/.test(item.id) && !out.some((p) => p.id === item.id) ? item.id : newPresetId();
			const revision = typeof item.revision === 'string' ? item.revision : null;
			out.push({ id, name: item.name, document: revision === REVISION ? this.readDocument(item.document) : item.document, revision });
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
