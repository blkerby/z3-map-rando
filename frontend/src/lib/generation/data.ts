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
	notables: {
		world: string;
		room: number;
		room_name: string;
		id: number;
		name: string;
		difficulty: string;
	}[];
	items: ItemRecord[];
	generation: {
		revision: string;
		/** The default first; settings are complete, with the selected scope presets applied. */
		root: {
			id: string;
			label: string;
			skill_preset: string;
			item_pool_preset: string;
			starting_state_preset: string;
			settings: Tree;
		}[];
		skill: { id: string; label: string; settings: Tree }[];
		item_pool: { id: string; label: string; settings: Tree }[];
		starting_state: { id: string; label: string; settings: Tree }[];
	};
	customization: { id: string; label: string; settings: Tree }[];
}

export interface ItemRecord {
	name: string;
	min: number;
	/** The most the player can make use of; null for no limit. */
	max: number | null;
	category: string;
	/** Placement tier; null for dungeon items and prizes. */
	tier: string | null;
}

export const settingsData = settingsDataJson as SettingsData;

/** Ranked easiest to hardest. Other difficulties (e.g. "Ignored") never take part in skill presets. */
export const DIFFICULTIES = settingsData.difficulties;

/** Number of ordinary item locations: the most items the item pool can hold. */
export const ITEM_LOCATIONS = 151;

export const items = settingsData.items;

/** Items the item pool can hold: all but dungeon items and prizes, which have their own placement. */
export const poolItems: ItemRecord[] = [];
for (const item of items)
	if (item.category !== 'dungeon' && item.category !== 'prize') poolItems.push(item);

// Member IDs as they appear in setting paths.

export const proficiencies: {
	id: string;
	name: string;
	levels: { level: number; difficulty: string }[];
}[] = [];
for (const p of settingsData.proficiencies) proficiencies.push({ ...p, id: String(p.id) });

export const tech: { id: string; name: string; difficulty: string }[] = [];
for (const t of settingsData.techs) tech.push({ ...t, id: String(t.id) });

export const notables: {
	world: string;
	room: string;
	roomName: string;
	id: string;
	name: string;
	difficulty: string;
}[] = [];
for (const n of settingsData.notables) {
	notables.push({
		world: n.world,
		room: String(n.room),
		roomName: n.room_name,
		id: String(n.id),
		name: n.name,
		difficulty: n.difficulty
	});
}

export const skillScope: Scope = {
	id: 'skill',
	label: 'Skill assumptions',
	presets: settingsData.generation.skill
};
export const startingStateScope: Scope = {
	id: 'starting_state',
	label: 'Starting state',
	presets: settingsData.generation.starting_state
};
export const itemPoolScope: Scope = {
	id: 'item_pool',
	label: 'Item pool',
	presets: settingsData.generation.item_pool
};

const rootPresets = [];
for (const p of settingsData.generation.root) {
	rootPresets.push({
		id: p.id,
		label: p.label,
		settings: p.settings,
		scopes: {
			skill: p.skill_preset,
			item_pool: p.item_pool_preset,
			starting_state: p.starting_state_preset
		}
	});
}

export const generationModel: SettingsModel = {
	rootPresets,
	scopes: [skillScope, startingStateScope, itemPoolScope],
	revision: settingsData.generation.revision
};

export const startingUniqueItems: ItemRecord[] = [];
const startingEquipment = settingsData.generation.starting_state[0].settings.equipment as Tree;
for (const item of poolItems)
	if (typeof startingEquipment[item.name] === 'boolean') startingUniqueItems.push(item);

export const startingEquipmentChoices = [
	{
		id: 'sword',
		options: [
			{ value: 'none', label: 'None', count: 0 },
			{ value: 'fighter', label: 'Fighter', count: 1 },
			{ value: 'master', label: 'Master', count: 2 },
			{ value: 'tempered', label: 'Tempered', count: 3 },
			{ value: 'golden', label: 'Golden', count: 4 }
		]
	},
	{
		id: 'shield',
		options: [
			{ value: 'none', label: 'None', count: 0 },
			{ value: 'fighter', label: 'Fighter', count: 1 },
			{ value: 'fire', label: 'Fire', count: 2 },
			{ value: 'mirror', label: 'Mirror', count: 3 }
		]
	},
	{
		id: 'glove',
		options: [
			{ value: 'none', label: 'None', count: 0 },
			{ value: 'power', label: 'Power Glove', count: 1 },
			{ value: 'titans', label: "Titan's Mitt", count: 2 }
		]
	},
	{
		id: 'mail',
		options: [
			{ value: 'green', label: 'Green', count: 0 },
			{ value: 'blue', label: 'Blue', count: 1 },
			{ value: 'red', label: 'Red', count: 2 }
		]
	},
	{
		id: 'bow',
		options: [
			{ value: 'none', label: 'None', count: 0 },
			{ value: 'bow', label: 'Bow', count: 1 },
			{ value: 'silver', label: 'Silver Arrows', count: 2 }
		]
	},
	{
		id: 'boomerang',
		options: [
			{ value: 'none', label: 'None', count: 0 },
			{ value: 'blue', label: 'Blue', count: 1 },
			{ value: 'red', label: 'Red', count: 2 }
		]
	},
	{
		id: 'ocarina',
		options: [
			{ value: 'none', label: 'None', count: 0 },
			{ value: 'inactive', label: 'Inactive', count: 1 },
			{ value: 'active', label: 'Active', count: 1 }
		]
	}
];

/** Number of ordinary pickups placed after equipment-only deduction. */
export function countPoolItems(store: SettingsStore): number {
	let n = 0;
	for (const item of poolItems) n += store.get(['item_pool', 'items', item.name]) as number;
	if (store.get(['item_pool', 'deduct_starting_items'])) {
		for (const item of startingUniqueItems)
			if (store.get(['starting_state', 'equipment', item.name])) n--;
		for (const choice of startingEquipmentChoices) {
			const value = store.get(['starting_state', 'equipment', choice.id]);
			n -= choice.options.find((option) => option.value === value)?.count ?? 0;
		}
		n -= store.get(['starting_state', 'equipment', 'bottles']) as number;
	}
	return n;
}
