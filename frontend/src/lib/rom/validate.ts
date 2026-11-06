/**
 * Pure ROM checks: no storage, no UI. Everything here runs in the browser; the ROM never leaves it.
 */

export interface AcceptedRom {
	/** Shown to the player when their file doesn't match. */
	label: string;
	/** Lowercase hex SHA-1 of the headerless ROM. */
	sha1: string;
}

/**
 * The base ROMs the patcher/generator can work from.
 *
 * Hash taken from the project's own zelda_vanilla.sfc (MD5 03a63945398191337e896e5771f77173).
 * Add other revisions here if you ever support them.
 */
export const ACCEPTED_ROMS: AcceptedRom[] = [
	{
		label: 'The Legend of Zelda: A Link to the Past (Japan, v1.0)',
		sha1: 'e7e852f0159ce612e3911164878a9b08b3cb9060'
	}
];

export const ROM_SIZE = 0x100000; // 1 MiB
const COPIER_HEADER = 0x200; // 512 bytes some dumps prepend

export interface ValidRom {
	/** Headerless ROM bytes. */
	bytes: Uint8Array;
	sha1: string;
	/** Which accepted entry it matched. */
	label: string;
}

export type RomCheck = { ok: true; rom: ValidRom } | { ok: false; error: string };

async function sha1Hex(bytes: Uint8Array): Promise<string> {
	// Copy into a plain ArrayBuffer so this also type-checks for views over a SharedArrayBuffer.
	const digest = await crypto.subtle.digest('SHA-1', new Uint8Array(bytes));
	return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, '0')).join('');
}

/** Drop the 512-byte copier header if the size says there is one. */
export function stripHeader(bytes: Uint8Array): Uint8Array {
	return bytes.length === ROM_SIZE + COPIER_HEADER ? bytes.subarray(COPIER_HEADER) : bytes;
}

export function matchRom(sha1: string): AcceptedRom | undefined {
	return ACCEPTED_ROMS.find((r) => r.sha1 === sha1);
}

export async function checkRom(input: ArrayBuffer | Uint8Array): Promise<RomCheck> {
	const raw = input instanceof Uint8Array ? input : new Uint8Array(input);
	const bytes = stripHeader(raw);

	if (bytes.length !== ROM_SIZE) {
		return {
			ok: false,
			error: `That file is ${raw.length.toLocaleString()} bytes; the ROM should be ${ROM_SIZE.toLocaleString()} (1 MB).`
		};
	}
	if (!globalThis.crypto?.subtle) {
		return { ok: false, error: 'Your browser can’t check the ROM here (it needs a secure https page).' };
	}

	const sha1 = await sha1Hex(bytes);
	const match = matchRom(sha1);
	if (!match) {
		const expected = ACCEPTED_ROMS.map((r) => r.label).join(' or ');
		return { ok: false, error: `That isn’t the ROM we need. Please select: ${expected}.` };
	}
	return { ok: true, rom: { bytes: bytes.slice(), sha1, label: match.label } };
}
