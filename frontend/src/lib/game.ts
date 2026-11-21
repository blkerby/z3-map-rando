import type { Tree } from '$lib/settings/tree';
import { loadBrowser } from '$lib/browser';
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
	/** The player's cosmetic choices (never part of the game link). */
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
