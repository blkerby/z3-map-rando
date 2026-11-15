import type { Tree } from '$lib/settings/tree';
import { loadBrowser } from '$lib/browser';

export interface GenerateResult {
	seed: string;
	notices: string[];
}

/** Submit a scoped settings document; the response identifies the stored seed. */
export async function generateGame(settings: Tree): Promise<GenerateResult> {
	const browser = await loadBrowser();
	const response = await fetch('/api/generate', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ settings: { ...settings, format_version: browser.get_settings_format_version() } })
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
