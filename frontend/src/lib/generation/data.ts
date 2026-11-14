import settingsDataJson from '$settings_data';
import type { Scope, SettingsModel, SettingsStore } from '$lib/settings/store.svelte';
import type { Tree } from '$lib/settings/tree';

// Generation settings data, prepared by the randomizer's catalog builder (see vite.config.ts): the
// built-in presets, with skill presets already derived, and what the page shows about each setting.

/** The shape of `settings_data.json`. */
interface SettingsData {
	/** Ranked easiest to hardest. */
	difficulties: string[];
	proficiencies: { id: number; name: string; levels: { level: number; difficulty: string }[] }[];
	techs: { id: number; name: string; difficulty: string }[];
	notables: { world: string; room: number; room_name: string; id: number; name: string; difficulty: string }[];
	items: ItemRecord[];
	generation: {
		/** The default first; settings are complete, with the selected skill preset applied. */
		root: { id: string; label: string; skill_preset: string; settings: Tree }[];
		skill: { id: string; label: string; settings: Tree }[];
	};
	customization: { id: string; label: string; settings: Tree }[];
}

export interface ItemRecord {
	name: string;
	min: number;
	/** The most the player can make use of; null for no limit. */
	max: number | null;
	category: string;
	tier: string;
}

export const settingsData = settingsDataJson as SettingsData;

/** Ranked easiest to hardest. Other difficulties (e.g. "Ignored") never take part in skill presets. */
export const DIFFICULTIES = settingsData.difficulties;

/** Number of ordinary item locations: the most items the item pool can hold. */
export const ITEM_LOCATIONS = 149;

export const items = settingsData.items;

// Member IDs as they appear in setting paths.

export const proficiencies: { id: string; name: string; levels: { level: number; difficulty: string }[] }[] = [];
for (const p of settingsData.proficiencies) proficiencies.push({ ...p, id: String(p.id) });

export const tech: { id: string; name: string; difficulty: string }[] = [];
for (const t of settingsData.techs) tech.push({ ...t, id: String(t.id) });

export const notables: { world: string; room: string; roomName: string; id: string; name: string; difficulty: string }[] = [];
for (const n of settingsData.notables) {
	notables.push({ world: n.world, room: String(n.room), roomName: n.room_name, id: String(n.id), name: n.name, difficulty: n.difficulty });
}

export const skillScope: Scope = { id: 'skill', label: 'Skill assumptions', presets: settingsData.generation.skill };

const rootPresets = [];
for (const p of settingsData.generation.root) {
	rootPresets.push({ id: p.id, label: p.label, settings: p.settings, scopes: { skill: p.skill_preset } });
}

export const generationModel: SettingsModel = { rootPresets, scopes: [skillScope] };

/** How many items the item pool holds. */
export function countPoolItems(store: SettingsStore): number {
	let n = 0;
	for (const item of items) n += store.get(['item_pool', item.name]) as number;
	return n;
}
