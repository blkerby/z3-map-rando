import type { Tree } from '$lib/settings/tree';

/**
 * Version of the generator the page was built against. Shared links record it so a
 * link made today can warn (or route to an older generator) after the logic changes.
 * Keep this in sync with the `version` shown in +layout.svelte.
 */
export const GENERATOR_VERSION = 'v1';

export interface GameLink {
	version: string;
	seed: string;
	/** Settings code, as produced by SettingsStore.serialize(). */
	settings: string;
}

/**
 * Link format: #<version>.<seed>.<settings>
 * The hash stays in the browser; the seed identifies the stored generation result.
 */
export function buildHash(link: GameLink): string {
	return `${link.version}.${link.seed}.${link.settings}`;
}

const VERSION_RE = /^[A-Za-z0-9_-]{1,16}$/;
const SEED_RE = /^[A-Za-z0-9_-]{1,64}$/;
const SETTINGS_RE = /^[A-Za-z0-9_-]+$/;

/** Parse a location.hash. Returns null if it isn't a well-formed game link. */
export function parseHash(hash: string): GameLink | null {
	const parts = hash.replace(/^#/, '').split('.');
	if (parts.length !== 3) return null;
	const [version, seed, settings] = parts;
	if (!VERSION_RE.test(version) || !SEED_RE.test(seed) || !SETTINGS_RE.test(settings)) return null;
	return { version, seed, settings };
}

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
	version: string;
	/** The player's cosmetic choices (never part of the game link). */
	customization: Tree;
}

/**
 * STUB. Not connected yet, so it throws and the page shows the message.
 * Replace the body with the real work, either asking the server for a patched ROM, e.g.
 *
 *   const res = await fetch('/api/rom', {
 *     method: 'POST',
 *     headers: { 'Content-Type': 'application/json' },
 *     body: JSON.stringify(request)
 *   });
 *   if (!res.ok) throw new Error(await res.text());
 *   saveBlob(await res.blob(), `alttp-maprando-${request.seed}.sfc`);
 *
 * or applying the customization to a server-provided patch in the browser.
 */
export async function downloadRom(request: RomRequest): Promise<void> {
	void request;
	throw new Error("ROM download isn't connected yet.");
}
