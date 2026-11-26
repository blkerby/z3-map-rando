/** Data for the Logic pages, prepared at build time from the z3-json-data room files. */

/** URL segment of a room's ID space: overworld or underworld. */
export type RoomSpace = 'o' | 'u';

/** Index page section; Special rooms are listed with the overworld. */
export type RoomCategory = 'overworld' | 'dungeon' | 'cave' | 'house';

export interface RoomSummary {
	space: RoomSpace;
	id: number;
	name: string;
	darkWorldName?: string;
	category: RoomCategory;
}

export interface Notes {
	note: string[];
	detailNote: string[];
	devNote: string[];
}

export interface StratNode {
	id: number;
	name: string;
}

/** A strat property shown as its source JSON text. */
export interface StratProperty {
	label: string;
	text: string;
}

export interface LogicStrat extends Notes {
	id: number;
	name: string;
	from: StratNode;
	to: StratNode;
	/** Requirement items as source JSON text, one or more lines each; empty when there are none. */
	requires: string;
	properties: StratProperty[];
}

export interface LogicRoom extends RoomSummary, Notes {
	/** Room file path relative to the rooms folder, which locates its diagrams. */
	path: string;
	strats: LogicStrat[];
}
