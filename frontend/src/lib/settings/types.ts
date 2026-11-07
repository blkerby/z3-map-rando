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
	/** 'input' = number box (a 0-or-1 range becomes a No/Yes toggle). Default is a slider. */
	display?: 'slider' | 'input';
	details?: Setting[];
}

/** A stepped slider whose steps have names (e.g. proficiency levels). Stored as the level number. */
export interface LevelSetting extends Base {
	kind: 'level';
	levels: { value: number; label: string }[];
	default: number;
}

/**
 * A count the player does not set: it takes whatever item slots the other counts leave over (see
 * Group.pool). The store draws the split at random and keeps it in `values`, so it is saved, put in
 * share links and sent to the server like any other value, but it never counts as a "change".
 */
export interface CalculatedSetting extends Base {
	kind: 'calculated';
	/** Share of the leftover slots this item tends to get (the vanilla amount). */
	weight: number;
	/** Fewest it can end up with. */
	min: number;
	max?: number;
}

export type Setting = ChoiceSetting | ToggleSetting | NumberSetting | LevelSetting | CalculatedSetting;
/** Settings that hold a value the player can change. */
export type ValueSetting = Exclude<Setting, CalculatedSetting>;

export interface Group {
	id: string;
	title: string;
	icon?: string; // bootstrap-icons class suffix, e.g. "globe"
	/** The whole card can be folded away. */
	collapsible?: boolean;
	/** A collapsible card starts folded. */
	collapsed?: boolean;
	/**
	 * A fixed number of slots shared by the card's number settings. Whatever they leave over goes to
	 * the card's calculated settings.
	 */
	pool?: { total: number };
	/** The card has its own preset picker with these built-in presets (the first is the default). */
	presets?: Preset[];
	settings: Setting[];
}

export interface Preset {
	id: string;
	name: string;
	/** Built-in presets: only the values that differ from schema defaults. Saved presets: a full snapshot. */
	values: Record<string, Value>;
	/** Full-settings presets only: the preset selected in each card that has its own (card id -> preset id). */
	groupPresets?: Record<string, string>;
}
