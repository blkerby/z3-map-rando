import type { Preset } from './types';
import { newPresetId } from './presetIO';

/** What a preset holds besides its id and name. */
export type PresetContent = Pick<Preset, 'base' | 'sections' | 'values'>;

/** The presets behind one preset picker: the built-in ones, the player's own, and which is selected. */
export class PresetList {
	readonly builtIn: Preset[];
	/** The built-in preset selected to begin with, and whenever the selected preset goes away. */
	readonly defaultId: string;
	/** Presets the player saved or imported. */
	userPresets = $state<Preset[]>([]);
	/**
	 * The preset picked for this section, or null when it takes whatever the presets around it choose
	 * (see SettingsStore.selectedPreset). The root section always has one.
	 */
	selectedId = $state<string | null>(null);

	constructor(builtIn: Preset[], defaultId?: string) {
		this.builtIn = builtIn;
		this.defaultId = builtIn.some((p) => p.id === defaultId) ? defaultId! : builtIn[0].id;
	}

	/** Built-in presets followed by the player's own. */
	get all(): Preset[] {
		return [...this.builtIn, ...this.userPresets];
	}

	get defaultPreset(): Preset {
		return this.builtIn.find((p) => p.id === this.defaultId)!;
	}

	find(id: unknown): Preset | undefined {
		return this.all.find((p) => p.id === id);
	}

	/** The built-in preset with this id, or the default one. */
	builtInOrDefault(id: unknown): Preset {
		return this.builtIn.find((p) => p.id === id) ?? this.defaultPreset;
	}

	/** The built-in preset a preset stands on: itself if built-in, otherwise its base. */
	builtInBehind(preset: Preset): Preset {
		return preset.base === undefined ? preset : this.builtInOrDefault(preset.base);
	}

	isUser(id: string): boolean {
		return this.userPresets.some((p) => p.id === id);
	}

	add(name: string, content: PresetContent): Preset {
		const preset: Preset = { id: newPresetId(), name, ...content };
		this.userPresets.push(preset);
		return preset;
	}

	update(id: string, content: PresetContent) {
		const preset = this.userPresets.find((p) => p.id === id);
		if (preset) Object.assign(preset, content);
	}

	rename(id: string, name: string) {
		const preset = this.userPresets.find((p) => p.id === id);
		if (preset) preset.name = name;
	}
}
