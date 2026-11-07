export type Value = string | boolean | number;

interface Base {
	/** Unique key. Nested settings use dotted ids, e.g. "itemProgression.rate". */
	id: string;
	label: string;
	/** Longer explanation, shown in a tooltip / expander. */
	help?: string;
	/** Short descriptor shown in small text under the label. */
	notes?: string;
	/** Sub-heading shown above this row when it differs from the previous row's. */
	heading?: string;
	/** The same heading as a path, outermost first (e.g. ["Tech", "Hard"]). */
	headingPath?: string[];
	/** How the rows under this heading are laid out: one per line, or compact columns. */
	layout?: 'list' | 'grid' | 'compact';
	/** Small tag next to the label, e.g. a difficulty. */
	badge?: string;
}

export interface ChoiceOption {
	value: string;
	label: string;
	/** One-line description, shown under the control while this option is selected. */
	desc?: string;
	/** Optional CSS colour; shown as a small dot beside the label (handy for colour pickers). */
	swatch?: string;
}

export interface ChoiceSetting extends Base {
	kind: 'choice';
	options: ChoiceOption[];
	default: string;
	/**
	 * 'auto' picks: switch (2 opts), segmented (3-5), dropdown (6+).
	 * 'cards' forces selectable cards for options with real consequences.
	 */
	display?: 'auto' | 'segmented' | 'cards' | 'dropdown' | 'swatches';
	/** Sub-settings that replace the old gear-icon modal. Rendered in an inline expander. */
	details?: Setting[];
	/** If set, the details expander only appears while this setting has exactly this value. */
	detailsWhen?: string;
}

export interface ToggleSetting extends Base {
	kind: 'toggle';
	default: boolean;
	details?: Setting[];
	/** If set, the details expander only appears while the toggle has this value. */
	detailsWhen?: boolean;
}

export interface NumberSetting extends Base {
	kind: 'number';
	min: number;
	max: number;
	step: number;
	default: number;
	unit?: string;
	/** 'input' = number box. Default is a slider. */
	display?: 'slider' | 'input';
	/** A starting-item count: the id of the Item Pool count it is taken from (it may not exceed that count). */
	poolItem?: string;
	details?: Setting[];
}

/** A stepped slider whose steps have names (e.g. proficiency levels). Stored as the level number. */
export interface LevelSetting extends Base {
	kind: 'level';
	levels: { value: number; label: string }[];
	default: number;
}

export type Setting = ChoiceSetting | ToggleSetting | NumberSetting | LevelSetting;

/** A card. Its items are settings and nested sections, in display order. */
export interface Section {
	kind: 'section';
	id: string;
	title: string;
	icon?: string; // bootstrap-icons class suffix, e.g. "globe"
	/** The whole card can be folded away. */
	collapsible?: boolean;
	/** A collapsible card starts folded. */
	collapsed?: boolean;
	/** The card's own number settings fill this many item locations at most (fewer leaves locations empty). */
	pool?: { total: number };
	/** Built-in presets for this section. */
	presets: Preset[];
	/** Id of the built-in preset selected to begin with (and when a preset goes missing). Default: the first. */
	defaultPreset?: string;
	items: Item[];
}

/** An entry in a list of settings: a setting, or a section holding more. */
export type Item = Setting | Section;

/**
 * A preset for one section (or for everything: the root section).
 *
 * A built-in preset stands on its own: `values` are its section's own settings (only those differing from
 * the schema defaults) and `sections` picks a built-in preset for child sections (any left out get their
 * default). A user preset has a `base`: another preset of the same section, built-in or user, so user presets
 * form chains that end at a built-in one. Its `sections` picks presets (built-in or user) for sections inside
 * it, but only where they differ from what the base would pick, and its `values` are tweaks: settings
 * anywhere inside the section that differ from the base and those picks.
 */
export interface Preset {
	id: string;
	name: string;
	/** User presets only: the id of the preset of the same section it is built on. */
	base?: string;
	/** Section id -> preset id, for sections inside this one. */
	sections?: Record<string, string>;
	values: Record<string, Value>;
}
