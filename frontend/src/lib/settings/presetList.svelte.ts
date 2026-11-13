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

	/**
	 * A preset and the presets it is built on, the built-in one first. A base that is missing, or that would
	 * make a loop (from a damaged file), ends the chain at the default preset.
	 */
	chain(preset: Preset): Preset[] {
		const out = [preset];
		let p = preset;
		while (p.base !== undefined) {
			const next = this.find(p.base);
			p = next && !out.includes(next) ? next : this.defaultPreset;
			out.unshift(p);
		}
		return out;
	}

	/** The built-in preset at the bottom of a preset's chain: itself if built-in. */
	builtInBehind(preset: Preset): Preset {
		return this.chain(preset)[0];
	}

	/** The preset a preset is built on: itself if built-in. A new preset saved from it gets this base. */
	baseOf(preset: Preset): Preset {
		const chain = this.chain(preset);
		return chain[Math.max(0, chain.length - 2)];
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
