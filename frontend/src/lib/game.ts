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
 * The hash never reaches the server, and the same three parts always reproduce the same map.
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

export function randomSeed(length = 8): string {
	// No 0/O/1/I/L so seeds are easy to read out loud or retype.
	const alphabet = 'ABCDEFGHJKMNPQRSTVWXYZ23456789';
	const bytes = crypto.getRandomValues(new Uint8Array(length));
	return Array.from(bytes, (b) => alphabet[b % alphabet.length]).join('');
}

export interface GenerateResult {
	seed: string;
}

/**
 * STUB. Replace the body with a call to your generation server, e.g.
 *
 *   const res = await fetch('/api/generate', {
 *     method: 'POST',
 *     headers: { 'Content-Type': 'application/json' },
 *     body: JSON.stringify({ version: GENERATOR_VERSION, settings })
 *   });
 *   if (!res.ok) throw new Error(await res.text());
 *   return await res.json(); // { seed: string }
 *
 * Throw an Error with a user-readable message on failure; the page shows it.
 */
export async function generateGame(settings: Tree): Promise<GenerateResult> {
	void settings;
	await new Promise((resolve) => setTimeout(resolve, 1200));
	return { seed: randomSeed() };
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
