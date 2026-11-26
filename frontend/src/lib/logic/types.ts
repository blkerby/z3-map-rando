/** Data for the Logic pages, prepared at build time from the z3-json-data room files. */

/** Index page section; Special rooms are listed with the overworld. */
export type RoomCategory = 'overworld' | 'dungeon' | 'cave' | 'house';

export interface RoomSummary {
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

/** Overworld strats set either `world` or, when crossing between worlds, `fromWorld` and `toWorld`. */
export interface LogicStrat extends Notes {
	id: number;
	name: string;
	/** The easiest skill preset label the strat passes with, or "Ignored". */
	difficulty: string;
	world?: 'light' | 'dark' | 'any';
	fromWorld?: 'light' | 'dark';
	toWorld?: 'light' | 'dark';
	from: StratNode;
	to: StratNode;
	/** Requirement items as source JSON text, one or more lines each; empty when there are none. */
	requires: string;
	properties: StratProperty[];
}

export interface LogicRoom extends RoomSummary, Notes {
	/** Room file path relative to the rooms folder, which locates its diagrams. */
	path: string;
	nodes: StratNode[];
	strats: LogicStrat[];
}
