import type { LogicRoom, LogicStrat, Notes, RoomCategory, RoomSummary } from '$lib/logic/types';

/** Raw room files keyed by Vite path; the text is kept so requirements display with their source formatting. */
const sources = import.meta.glob<string>('$z3_json_data/rooms/**/*.json', {
	query: '?raw',
	import: 'default',
	eager: true
});

/** Location of a JSON value in its source text; objects record their members. */
interface Span {
	start: number;
	end: number;
	members: Map<string, Span>;
	items: Span[];
}

/** Parse the JSON value starting at `text[start]` (after whitespace), recording where each value lies. */
function parseSpans(text: string, start: number): Span {
	let i = skipWhitespace(text, start);
	const span: Span = { start: i, end: i, members: new Map(), items: [] };
	const c = text[i];
	if (c === '{' || c === '[') {
		const close = c === '{' ? '}' : ']';
		i = skipWhitespace(text, i + 1);
		while (text[i] !== close) {
			let key = '';
			if (c === '{') {
				const keyEnd = endOfString(text, i);
				key = JSON.parse(text.slice(i, keyEnd));
				i = skipWhitespace(text, keyEnd) + 1; // Past ':'.
			}
			const value = parseSpans(text, i);
			if (c === '{') {
				span.members.set(key, value);
			} else {
				span.items.push(value);
			}
			i = skipWhitespace(text, value.end);
			if (text[i] === ',') {
				i = skipWhitespace(text, i + 1);
			}
		}
		span.end = i + 1;
	} else if (c === '"') {
		span.end = endOfString(text, i);
	} else {
		span.end = i + /^[^\s,\]}]+/.exec(text.slice(i))![0].length;
	}
	return span;
}

function skipWhitespace(text: string, i: number): number {
	while (/\s/.test(text[i])) {
		i++;
	}
	return i;
}

function endOfString(text: string, start: number): number {
	let i = start + 1;
	while (text[i] !== '"') {
		i += text[i] === '\\' ? 2 : 1;
	}
	return i + 1;
}

/** Remove the indentation that all lines but the first share, as the first line starts mid-line in its file. */
function dedent(lines: string[]): string {
	let indent = Infinity;
	for (const line of lines.slice(1)) {
		if (line.trim() !== '') {
			indent = Math.min(indent, line.length - line.trimStart().length);
		}
	}
	const result = [lines[0]];
	for (const line of lines.slice(1)) {
		result.push(line.slice(Math.min(indent, line.length)));
	}
	return result.join('\n');
}

/** Requirement items without the enclosing brackets, dedented. Requirement lists are always expanded. */
function formatRequires(text: string): string {
	const lines = text.split('\n');
	if (lines.length < 3) {
		return '';
	}
	return dedent(['', ...lines.slice(1, -1)]).slice(1);
}

function toParagraphs(value: string | string[] | undefined): string[] {
	if (value === undefined) {
		return [];
	}
	return typeof value === 'string' ? [value] : value;
}

function readNotes(source: {
	note?: string | string[];
	detailNote?: string[];
	devNote?: string[];
}): Notes {
	return {
		note: toParagraphs(source.note),
		detailNote: toParagraphs(source.detailNote),
		devNote: toParagraphs(source.devNote)
	};
}

/** Label for a property name, e.g. "collectsItems" becomes "Collects items". */
function labelProperty(name: string): string {
	const words = name.replace(/([A-Z])/g, ' $1').toLowerCase();
	return words[0].toUpperCase() + words.slice(1);
}

/** Strat properties shown in the strat header or as notes rather than as source text. */
const specialProperties = new Set([
	'id',
	'link',
	'name',
	'requires',
	'note',
	'detailNote',
	'devNote'
]);

const categories: Record<string, RoomCategory> = {
	Overworld: 'overworld',
	Special: 'overworld',
	Dungeon: 'dungeon',
	Cave: 'cave',
	House: 'house'
};

interface SourceRoom {
	id: number;
	name: string;
	darkWorldName?: string;
	roomType: string;
	note?: string[];
	detailNote?: string[];
	devNote?: string[];
	nodes: { id: number; name: string }[];
	strats: {
		id: number;
		link: [number, number];
		name: string;
		world?: 'light' | 'dark' | 'any';
		fromWorld?: 'light' | 'dark';
		toWorld?: 'light' | 'dark';
		note?: string[];
		detailNote?: string[];
		devNote?: string[];
	}[];
}

function readRoom(key: string, text: string): LogicRoom {
	const room: SourceRoom = JSON.parse(text);
	const stratSpans = parseSpans(text, 0).members.get('strats')!.items;
	const nodeNames = new Map<number, string>();
	for (const node of room.nodes) {
		nodeNames.set(node.id, node.name);
	}
	const strats: LogicStrat[] = [];
	for (const [i, strat] of room.strats.entries()) {
		const members = stratSpans[i].members;
		const sourceText = (name: string) => {
			const span = members.get(name)!;
			return text.slice(span.start, span.end);
		};
		const properties = [];
		for (const name of members.keys()) {
			if (!specialProperties.has(name)) {
				properties.push({ label: labelProperty(name), text: dedent(sourceText(name).split('\n')) });
			}
		}
		const [from, to] = strat.link;
		strats.push({
			id: strat.id,
			name: strat.name,
			world: strat.world,
			fromWorld: strat.fromWorld,
			toWorld: strat.toWorld,
			from: { id: from, name: nodeNames.get(from)! },
			to: { id: to, name: nodeNames.get(to)! },
			requires: formatRequires(sourceText('requires')),
			properties,
			...readNotes(strat)
		});
	}
	return {
		id: room.id,
		name: room.name,
		darkWorldName: room.darkWorldName,
		category: categories[room.roomType],
		path: key.slice(key.indexOf('/rooms/') + '/rooms/'.length),
		nodes: room.nodes.map(({ id, name }) => ({ id, name })),
		strats,
		...readNotes(room)
	};
}

/** All rooms, sorted by room ID. */
export const rooms: LogicRoom[] = [];
for (const [key, text] of Object.entries(sources)) {
	rooms.push(readRoom(key, text));
}
rooms.sort((a, b) => a.id - b.id);

/** All rooms, sorted by room ID. */
export function listRooms(): RoomSummary[] {
	const summaries = [];
	for (const { id, name, darkWorldName, category } of rooms) {
		summaries.push({ id, name, darkWorldName, category });
	}
	return summaries;
}

export function findRoom(id: number): LogicRoom | undefined {
	return rooms.find((room) => room.id === id);
}
