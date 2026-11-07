import type { Preset } from './types';
import { newPresetId } from './presetIO';

/** What a preset holds besides its id and name. */
export type PresetContent = Pick<Preset, 'values' | 'groupPresets'>;

/** The presets behind one preset picker: the built-in ones, the player's own, and which is selected. */
export class PresetList {
	readonly builtIn: Preset[];
	/** Presets the player saved or imported. */
	userPresets = $state<Preset[]>([]);
	selectedId = $state('');

	constructor(builtIn: Preset[]) {
		this.builtIn = builtIn;
		this.selectedId = this.defaultId;
	}

	/** The first built-in preset: selected to begin with, and whenever the selected preset goes away. */
	get defaultId(): string {
		return this.builtIn[0].id;
	}

	/** Built-in presets followed by the player's own. */
	get all(): Preset[] {
		return [...this.builtIn, ...this.userPresets];
	}

	get selected(): Preset {
		return this.find(this.selectedId) ?? this.builtIn[0];
	}

	find(id: unknown): Preset | undefined {
		return this.all.find((p) => p.id === id);
	}

	isUser(id: string): boolean {
		return this.userPresets.some((p) => p.id === id);
	}

	/** Select a preset by id, falling back to the default one if there is no such preset. */
	select(id: unknown) {
		this.selectedId = this.find(id)?.id ?? this.defaultId;
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

	delete(id: string) {
		this.userPresets = this.userPresets.filter((p) => p.id !== id);
		if (this.selectedId === id) this.selectedId = this.defaultId;
	}
}
