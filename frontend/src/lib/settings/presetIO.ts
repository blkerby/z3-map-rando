import type { Preset, Value } from './types';

/** Which store a preset belongs to. Presets for one can't be imported into the other. */
export type PresetKind = 'settings' | 'customization';

export const FORMAT = 'alttp-maprando-presets';
export const FORMAT_VERSION = 1;

export const MAX_NAME = 40;
export const MAX_USER_PRESETS = 200;
export const MAX_IMPORT_PRESETS = 100;
export const MAX_IMPORT_BYTES = 1_000_000;

const KIND_LABEL: Record<PresetKind, string> = {
	settings: 'game settings',
	customization: 'customization'
};

export function newPresetId(): string {
	const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789';
	const bytes = crypto.getRandomValues(new Uint8Array(10));
	return 'user-' + Array.from(bytes, (b) => alphabet[b % alphabet.length]).join('');
}

/** Trim, collapse whitespace, drop control characters, cap the length. Returns '' if unusable. */
export function cleanName(raw: unknown): string {
	if (typeof raw !== 'string') return '';
	return raw.replace(/\p{Cc}/gu, ' ').replace(/\s+/g, ' ').trim().slice(0, MAX_NAME).trim();
}

export function sameName(a: string, b: string): boolean {
	return a.toLowerCase() === b.toLowerCase();
}

/** "Name", or "Name (2)", "Name (3)"… so it doesn't collide with any name in `taken`. */
export function uniqueName(name: string, taken: string[]): string {
	if (!taken.some((t) => sameName(t, name))) return name;
	for (let i = 2; ; i++) {
		const suffix = ` (${i})`;
		const candidate = name.slice(0, MAX_NAME - suffix.length).trim() + suffix;
		if (!taken.some((t) => sameName(t, candidate))) return candidate;
	}
}

// ---- Export file --------------------------------------------------------------

export function buildExportFile(kind: PresetKind, presets: Preset[]): string {
	return JSON.stringify(
		{
			format: FORMAT,
			version: FORMAT_VERSION,
			kind,
			presets: presets.map((p) => ({ name: p.name, values: p.values }))
		},
		null,
		2
	);
}

// ---- Preset code (short text for pasting into chat) ---------------------------

function toBase64Url(text: string): string {
	let binary = '';
	for (const byte of new TextEncoder().encode(text)) binary += String.fromCharCode(byte);
	return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function fromBase64Url(code: string): string {
	const b64 = code.replace(/-/g, '+').replace(/_/g, '/');
	const binary = atob(b64 + '='.repeat((4 - (b64.length % 4)) % 4));
	return new TextDecoder('utf-8', { fatal: true }).decode(Uint8Array.from(binary, (c) => c.charCodeAt(0)));
}

/** `diff` holds only values that differ from the schema defaults, which keeps the code short. */
export function encodePresetCode(kind: PresetKind, name: string, diff: Record<string, Value>): string {
	return toBase64Url(JSON.stringify({ k: kind, n: name, v: diff }));
}

// ---- Import -------------------------------------------------------------------

export interface ParsedPreset {
	name: string;
	values: Record<string, Value>;
}

export type ParseResult = { ok: true; presets: ParsedPreset[] } | { ok: false; error: string };

function isObject(x: unknown): x is Record<string, unknown> {
	return typeof x === 'object' && x !== null && !Array.isArray(x);
}

/**
 * Accepts either an exported JSON file or a single preset code.
 * Everything is validated; `sanitize` keeps only known, correctly-typed values.
 */
export function parseImport(
	text: string,
	kind: PresetKind,
	sanitize: (raw: unknown) => Record<string, Value>
): ParseResult {
	const input = text.trim();
	if (!input) return { ok: false, error: 'Paste a preset code or choose a preset file.' };
	if (input.length > MAX_IMPORT_BYTES) return { ok: false, error: 'That is too large to be a preset file.' };

	const wrongKind = (other: unknown) =>
		other === 'settings' || other === 'customization'
			? `That is a ${KIND_LABEL[other]} preset, but this list holds ${KIND_LABEL[kind]} presets.`
			: "That preset isn't for this list.";

	if (input.startsWith('{')) {
		let data: unknown;
		try {
			data = JSON.parse(input);
		} catch {
			return { ok: false, error: "That doesn't look like a valid preset file." };
		}
		if (!isObject(data) || data.format !== FORMAT || !Array.isArray(data.presets)) {
			return { ok: false, error: "That doesn't look like a preset file from this site." };
		}
		if (typeof data.version === 'number' && data.version > FORMAT_VERSION) {
			return { ok: false, error: 'That file was made by a newer version of the site. Please update first.' };
		}
		if (data.kind !== kind) return { ok: false, error: wrongKind(data.kind) };

		const presets: ParsedPreset[] = [];
		for (const item of data.presets.slice(0, MAX_IMPORT_PRESETS)) {
			if (!isObject(item) || !isObject(item.values)) continue;
			presets.push({
				name: cleanName(item.name) || 'Imported preset',
				values: sanitize(item.values)
			});
		}
		if (!presets.length) return { ok: false, error: 'That file has no usable presets in it.' };
		return { ok: true, presets };
	}

	const code = input.replace(/\s+/g, '');
	if (!/^[A-Za-z0-9_-]+$/.test(code)) {
		return { ok: false, error: "That doesn't look like a preset code or file." };
	}
	let data: unknown;
	try {
		data = JSON.parse(fromBase64Url(code));
	} catch {
		return { ok: false, error: "That preset code couldn't be read. Was it copied completely?" };
	}
	if (!isObject(data) || !isObject(data.v)) {
		return { ok: false, error: "That preset code couldn't be read." };
	}
	if (data.k !== kind) return { ok: false, error: wrongKind(data.k) };
	return {
		ok: true,
		presets: [{ name: cleanName(data.n) || 'Imported preset', values: sanitize(data.v) }]
	};
}

// ---- Browser storage ----------------------------------------------------------

/** Read presets back out of localStorage, discarding anything malformed. */
export function readStoredPresets(
	raw: string,
	sanitize: (raw: unknown) => Record<string, Value>
): Preset[] {
	let data: unknown;
	try {
		data = JSON.parse(raw);
	} catch {
		return [];
	}
	if (!isObject(data) || !Array.isArray(data.presets)) return [];

	const out: Preset[] = [];
	const names: string[] = [];
	for (const item of data.presets.slice(0, MAX_USER_PRESETS)) {
		if (!isObject(item) || !isObject(item.values)) continue;
		const name = uniqueName(cleanName(item.name) || 'Preset', names);
		names.push(name);
		const id =
			typeof item.id === 'string' && /^user-[a-z0-9]{4,32}$/.test(item.id) && !out.some((p) => p.id === item.id)
				? item.id
				: newPresetId();
		out.push({ id, name, values: sanitize(item.values) });
	}
	return out;
}
