import defaultPreset from '$data/presets/generation/root/default.json';
import techData from '$data/tech_data.json';
import notableData from '$data/notable_data.json';
import itemsJson from '$data/items.json';
import type { RootPreset, Scope, ScopePreset, SettingsModel, SettingsStore } from '$lib/settings/store.svelte';
import type { Tree } from '$lib/settings/tree';

// Generation settings data, bundled at build time from the randomizer's `data/` directory. Built-in
// presets come from `data/presets/generation/`, except skill presets, which are derived below.

/** Ranked easiest to hardest. Other difficulties (e.g. "Ignored") never take part in skill presets. */
export const DIFFICULTIES = ['Basic', 'Medium', 'Hard', 'Very Hard', 'Expert'];

/** Number of ordinary item locations: the most items the item pool can hold. */
export const ITEM_LOCATIONS = 149;

export interface Proficiency {
	id: string;
	name: string;
	levels: { level: number; difficulty: string }[];
}

export interface Tech {
	id: string;
	name: string;
	difficulty: string;
}

export interface Notable {
	world: string;
	room: string;
	roomName: string;
	id: string;
	name: string;
	difficulty: string;
}

export interface ItemRecord {
	name: string;
	min: number;
	/** The most the player can make use of; null for no limit. */
	max: number | null;
	category: string;
	tier: string;
}

export const items = itemsJson as ItemRecord[];

export const proficiencies: Proficiency[] = [];
export const tech: Tech[] = [];
for (const t of techData) {
	const id = String(t.tech_id);
	// Upstream techs with levels are proficiencies.
	if (t.level === undefined) {
		tech.push({ id, name: t.name, difficulty: t.difficulty });
		continue;
	}
	let p = proficiencies.find((x) => x.id === id);
	if (!p) {
		p = { id, name: t.name, levels: [] };
		proficiencies.push(p);
	}
	p.levels.push({ level: t.level, difficulty: t.difficulty });
}

export const notables: Notable[] = [];
for (const n of notableData) {
	const [world, room] = Object.entries(n.room_id)[0];
	notables.push({
		world: world.toLowerCase(),
		room: String(room),
		roomName: n.room_name,
		id: String(n.notable_id),
		name: n.name,
		difficulty: n.difficulty
	});
}

/**
 * One skill preset per difficulty: every technique and notable ranked at or below it enabled, and each
 * proficiency at its highest level ranked at or below it (its lowest level if none is). Unranked members
 * are disabled. The catalog builder derives the same presets for the backend (`crates/catalog_builder/src/presets.rs`).
 */
function buildSkillPresets(): ScopePreset[] {
	const presets: ScopePreset[] = [];
	for (let i = 0; i < DIFFICULTIES.length; i++) {
		const reached = (difficulty: string) => {
			const rank = DIFFICULTIES.indexOf(difficulty);
			return rank >= 0 && rank <= i;
		};
		const proficiencyLevels: Tree = {};
		for (const p of proficiencies) {
			proficiencyLevels[p.id] = p.levels[0].level;
			for (const l of p.levels) if (reached(l.difficulty)) proficiencyLevels[p.id] = l.level;
		}
		const techEnabled: Tree = {};
		for (const t of tech) techEnabled[t.id] = reached(t.difficulty);
		const notableEnabled: Tree = { overworld: {}, underworld: {} };
		for (const n of notables) {
			const rooms = notableEnabled[n.world] as Tree;
			if (!rooms[n.room]) rooms[n.room] = {};
			(rooms[n.room] as Tree)[n.id] = reached(n.difficulty);
		}
		const label = DIFFICULTIES[i];
		presets.push({
			id: label.toLowerCase().replace(/ /g, '_'),
			label,
			settings: { proficiencies: proficiencyLevels, tech: techEnabled, notables: notableEnabled }
		});
	}
	return presets;
}

export const skillScope: Scope = { id: 'skill', label: 'Skill assumptions', presets: buildSkillPresets() };

/**
 * A root preset file's settings as the store needs them: complete values, with each child scope's preset
 * selection taken out and its subtree filled from that preset.
 */
function readRootPreset(file: { id: string; label: string; settings: Tree }, scopes: Scope[]): RootPreset {
	const settings: Tree = { ...file.settings };
	const selections: Record<string, string> = {};
	for (const scope of scopes) {
		const id = (settings[scope.id] as Tree).preset as string;
		selections[scope.id] = id;
		settings[scope.id] = scope.presets.find((p) => p.id === id)!.settings;
	}
	return { id: file.id, label: file.label, settings, scopes: selections };
}

export const generationModel: SettingsModel = {
	rootPresets: [readRootPreset(defaultPreset as { id: string; label: string; settings: Tree }, [skillScope])],
	scopes: [skillScope]
};

/** How many items the item pool holds. */
export function countPoolItems(store: SettingsStore): number {
	let n = 0;
	for (const item of items) n += store.get(['item_pool', item.name]) as number;
	return n;
}
