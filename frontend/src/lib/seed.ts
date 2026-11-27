import type { Tree } from '$lib/settings/tree';
import { loadBrowser } from '$lib/browser';
import logicCatalogUrl from '$build/logic_catalog.bin?url';
import patchCatalogUrl from '$build/patch_catalog.bin?url';

export interface GenerateResult {
	seed: string;
	notices: string[];
}

/** Submit a scoped settings document; the response identifies the stored seed. */
export async function generateGame(settings: Tree): Promise<GenerateResult> {
	const response = await fetch('/api/generate', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ settings })
	});
	if (!response.ok) {
		const text = await response.text();
		let message = text;
		try {
			message = JSON.parse(text).error ?? text;
		} catch {
			/* Non-JSON HTTP error. */
		}
		throw new Error(message || `Generation failed (${response.status}).`);
	}
	return await response.json();
}

export interface RomRequest {
	seed: string;
	/** Raw saved seed JSON bytes. */
	seedBytes: Uint8Array;
	/** Override generation settings, or null to use the seed's recorded settings. */
	settings: Tree | null;
	/** Whether the patched ROM reflects modified gameplay settings. */
	modified: boolean;
	/** The player's vanilla ROM. */
	rom: Uint8Array;
	/** The player's cosmetic choices (never part of the seed link). */
	customization: Tree;
}

export async function fetchBytes(url: string): Promise<Uint8Array> {
	const response = await fetch(url);
	if (!response.ok) throw new Error(`Could not download ${url} (${response.status}).`);
	return new Uint8Array(await response.arrayBuffer());
}

function encodeOptionalJson(value: Tree | null): Uint8Array {
	return value === null ? new Uint8Array() : new TextEncoder().encode(JSON.stringify(value));
}

/** Patch the player's ROM in the browser with the saved seed, and save the result. */
export async function downloadRom(request: RomRequest): Promise<void> {
	const seedUrl = `/api/seed/${encodeURIComponent(request.seed)}`;
	const [browser, retiling, patches] = await Promise.all([
		loadBrowser(),
		fetchBytes(`${seedUrl}/retiling`),
		fetchBytes(patchCatalogUrl)
	]);
	const settings = encodeOptionalJson(request.settings);
	const customization = new TextEncoder().encode(JSON.stringify(request.customization));
	const output = browser.patch_rom(
		request.rom,
		request.seedBytes,
		settings,
		customization,
		retiling,
		patches
	);
	const url = URL.createObjectURL(
		new Blob([output as Uint8Array<ArrayBuffer>], { type: 'application/octet-stream' })
	);
	const a = document.createElement('a');
	a.href = url;
	a.download = request.modified
		? `z3-map-rando-${request.seed}-modified.sfc`
		: `z3-map-rando-${request.seed}.sfc`;
	document.body.append(a);
	a.click();
	a.remove();
	setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export interface SpoilerEntry {
	/** Catalog item location index, for route queries. */
	location_idx: number;
	location: string;
	/** Item kind token, or null for a location without a saved item. */
	item: string | null;
	/** For a Small Key, the door it opens. */
	door: string | null;
	/** For a Map, Compass, or Big Key, its dungeon. */
	dungeon: string | null;
}

export interface Spoiler {
	/** Locations first collectible in each sphere, in catalog order. */
	spheres: SpoilerEntry[][];
	/** Locations the current logic never reaches. */
	unreachable: SpoilerEntry[];
}

/** Local state after a route row's step; usage is counted from the start of the route. */
export interface RouteState {
	form: string;
	/** Eighths of a heart consumed, or remaining after a Fairy revival; exactly one is set. */
	health_used: number | null;
	health_remaining: number | null;
	/** Base magic points; a full meter holds 128. */
	magic_used: number;
	bombs_used: number;
	arrows_used: number;
	bottles_used: number;
	/** Followers that may be present; having none is always possible. */
	followers: string[];
}

/** What a route leads to: collecting an item location, or a permanent discovery. */
export type RouteTarget =
	{ kind: 'location' | 'flag' | 'door'; index: number } | { kind: 'ocarina' };

export interface RouteDependency {
	name: string;
	/** The route to follow for it, or null for progress held from the start. */
	target: RouteTarget | null;
}

export interface RouteRow {
	/**
	 * For a step entering a room or changing world, the connection and the room entered, such as "West to Swamp Ruins"
	 * or "Portal to Ice Lake"; such a step starts a new group of steps.
	 */
	entered: string | null;
	/** The node reached, as "<node ID>: <node name>", so it can be found on the room diagrams. */
	node: string;
	/** Whether the step returns to the node it leaves from, in the same world. */
	stays: boolean;
	/** Whether the node is in the Dark World, or in an interior entered from it. */
	dark_world: boolean;
	/** The strat name, or a name synthesized from the connection kind, such as "Cross West" or "Flute"; null for the start. */
	strat: string | null;
	/** The room ID of the node reached, for the room's logic page. */
	room_id: number;
	/** `[roomId, stratId]` when this step is an authored strat with a logic page. */
	strat_link?: [number, number];
	/** Flags, opened doors, and Ocarina activation that the step's requirement reads. */
	dependencies: RouteDependency[];
	state: RouteState;
	/**
	 * Where the step crosses an overworld entrance, or for a map boundary, points just before and after it; then on the
	 * last step, where its target item sits on the overworld. Empty for other steps, or for a door before the world maps
	 * are loaded.
	 */
	points: MapPoint[];
	/** Whether the step is a warp or starts in an interior, so travel since the previous point isn't overworld travel. */
	indirect: boolean;
}

/** A position on the world maps, in pixels of their 320x320 images. */
export interface MapPoint {
	world: 'light' | 'dark';
	x: number;
	y: number;
}

/** A replayed seed held in WebAssembly memory; call `free` when done with it. */
export interface SpoilerSession {
	spoiler: Spoiler;
	/** Recompute spheres and routes under new settings (null to use the seed's recorded settings). */
	replay(settings: Tree | null): Spoiler;
	/** A route from the start to a target, or null if none is found. */
	traceRoute(target: RouteTarget): RouteRow[] | null;
	/**
	 * Render the Light and Dark World maps from the player's ROM, as PNG object URLs; revoke them when done. Routes
	 * traced afterward include entrance points.
	 */
	loadWorldMaps(rom: Uint8Array): Promise<string[]>;
	free(): void;
}

/** Replay the saved seed's logical traversal in the browser to group its items into spheres. */
export async function openSpoiler(
	seed: string,
	seedBytes: Uint8Array,
	settings: Tree | null
): Promise<SpoilerSession> {
	const [browser, logic] = await Promise.all([loadBrowser(), fetchBytes(logicCatalogUrl)]);
	const session = new browser.SpoilerSession(seedBytes, encodeOptionalJson(settings), logic);
	return {
		spoiler: JSON.parse(session.get_spheres()),
		replay: (nextSettings) => {
			session.replay(encodeOptionalJson(nextSettings));
			return JSON.parse(session.get_spheres());
		},
		traceRoute: (target) => JSON.parse(session.trace_route(JSON.stringify(target))),
		loadWorldMaps: async (rom) => {
			const seedUrl = `/api/seed/${encodeURIComponent(seed)}`;
			const [retiling, patches] = await Promise.all([
				fetchBytes(`${seedUrl}/retiling`),
				fetchBytes(patchCatalogUrl)
			]);
			const urls: string[] = [];
			for (const png of session.load_world_maps(rom, retiling, patches)) {
				const blob = new Blob([png as Uint8Array<ArrayBuffer>], { type: 'image/png' });
				urls.push(URL.createObjectURL(blob));
			}
			return urls;
		},
		free: () => session.free()
	};
}
