import settingsDataJson from '$settings_data';
import type { Scope, SettingsModel, SettingsStore } from '$lib/settings/store.svelte';
import type { Tree } from '$lib/settings/tree';

// Generation settings data, prepared by the randomizer's catalog builder (see vite.config.ts): the
// built-in presets, with skill presets already derived, and what the page shows about each setting.

/** The shape of `settings_data.json`. */
interface SettingsData {
	/** Ranked easiest to hardest. */
	difficulties: string[];
	/** Notes are upstream descriptions, each joined into one paragraph. */
	proficiencies: {
		id: number;
		name: string;
		/** A level's note is null for level 0 (no proficiency). */
		levels: { level: number; difficulty: string; note: string | null }[];
	}[];
	techs: { id: number; name: string; difficulty: string; note: string }[];
	notables: {
		world: string;
		room: number;
		room_name: string;
		id: number;
		name: string;
		difficulty: string;
		note: string;
		detail_note: string | null;
	}[];
	items: ItemRecord[];
	/** The built-in presets of both settings contexts. */
	presets: {
		/** Identifies the settings processing code and data of this build. */
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
		customization: { id: string; label: string; settings: Tree }[];
	};
}

export interface ItemRecord {
	name: string;
	label: string;
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

const iconFiles = import.meta.glob<string>('../assets/items/*.png', {
	eager: true,
	import: 'default'
});
/** Item icon URLs, by file name without extension (an item name, or `<name>_<level>` for progressive items). */
const ICONS: Record<string, string> = {};
for (const [path, url] of Object.entries(iconFiles)) ICONS[path.slice(path.lastIndexOf('/') + 1, -4)] = url;

/** Icon URL of an icon name (as for `ICONS`), or of a progressive item's first level; null if there is none yet. */
export function getItemIcon(name: string): string | null {
	return ICONS[name] ?? ICONS[`${name}_1`] ?? null;
}

/**
 * Icon URL of a progressive item at a level (1 for its first upgrade), using its last icon for levels past
 * it; a non-progressive item's icon for any level. Null if there is none yet.
 */
export function getItemLevelIcon(name: string, level: number): string | null {
	if (!ICONS[`${name}_1`]) return ICONS[name] ?? null;
	for (let l = level; l >= 1; l--) if (ICONS[`${name}_${l}`]) return ICONS[`${name}_${l}`];
	return ICONS[`${name}_1`];
}

/** Display names of items, by item token. */
export const ITEM_LABELS: Record<string, string> = {};
for (const item of items) ITEM_LABELS[item.name] = item.label;

// Member IDs as they appear in setting paths.

export const proficiencies: {
	id: string;
	name: string;
	levels: { level: number; difficulty: string; note: string | null }[];
}[] = [];
for (const p of settingsData.proficiencies) proficiencies.push({ ...p, id: String(p.id) });

export const tech: { id: string; name: string; difficulty: string; note: string }[] = [];
for (const t of settingsData.techs) tech.push({ ...t, id: String(t.id) });

export const notables: {
	world: string;
	room: string;
	roomName: string;
	id: string;
	name: string;
	difficulty: string;
	note: string;
	detailNote: string | null;
}[] = [];
for (const n of settingsData.notables) {
	notables.push({
		world: n.world,
		room: String(n.room),
		roomName: n.room_name,
		id: String(n.id),
		name: n.name,
		difficulty: n.difficulty,
		note: n.note,
		detailNote: n.detail_note
	});
}

export const skillScope: Scope = {
	id: 'skill',
	label: 'Skill assumptions',
	presets: settingsData.presets.skill
};
export const startingStateScope: Scope = {
	id: 'starting_state',
	label: 'Starting state',
	presets: settingsData.presets.starting_state
};
export const itemPoolScope: Scope = {
	id: 'item_pool',
	label: 'Item pool',
	presets: settingsData.presets.item_pool
};

const rootPresets = [];
for (const p of settingsData.presets.root) {
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
	kind: 'generation',
	rootPresets,
	scopes: [skillScope, startingStateScope, itemPoolScope]
};

interface StartingTier {
	value: string;
	label: string;
	/** Pool items this tier stands for. */
	count: number;
}

const STARTING_TIERS: Record<string, StartingTier[]> = {
	sword: [
		{ value: 'none', label: 'None', count: 0 },
		{ value: 'fighter', label: 'Fighter', count: 1 },
		{ value: 'master', label: 'Master', count: 2 },
		{ value: 'tempered', label: 'Tempered', count: 3 },
		{ value: 'golden', label: 'Golden', count: 4 }
	],
	shield: [
		{ value: 'none', label: 'None', count: 0 },
		{ value: 'fighter', label: 'Fighter', count: 1 },
		{ value: 'fire', label: 'Fire', count: 2 },
		{ value: 'mirror', label: 'Mirror', count: 3 }
	],
	glove: [
		{ value: 'none', label: 'None', count: 0 },
		{ value: 'power', label: 'Power Glove', count: 1 },
		{ value: 'titans', label: "Titan's Mitt", count: 2 }
	],
	mail: [
		{ value: 'green', label: 'Green', count: 0 },
		{ value: 'blue', label: 'Blue', count: 1 },
		{ value: 'red', label: 'Red', count: 2 }
	],
	bow: [
		{ value: 'none', label: 'None', count: 0 },
		{ value: 'bow', label: 'Bow', count: 1 },
		{ value: 'silver', label: 'Silver Arrows', count: 2 }
	],
	boomerang: [
		{ value: 'none', label: 'None', count: 0 },
		{ value: 'blue', label: 'Blue', count: 1 },
		{ value: 'red', label: 'Red', count: 2 }
	],
	ocarina: [
		{ value: 'none', label: 'None', count: 0 },
		{ value: 'inactive', label: 'Inactive', count: 1 },
		{ value: 'active', label: 'Active', count: 1 }
	]
};

const defaultStartingEquipment = settingsData.presets.starting_state[0].settings
	.equipment as Tree;

/** Starting equipment, in item pool order. Unique items have no tiers and are switches. */
export const startingEquipment: { item: ItemRecord; tiers: StartingTier[] | null }[] = [];
for (const item of poolItems) {
	if (typeof defaultStartingEquipment[item.name] === 'boolean')
		startingEquipment.push({ item, tiers: null });
	else if (item.name in STARTING_TIERS)
		startingEquipment.push({ item, tiers: STARTING_TIERS[item.name] });
}

/** Pool items the starting equipment stands for, by item name; zero counts are left out. */
export function countStartingItems(store: SettingsStore): Record<string, number> {
	const counts: Record<string, number> = {};
	for (const { item, tiers } of startingEquipment) {
		const value = store.get(['starting_state', 'equipment', item.name]);
		if (tiers === null) {
			if (value === true) counts[item.name] = 1;
		} else {
			const count = tiers.find((tier) => tier.value === value)?.count ?? 0;
			if (count > 0) counts[item.name] = count;
		}
	}
	return counts;
}

/** Number of ordinary pickups placed after starting equipment is deducted (if enabled). */
export function countPoolItems(store: SettingsStore): number {
	let n = 0;
	for (const item of poolItems) n += store.get(['item_pool', 'items', item.name]) as number;
	if (store.get(['item_pool', 'deduct_starting_items'])) {
		for (const count of Object.values(countStartingItems(store))) n -= count;
	}
	return n;
}
