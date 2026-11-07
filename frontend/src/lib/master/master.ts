/**
 * Types and small helpers for master_data.json: the single file that describes every
 * generate-page setting. The server (and generator) owns it; the page renders from it.
 *
 * Rules the client follows:
 *  - Containers (section, group) only organize. Leaves carry a `type` that picks the control.
 *  - A node whose `type` the client doesn't know is skipped, so a newer server can't break an older page.
 *  - Original community fields (tech_id, name, room_id, room_name, notable_id, difficulty, level)
 *    are kept verbatim. `label`/`desc` are optional presentation fields; without a label the
 *    client shows the community name exactly as written (canBonkBoost stays canBonkBoost).
 */

export type Value = string | boolean | number;

/** e.g. { "Underworld": 60 }. Serde-style tagged id; keep whatever the server sends. */
export type RoomId = Record<string, number>;

interface NodeBase {
	/** Optional display text. Without one, the page shows `name` unchanged. */
	label?: string;
	/** A short descriptor shown in small text under the setting's name. Searchable. */
	notes?: string;
	/** Optional longer explanation, shown as a hover tooltip (the info icon). */
	desc?: string;
	/** Not rendered, but its `default` is still sent to the server as a fixed value. */
	hidden?: boolean;
}

// ---- Containers --------------------------------------------------------------------

export interface SectionNode extends NodeBase {
	type: 'section';
	/** Stable id, used for page navigation. */
	id: string;
	title: string;
	icon?: string; // bootstrap-icons suffix
	/** Compact rows in columns ('grid'), compact rows in one column ('compact'), or full rows one per line ('list', default). Inherited by groups below unless they set their own. */
	layout?: 'list' | 'grid' | 'compact';
	/** The whole section can be folded away (a click on its title). */
	collapsible?: boolean;
	/** A collapsible section starts folded. */
	collapsed?: boolean;
	children: MasterNode[];
}

export interface GroupNode extends NodeBase {
	type: 'group';
	title: string;
	/** Set on per-room groups so the group can link to its logic page later. */
	room_id?: RoomId;
	/** Overrides the layout inherited from the section or parent group. */
	layout?: 'list' | 'grid' | 'compact';
	/**
	 * Regroup by difficulty: one heading per difficulty in ladder order, and under each one the
	 * leaves of that difficulty. If the group's children are themselves groups (e.g. "Tech" and
	 * "Notables"), each shows up as a sub-heading under every difficulty it has leaves for.
	 * This group's own title is not shown. Changing a node's `difficulty` moves it to the right
	 * heading with no other edit.
	 */
	groupBy?: 'difficulty';
	/**
	 * With `groupBy`: set to false to leave out the child groups' titles, so under each difficulty
	 * their rows run together in one list/grid (in child order). Default true. The child groups stay
	 * in the file, so the tree still says which are tech and which are notables.
	 */
	subheadings?: boolean;
	children: MasterNode[];
}

// ---- Leaves ------------------------------------------------------------------------

export interface LevelNode extends NodeBase {
	type: 'level'; // stepped slider
	tech_id: number;
	name: string;
	/** The ladder of levels. Slider min/max/step labels come from here, not from hard-coded numbers. */
	levels: {
		level: number;
		/** Community difficulty. Drives presets. */
		difficulty: string;
		/** Text shown on the slider instead of `difficulty` (e.g. "None" for dark level 0). */
		label?: string;
	}[];
	/** Starting level (must be one of `levels`). */
	default: number;
}

export interface ToggleNode extends NodeBase {
	type: 'toggle';
	name: string;
	/** Present on tech and notables. Drives difficulty presets. */
	difficulty?: string;
	tech_id?: number;
	room_id?: RoomId;
	room_name?: string;
	notable_id?: number;
	default: boolean;
}

export interface CountNode extends NodeBase {
	type: 'count'; // number stepper / slider
	name: string;
	/** Starting amount. For a calculated count this is its weight (the vanilla amount), not a value. */
	default: number;
	min: number;
	/** Required, except on calculated counts (they take whatever slots are left). */
	max?: number;
	step?: number;
	/**
	 * The player does not set this. It takes a share of the item slots left over after all the other
	 * counts (`itemLocations` minus their total), split randomly between the calculated counts,
	 * weighted by `default`, each getting at least `min`. Not shown as an input, not saved in links,
	 * not sent to generate: the generator works it out from the seed.
	 */
	calculated?: boolean;
}

export interface ChoiceNode extends NodeBase {
	type: 'choice';
	name: string;
	default: string;
	display?: 'cards' | 'segmented' | 'dropdown';
	options: { value: string; label?: string; desc?: string }[];
}

export type MasterNode = SectionNode | GroupNode | LevelNode | ToggleNode | CountNode | ChoiceNode;
export type LeafNode = LevelNode | ToggleNode | CountNode | ChoiceNode;

/** An optional hand-written preset. Difficulty presets are derived and need no entry here. */
export interface MasterPreset {
	id: string;
	name: string;
	/** Setting key -> value. Only what differs from defaults. */
	values: Record<string, Value>;
}

export interface MasterData {
	/** Bump when defaults or meaning change; share links record it. */
	version: number;
	/** Ranked easiest -> hardest. Order here is the only ranking; never sort alphabetically. */
	difficulties: string[];
	/** Difficulty values that are not on the ladder and never take part in derived presets. */
	unrankedDifficulties: string[];
	/**
	 * Number of item slots in the game. The player's item counts plus the calculated counts always
	 * add up to this, so the page also stops the player's counts from using more than is free.
	 */
	itemLocations?: number;
	presets?: MasterPreset[];
	root: MasterNode[];
}

// ---- Helpers -----------------------------------------------------------------------

export function isLeaf(n: MasterNode): n is LeafNode {
	return n.type !== 'section' && n.type !== 'group';
}

/**
 * The key a leaf is stored and sent under.
 *  - tech, proficiencies, items, choices: the community name ("canBonkBoost", "FireRod")
 *  - notables: "notable.<RoomKind>.<room id>.<notable id>", e.g. "notable.Underworld.60.1"
 * Names are therefore never renamed once released.
 */
export function nodeKey(n: LeafNode): string {
	if (n.type === 'toggle' && n.notable_id !== undefined && n.room_id) {
		const [kind, id] = Object.entries(n.room_id)[0];
		return `notable.${kind}.${id}.${n.notable_id}`;
	}
	return n.name;
}

export function labelOf(n: LeafNode): string {
	if (n.label) return n.label;
	// Room tricks: make the place obvious, e.g. "Tower of Hera: Hera Pot Clip".
	if (n.type === 'toggle' && n.notable_id !== undefined && n.room_name) return `${n.room_name}: ${n.name}`;
	return n.name;
}

/** Depth-first walk over every node. */
export function* walk(nodes: MasterNode[]): Generator<MasterNode> {
	for (const n of nodes) {
		yield n;
		if (n.type === 'section' || n.type === 'group') yield* walk(n.children);
	}
}
