import logicData from '$build/logic_data.json';
import settingsDataJson from '$settings_data';
import entrancesJson from '$z3_json_data/connections/entrances.json';
import teleportsJson from '$z3_json_data/connections/teleports.json';
import whirlpoolsJson from '$z3_json_data/connections/whirlpools.json';
import helpersJson from '$z3_json_data/helpers.json';
import techRaw from '$z3_json_data/tech.json?raw';
import type {
	LogicRoom,
	LogicStrat,
	LogicTech,
	Notes,
	RoomCategory,
	RoomConnection,
	RoomSummary,
	TechStratEntry,
	TechSummary,
	TieredTechSummary
} from '$lib/logic/types';

/** Strat difficulties by room ID and strat ID, prepared by the catalog builder. */
const difficulties = logicData as Record<string, Record<string, string>>;

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

function joinNotes(value: string | string[] | undefined): string {
	if (value === undefined) {
		return '';
	}
	return typeof value === 'string' ? value : value.join(' ');
}

function readNotes(source: {
	note?: string | string[];
	detailNote?: string | string[];
	devNote?: string | string[];
}): Notes {
	return {
		note: joinNotes(source.note),
		detailNote: joinNotes(source.detailNote),
		devNote: joinNotes(source.devNote)
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
	position?: [number, number];
	size?: [number, number];
	note?: string[];
	detailNote?: string[];
	devNote?: string[];
	nodes: {
		id: number;
		name: string;
		entrances?: { id: number; name: string }[];
		whirlpools?: { id: number; name: string }[];
		teleports?: { id: number; name: string }[];
	}[];
	strats: {
		id: number;
		link: [number, number];
		name: string;
		requires: SourceRequirement[];
		world?: 'light' | 'dark' | 'any';
		fromWorld?: 'light' | 'dark';
		toWorld?: 'light' | 'dark';
		note?: string[];
		detailNote?: string[];
		devNote?: string[];
	}[];
}

interface SourceEntranceConnection {
	type: 'door' | 'drop';
	world: 'light' | 'dark';
	overworld: { roomId: number; entranceId: number };
	underworld: { roomId: number; nodeId: number };
}

interface SourceWhirlpoolConnection {
	world: 'light' | 'dark';
	overworld: { roomId: number; whirlpoolId: number };
	overworld2: { roomId: number; whirlpoolId: number };
}

interface SourceTeleportConnection {
	fromWorld: 'light' | 'dark';
	toWorld: 'light' | 'dark';
	underworld: { roomId: number; nodeId: number };
	overworld: { roomId: number; teleportId: number };
}

type SourceRequirement =
	| string
	| {
			and?: SourceRequirement[];
			or?: SourceRequirement[];
			[key: string]: unknown;
	  };

interface SourceTier {
	value: number;
	note?: string[];
	detailNote?: string[];
	devNote?: string[];
}

interface SourceTech {
	id: number;
	name: string;
	tiers?: SourceTier[];
	techRequires?: string[];
	otherRequires?: SourceRequirement[];
	extensionTechs?: SourceTech[];
	note?: string[];
	detailNote?: string[];
	devNote?: string[];
}

/** Form conversions that implicitly depend on a tech (matches `catalog_builder/src/logic/requirements.rs`). */
const formConversionTech: Record<string, string> = {
	bunnyToSuperbunny: 'canSuperbunny',
	bunnyToLonk: 'canCarryLonk',
	bunnyToLink: 'canPearllessLink',
	superbunnyToLink: 'canPearllessLink'
};

/** Helpers overridden to `free` by the randomizer (matches `catalog_builder/src/logic/requirements.rs`). */
const freeHelpers = new Set(['h_MothulaVulnerableToGoldSword', 'h_greatFairyFreeUpgrades']);

const helpers = new Map<string, SourceRequirement[]>();
for (const helper of (helpersJson as { helpers: { name: string; requires: SourceRequirement[] }[] })
	.helpers) {
	if (!freeHelpers.has(helper.name)) {
		helpers.set(helper.name, helper.requires);
	}
}

const settingsData = settingsDataJson as {
	difficulties: string[];
	techs: { id: number; difficulty: string }[];
	proficiencies: { id: number; levels: { level: number; difficulty: string }[] }[];
};

const techDifficulties = new Map<number, string>();
for (const tech of settingsData.techs) {
	techDifficulties.set(tech.id, tech.difficulty);
}

const tierDifficulties = new Map<string, string>();
for (const prof of settingsData.proficiencies) {
	for (const level of prof.levels) {
		tierDifficulties.set(`${prof.id}:${level.level}`, level.difficulty);
	}
}

/** Ranked difficulties from easiest to hardest. */
export const difficultyOrder: string[] = settingsData.difficulties;

const parsedTechs: { tech: SourceTech; span: Span }[] = [];
const parsedTieredTechs: SourceTech[] = [];
{
	const techData = JSON.parse(techRaw) as { techCategories: { techs: SourceTech[] }[] };
	const categorySpans = parseSpans(techRaw, 0).members.get('techCategories')!.items;
	const collect = (list: SourceTech[], spans: Span[]) => {
		for (let i = 0; i < list.length; i++) {
			const tech = list[i];
			const span = spans[i];
			if (tech.tiers) {
				parsedTieredTechs.push(tech);
			} else {
				parsedTechs.push({ tech, span });
			}
			if (tech.extensionTechs?.length) {
				collect(tech.extensionTechs, span.members.get('extensionTechs')!.items);
			}
		}
	};
	for (let i = 0; i < techData.techCategories.length; i++) {
		collect(techData.techCategories[i].techs, categorySpans[i].members.get('techs')!.items);
	}
}

const techSummaryByName = new Map<string, TechSummary>();
for (const { tech } of parsedTechs) {
	techSummaryByName.set(tech.name, {
		id: tech.id,
		name: tech.name,
		difficulty: techDifficulties.get(tech.id)!
	});
}

const tieredTechNames = new Set(parsedTieredTechs.map((tech) => tech.name));

/**
 * Collect tech keys referenced by `requires`, expanding helpers and form conversions but not
 * `techRequires`. Non-tiered techs use their name; tiered techs use `${name}:${tier}`.
 */
function collectReferencedTechs(
	requires: SourceRequirement[],
	found: Set<string>,
	visitedHelpers = new Set<string>()
) {
	for (const req of requires) {
		if (typeof req === 'string') {
			if (techSummaryByName.has(req)) {
				found.add(req);
			} else if (req in formConversionTech) {
				found.add(formConversionTech[req]);
			} else {
				const helperReqs = helpers.get(req);
				if (helperReqs && !visitedHelpers.has(req)) {
					visitedHelpers.add(req);
					collectReferencedTechs(helperReqs, found, visitedHelpers);
				}
			}
		} else if (req && typeof req === 'object') {
			if (req.and) collectReferencedTechs(req.and, found, visitedHelpers);
			if (req.or) collectReferencedTechs(req.or, found, visitedHelpers);
			for (const [key, value] of Object.entries(req)) {
				if (tieredTechNames.has(key) && typeof value === 'number') {
					found.add(`${key}:${value}`);
				}
			}
		}
	}
}

type OverworldEndpoint = { name: string; nodeId: number };

const overworldEntrances = new Map<string, OverworldEndpoint>();
const overworldWhirlpools = new Map<string, OverworldEndpoint>();
const overworldTeleports = new Map<string, OverworldEndpoint>();

function readRoom(key: string, text: string): { room: LogicRoom; stratTechs: Set<string>[] } {
	const room: SourceRoom = JSON.parse(text);
	const stratSpans = parseSpans(text, 0).members.get('strats')!.items;
	const nodeNames = new Map<number, string>();
	for (const node of room.nodes) {
		nodeNames.set(node.id, node.name);
		for (const entrance of node.entrances ?? []) {
			overworldEntrances.set(`${room.id}:${entrance.id}`, {
				name: entrance.name,
				nodeId: node.id
			});
		}
		for (const whirlpool of node.whirlpools ?? []) {
			overworldWhirlpools.set(`${room.id}:${whirlpool.id}`, {
				name: whirlpool.name,
				nodeId: node.id
			});
		}
		for (const teleport of node.teleports ?? []) {
			overworldTeleports.set(`${room.id}:${teleport.id}`, {
				name: teleport.name,
				nodeId: node.id
			});
		}
	}
	const strats: LogicStrat[] = [];
	const stratTechs: Set<string>[] = [];
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
			difficulty: difficulties[room.id][strat.id],
			world: strat.world,
			fromWorld: strat.fromWorld,
			toWorld: strat.toWorld,
			from: { id: from, name: nodeNames.get(from)! },
			to: { id: to, name: nodeNames.get(to)! },
			requires: formatRequires(sourceText('requires')),
			properties,
			...readNotes(strat)
		});
		const referenced = new Set<string>();
		collectReferencedTechs(strat.requires, referenced);
		stratTechs.push(referenced);
	}
	return {
		room: {
			id: room.id,
			name: room.name,
			darkWorldName: room.darkWorldName,
			category: categories[room.roomType],
			mapRect:
				room.position && room.size
					? {
							x: room.position[0],
							y: room.position[1],
							width: room.size[0] / 2,
							height: room.size[1] / 2
						}
					: undefined,
			path: key.slice(key.indexOf('/rooms/') + '/rooms/'.length),
			nodes: room.nodes.map(({ id, name }) => ({ id, name })),
			connections: [],
			strats,
			...readNotes(room)
		},
		stratTechs
	};
}

/** All rooms, sorted by room ID. */
const parsedRooms = Object.entries(sources)
	.map(([key, text]) => readRoom(key, text))
	.sort((a, b) => a.room.id - b.room.id);

export const rooms: LogicRoom[] = parsedRooms.map((entry) => entry.room);

{
	const worldLabel = (world: 'light' | 'dark') => (world === 'light' ? 'Light' : 'Dark');
	const roomTarget = (room: LogicRoom) => ({
		id: room.id,
		name: room.name,
		darkWorldName: room.darkWorldName
	});

	const roomsById = new Map<number, LogicRoom>();
	for (const room of rooms) {
		roomsById.set(room.id, room);
	}

	const entrances = (entrancesJson as { entrances: SourceEntranceConnection[] }).entrances;
	for (const conn of entrances) {
		const owRoom = roomsById.get(conn.overworld.roomId)!;
		const uwRoom = roomsById.get(conn.underworld.roomId)!;
		const owEntrance = overworldEntrances.get(
			`${conn.overworld.roomId}:${conn.overworld.entranceId}`
		)!;
		const detail = worldLabel(conn.world);
		owRoom.connections.push({
			kind: 'entrance',
			nodeId: owEntrance.nodeId,
			name: owEntrance.name,
			direction: conn.type === 'drop' ? 'out' : 'both',
			targetRoom: roomTarget(uwRoom),
			detail,
			targetWorld: conn.world
		});
		uwRoom.connections.push({
			kind: 'entrance',
			nodeId: conn.underworld.nodeId,
			name: owEntrance.name,
			direction: conn.type === 'drop' ? 'in' : 'both',
			targetRoom: roomTarget(owRoom),
			detail,
			targetWorld: conn.world
		});
	}

	const whirlpools = (whirlpoolsJson as { whirlpools: SourceWhirlpoolConnection[] }).whirlpools;
	for (const conn of whirlpools) {
		const room1 = roomsById.get(conn.overworld.roomId)!;
		const room2 = roomsById.get(conn.overworld2.roomId)!;
		const wp1 = overworldWhirlpools.get(`${conn.overworld.roomId}:${conn.overworld.whirlpoolId}`)!;
		const wp2 = overworldWhirlpools.get(
			`${conn.overworld2.roomId}:${conn.overworld2.whirlpoolId}`
		)!;
		const detail = worldLabel(conn.world);
		room1.connections.push({
			kind: 'whirlpool',
			nodeId: wp1.nodeId,
			name: wp1.name,
			direction: 'both',
			targetRoom: roomTarget(room2),
			detail,
			targetWorld: conn.world
		});
		room2.connections.push({
			kind: 'whirlpool',
			nodeId: wp2.nodeId,
			name: wp2.name,
			direction: 'both',
			targetRoom: roomTarget(room1),
			detail,
			targetWorld: conn.world
		});
	}

	const teleports = (teleportsJson as { teleports: SourceTeleportConnection[] }).teleports;
	for (const conn of teleports) {
		const uwRoom = roomsById.get(conn.underworld.roomId)!;
		const owRoom = roomsById.get(conn.overworld.roomId)!;
		const owTeleport = overworldTeleports.get(
			`${conn.overworld.roomId}:${conn.overworld.teleportId}`
		)!;
		const detail =
			conn.fromWorld === conn.toWorld
				? worldLabel(conn.fromWorld)
				: `${worldLabel(conn.fromWorld)} → ${worldLabel(conn.toWorld)}`;
		uwRoom.connections.push({
			kind: 'teleport',
			nodeId: conn.underworld.nodeId,
			name: owTeleport.name,
			direction: 'out',
			targetRoom: roomTarget(owRoom),
			detail,
			targetWorld: conn.toWorld
		});
		owRoom.connections.push({
			kind: 'teleport',
			nodeId: owTeleport.nodeId,
			name: owTeleport.name,
			direction: 'in',
			targetRoom: roomTarget(uwRoom),
			detail,
			targetWorld: conn.fromWorld
		});
	}

	const worldOrder: Record<RoomConnection['targetWorld'], number> = {
		light: 0,
		dark: 1
	};
	for (const room of rooms) {
		room.connections.sort(
			(a, b) => worldOrder[a.targetWorld] - worldOrder[b.targetWorld] || a.nodeId - b.nodeId
		);
	}
}

const stratsByTechKey = new Map<string, TechStratEntry[]>();
for (const name of techSummaryByName.keys()) {
	stratsByTechKey.set(name, []);
}
for (const tech of parsedTieredTechs) {
	for (const tier of tech.tiers!) {
		stratsByTechKey.set(`${tech.name}:${tier.value}`, []);
	}
}
for (const { room, stratTechs } of parsedRooms) {
	const roomRef = { id: room.id, name: room.name };
	for (let i = 0; i < room.strats.length; i++) {
		for (const techKey of stratTechs[i]) {
			stratsByTechKey.get(techKey)!.push({ room: roomRef, strat: room.strats[i] });
		}
	}
}

/** All non-tiered techs, in `tech.json` order. */
export const techs: LogicTech[] = parsedTechs.map(({ tech, span }) => {
	const otherSpan = span.members.get('otherRequires');
	const otherRequires = otherSpan
		? formatRequires(techRaw.slice(otherSpan.start, otherSpan.end))
		: '';
	return {
		id: tech.id,
		name: tech.name,
		difficulty: techDifficulties.get(tech.id)!,
		techRequires: (tech.techRequires ?? []).map((name) => techSummaryByName.get(name)!),
		otherRequires,
		strats: stratsByTechKey.get(tech.name)!,
		...readNotes(tech)
	};
});

/** All tiered tech pages, in `tech.json` order. */
export const tieredTechPages: LogicTech[] = [];
for (const tech of parsedTieredTechs) {
	for (const tier of tech.tiers!) {
		tieredTechPages.push({
			id: tech.id,
			name: tech.name,
			tier: tier.value,
			difficulty: tierDifficulties.get(`${tech.id}:${tier.value}`)!,
			techRequires: [],
			otherRequires: '',
			strats: stratsByTechKey.get(`${tech.name}:${tier.value}`)!,
			...readNotes(tier)
		});
	}
}

/** All rooms, sorted by room ID. */
export function listRooms(): RoomSummary[] {
	const summaries = [];
	for (const { id, name, darkWorldName, category, mapRect } of rooms) {
		summaries.push({ id, name, darkWorldName, category, mapRect });
	}
	return summaries;
}

export function findRoom(id: number): LogicRoom | undefined {
	return rooms.find((room) => room.id === id);
}

/** All non-tiered techs, in `tech.json` order. */
export function listTechs(): TechSummary[] {
	const summaries = [];
	for (const { id, name, difficulty } of techs) {
		summaries.push({ id, name, difficulty });
	}
	return summaries;
}

/** All tiered techs with their tiers and difficulties, in `tech.json` order. */
export function listTieredTechs(): TieredTechSummary[] {
	const summaries: TieredTechSummary[] = [];
	for (const tech of parsedTieredTechs) {
		summaries.push({
			id: tech.id,
			name: tech.name,
			tiers: tech.tiers!.map((tier) => ({
				tier: tier.value,
				difficulty: tierDifficulties.get(`${tech.id}:${tier.value}`)!
			}))
		});
	}
	return summaries;
}

export function findTech(id: number): LogicTech | undefined {
	return techs.find((tech) => tech.id === id);
}

export function findTieredTech(id: number, tier: number): LogicTech | undefined {
	return tieredTechPages.find((tech) => tech.id === id && tech.tier === tier);
}
