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
	/** The player's vanilla ROM. */
	rom: Uint8Array;
	/** The player's cosmetic choices (never part of the seed link). */
	customization: Tree;
}

async function fetchBytes(url: string): Promise<Uint8Array> {
	const response = await fetch(url);
	if (!response.ok) throw new Error(`Could not download ${url} (${response.status}).`);
	return new Uint8Array(await response.arrayBuffer());
}

/** Patch the player's ROM in the browser with the saved seed, and save the result. */
export async function downloadRom(request: RomRequest): Promise<void> {
	const seedUrl = `/api/seed/${encodeURIComponent(request.seed)}`;
	const [browser, seed, retiling, patches] = await Promise.all([
		loadBrowser(),
		fetchBytes(seedUrl),
		fetchBytes(`${seedUrl}/retiling`),
		fetchBytes(patchCatalogUrl)
	]);
	const customization = new TextEncoder().encode(JSON.stringify(request.customization));
	const output = browser.patch_rom(request.rom, seed, customization, retiling, patches);
	const url = URL.createObjectURL(new Blob([output as Uint8Array<ArrayBuffer>], { type: 'application/octet-stream' }));
	const a = document.createElement('a');
	a.href = url;
	a.download = `z3-map-rando-${request.seed}.sfc`;
	document.body.append(a);
	a.click();
	a.remove();
	setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export interface SpoilerEntry {
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

/** Replay the saved seed's logical traversal in the browser to group its items into spheres. */
export async function computeSpoiler(seed: string): Promise<Spoiler> {
	const [browser, seedBytes, logic] = await Promise.all([
		loadBrowser(),
		fetchBytes(`/api/seed/${encodeURIComponent(seed)}`),
		fetchBytes(logicCatalogUrl)
	]);
	return JSON.parse(browser.replay_spoiler(seedBytes, logic));
}
