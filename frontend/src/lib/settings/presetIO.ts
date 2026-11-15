import { FORMAT_VERSION } from './store.svelte';
import { isTree, type Tree } from './tree';

/** Which preset list a preset belongs to ('settings' or 'customization'). Presets for one can't be imported into another. */
export type PresetKind = string;

export const FORMAT = 'alttp-maprando-presets';

export const MAX_NAME = 40;
export const MAX_USER_PRESETS = 200;
export const MAX_IMPORT_PRESETS = 100;
export const MAX_IMPORT_BYTES = 1_000_000;

function kindLabel(kind: string): string {
	return kind === 'settings' ? 'game settings' : kind;
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

/** An export file, which is also what "Copy as code" puts on the clipboard (with one preset). */
export function buildExportFile(kind: PresetKind, presets: { name: string; document: Tree }[]): string {
	const out = [];
	for (const p of presets) out.push({ name: p.name, document: { format_version: FORMAT_VERSION, ...p.document } });
	return JSON.stringify({ format: FORMAT, kind, presets: out }, null, 2);
}

export interface ParsedPreset {
	name: string;
	document: Tree;
}

export type ParseResult = { ok: true; presets: ParsedPreset[] } | { ok: false; error: string };

/** Reads an export file (or a copied preset code). `readDocument` reads each document leniently. */
export function parseImport(text: string, kind: PresetKind, readDocument: (raw: unknown) => Tree): ParseResult {
	const input = text.trim();
	if (!input) return { ok: false, error: 'Paste a preset code or choose a preset file.' };
	if (input.length > MAX_IMPORT_BYTES) return { ok: false, error: 'That is too large to be a preset file.' };

	const wrongKind = (other: unknown) =>
		typeof other === 'string' && other.length <= 40
			? `That is a ${kindLabel(other)} preset, but this list holds ${kindLabel(kind)} presets.`
			: "That preset isn't for this list.";

	let data: unknown;
	try {
		data = JSON.parse(input);
	} catch {
		return { ok: false, error: "That doesn't look like a valid preset file." };
	}
	if (!isTree(data) || data.format !== FORMAT || !Array.isArray(data.presets)) {
		return { ok: false, error: "That doesn't look like a preset file from this site." };
	}
	if (data.kind !== kind) return { ok: false, error: wrongKind(data.kind) };
	const presets: ParsedPreset[] = [];
	for (const item of data.presets.slice(0, MAX_IMPORT_PRESETS)) {
		if (!isTree(item)) continue;
		presets.push({ name: cleanName(item.name) || 'Imported preset', document: readDocument(item.document) });
	}
	if (!presets.length) return { ok: false, error: 'That file has no usable presets in it.' };
	return { ok: true, presets };
}
