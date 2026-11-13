import settingsJson from '$data/settings.json';
import techData from '$data/tech_data.json';
import notableData from '$data/notable_data.json';
import itemsJson from '$data/items.json';
import type { Scope, ScopePreset, SettingsModel, SettingsStore } from '$lib/settings/store.svelte';
import { cloneTree, type Tree } from '$lib/settings/tree';

// Generation settings data, bundled at build time from the randomizer's `data/` directory.

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

export const defaults = settingsJson as Tree;
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
 * proficiency at its highest level ranked at or below it. Unranked members keep their default.
 */
function buildSkillPresets(): ScopePreset[] {
	const presets: ScopePreset[] = [];
	for (let i = 0; i < DIFFICULTIES.length; i++) {
		const settings = cloneTree(defaults.skill as Tree);
		const reached = (difficulty: string) => {
			const rank = DIFFICULTIES.indexOf(difficulty);
			return rank < 0 ? undefined : rank <= i;
		};
		for (const p of proficiencies) {
			for (const l of p.levels) if (reached(l.difficulty)) (settings.proficiencies as Tree)[p.id] = l.level;
		}
		for (const t of tech) {
			const on = reached(t.difficulty);
			if (on !== undefined) (settings.tech as Tree)[t.id] = on;
		}
		for (const n of notables) {
			const on = reached(n.difficulty);
			if (on !== undefined) (((settings.notables as Tree)[n.world] as Tree)[n.room] as Tree)[n.id] = on;
		}
		const label = DIFFICULTIES[i];
		presets.push({ id: label.toLowerCase().replace(/ /g, '_'), label, settings });
	}
	return presets;
}

export const skillScope: Scope = { id: 'skill', label: 'Skill assumptions', presets: buildSkillPresets() };

export const generationModel: SettingsModel = {
	rootPresets: [{ id: 'default', label: 'Default', settings: defaults, scopes: { skill: 'basic' } }],
	scopes: [skillScope]
};

/** How many items the item pool holds. */
export function countPoolItems(store: SettingsStore): number {
	let n = 0;
	for (const item of items) n += store.get(['item_pool', item.name]) as number;
	return n;
}
