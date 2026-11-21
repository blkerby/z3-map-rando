import { settingsData } from '$lib/generation/data';
import type { Tree } from './tree';

/**
 * Settings processing on the server (`POST /api/settings/process`), which migrates and recovers a stored
 * or imported settings document so the editor can load it. Documents this browser stored under the
 * current revision are loaded without it.
 */

/** Identifies this build's settings processing code and data. */
export const REVISION = settingsData.presets.revision;

/** Format version of the settings documents this build writes. */
export const FORMAT_VERSION = settingsData.format_version;

export type SettingsKind = 'generation' | 'customization';

/** One change recovery made, or one cross-setting problem left for the player to fix. */
export interface SettingsIssue {
	path: string[];
	message: string;
	replacement?: unknown;
}

export type ProcessResult =
	| { status: 'processed'; document: Tree; notices: SettingsIssue[]; errors: SettingsIssue[] }
	/** Recovery failed even without tweaks, which indicates a bug: the document can only be discarded. */
	| { status: 'unrecoverable' };

/** The server runs other settings data than this page was built with, so the page must be reloaded. */
export class StaleError extends Error {
	constructor() {
		super('The randomizer has been updated. Reload the page to continue.');
	}
}

/**
 * Process a document on the server. Throws `StaleError` for a server with another revision, or another
 * error when the request fails, which may be temporary. A processed document comes back without its
 * `format_version`, like the documents the store holds.
 */
export async function processDocument(
	kind: SettingsKind,
	document: unknown
): Promise<ProcessResult> {
	let response: Response;
	try {
		response = await fetch('/api/settings/process', {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ kind, document })
		});
	} catch {
		throw new Error("Couldn't reach the server.");
	}
	if (!response.ok)
		throw new Error(`Couldn't load your settings (server error ${response.status}).`);
	const data = await response.json();
	if (data.revision !== REVISION) throw new StaleError();
	if (data.status === 'unrecoverable') return { status: 'unrecoverable' };
	const { format_version: _, ...rest } = data.document;
	void _;
	return { status: 'processed', document: rest, notices: data.notices, errors: data.errors };
}
