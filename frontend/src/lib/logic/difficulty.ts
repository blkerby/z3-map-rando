import logicCatalogUrl from '$build/logic_catalog.bin?url';
import { loadBrowser } from '$lib/browser';
import { SETTINGS_PRESETS_KEY, settingsData } from '$lib/generation/data';
import { fetchBytes } from '$lib/seed';
import type { SettingsStore } from '$lib/settings/store.svelte';
import { getAt, isTree, leafPaths, type Tree } from '$lib/settings/tree';

/**
 * Skill settings to list only the strats they pass with: a built-in skill preset, or a root preset that tweaks one.
 * Root presets without skill tweaks are left out, as they take the same settings as a listed option.
 */
export interface DifficultyOption {
	/** `skill:<id>` or `root:<id>` for built-in presets, or a player's preset ID. */
	id: string;
	label: string;
	/** Absent for the player's presets, which are resolved when chosen (see `resolvePresetSkill`). */
	skill?: Tree;
}

/** The built-in skill presets, then the built-in root presets that tweak their skill preset. */
export function listBuiltInOptions(): DifficultyOption[] {
	const options: DifficultyOption[] = [];
	for (const preset of settingsData.presets.skill) {
		options.push({ id: `skill:${preset.id}`, label: preset.label, skill: preset.settings });
	}
	for (const preset of settingsData.presets.root) {
		const skill = preset.settings.skill as Tree;
		const base = settingsData.presets.skill.find((p) => p.id === preset.skill_preset)!.settings;
		if (!isSameSkill(base, skill)) {
			options.push({ id: `root:${preset.id}`, label: preset.label, skill });
		}
	}
	return options;
}

/** The built-in option with these skill settings, if any. */
export function findBuiltInOption(skill: Tree): DifficultyOption | undefined {
	return listBuiltInOptions().find((option) => isSameSkill(option.skill!, skill));
}

/** Whether two complete sets of skill settings are equal. */
function isSameSkill(a: Tree, b: Tree): boolean {
	for (const path of leafPaths(a)) {
		if (getAt(a, path) !== getAt(b, path)) return false;
	}
	return true;
}

/** Take the player's generation presets from this browser, kept as stored until one is chosen. */
export function loadUserPresets(store: SettingsStore) {
	let stored: unknown = null;
	try {
		stored = JSON.parse(localStorage.getItem(SETTINGS_PRESETS_KEY) ?? 'null');
	} catch {
		// Storage unavailable or unreadable; list no presets of the player's.
	}
	store.loadUserPresets(stored);
}

/** The player's presets that tweak skill settings. */
export function listUserOptions(store: SettingsStore): DifficultyOption[] {
	const options: DifficultyOption[] = [];
	for (const preset of store.userPresets) {
		const skill = getAt(preset.document, ['skill']);
		if (isTree(skill) && Object.keys(skill).some((key) => key !== 'preset')) {
			options.push({ id: preset.id, label: preset.name });
		}
	}
	return options;
}

/**
 * The skill settings of one of the player's presets. One stored under another settings revision is processed on
 * the server first and saved back, adding what processing changed to the store's notices. Returns null for a preset
 * processing can't read. Throws when the request fails.
 */
export async function resolvePresetSkill(store: SettingsStore, id: string): Promise<Tree | null> {
	if (store.isStale(id)) {
		if ((await store.processPreset(id)) === 'unrecoverable') return null;
		store.saveUserPresets(SETTINGS_PRESETS_KEY);
	}
	const preset = store.userPreset(id)!;
	return store.resolve(preset.document, {}, true).skill as Tree;
}

let session:
	Promise<{ check_room_strats(roomId: number, skill: Uint8Array): Uint32Array }> | undefined;

/** IDs of the room's strats that pass with the skill settings, with every item owned. */
export async function checkRoomStrats(roomId: number, skill: Tree): Promise<Set<number>> {
	session ??= Promise.all([loadBrowser(), fetchBytes(logicCatalogUrl)])
		.then(([browser, catalog]) => new browser.LogicSession(catalog))
		.catch((error) => {
			session = undefined;
			throw error;
		});
	const ids = (await session).check_room_strats(
		roomId,
		new TextEncoder().encode(JSON.stringify(skill))
	);
	return new Set(ids);
}
