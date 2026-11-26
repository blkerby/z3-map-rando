import { generationModel } from '$lib/generation/data';
import { SettingsStore } from '$lib/settings/store.svelte';
import type { Tree } from '$lib/settings/tree';
import {
	findBuiltInOption,
	listBuiltInOptions,
	listUserOptions,
	loadUserPresets,
	resolvePresetSkill
} from './difficulty';

const DIFFICULTY_KEY = 'logic-difficulty';

/** Shared difficulty filter state for Logic room and tech pages. */
export class DifficultyFilter {
	/** The player's generation presets, loaded once mounted. */
	readonly presets = new SettingsStore(generationModel, 'logic-');
	readonly options = $derived([...listBuiltInOptions(), ...listUserOptions(this.presets)]);
	/** An option ID, or '' for strats of any difficulty. */
	selectedId = $state('');
	readonly selected = $derived(this.options.find((option) => option.id === this.selectedId));
	error = $state<string | null>(null);

	load() {
		let stored = '';
		try {
			stored = localStorage.getItem(DIFFICULTY_KEY) ?? '';
		} catch {
			// Storage unavailable; list strats of any difficulty.
		}
		loadUserPresets(this.presets);
		if (this.options.some((option) => option.id === stored)) {
			this.selectedId = stored;
		}
	}

	save() {
		try {
			localStorage.setItem(DIFFICULTY_KEY, this.selectedId);
		} catch {
			// Storage unavailable; the choice lasts for this page only.
		}
	}

	/**
	 * Resolve the selected option's skill settings, or null if none is selected, unreadable, or
	 * replaced by a built-in option after stale-preset processing.
	 */
	async resolveSkill(): Promise<Tree | null> {
		const option = this.selected;
		this.error = null;
		if (!option) return null;
		const skill = option.skill ?? (await resolvePresetSkill(this.presets, option.id));
		const listed = listUserOptions(this.presets).some((o) => o.id === option.id);
		if (skill !== null && !option.skill && !listed && this.selectedId === option.id) {
			// Processing removed the preset's skill tweaks, so it takes a built-in option's settings.
			this.selectedId = findBuiltInOption(skill)?.id ?? '';
			this.save();
			return null;
		}
		if (skill === null) {
			this.error = `Preset “${option.label}” couldn't be read.`;
			return null;
		}
		return skill;
	}
}
