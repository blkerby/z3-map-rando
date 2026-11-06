/**
 * IndexedDB persistence for the player's ROM. Every function swallows storage failures
 * (private windows, blocked site data, quota) and reports them as "nothing stored" / false,
 * so the app still works when the ROM can only be kept for the current page.
 */

const DB_NAME = 'alttp-maprando';
const STORE = 'kv';
const KEY = 'rom';

export interface StoredRom {
	bytes: ArrayBuffer;
	sha1: string;
	/** Original file name, only for display. */
	name: string;
	savedAt: number;
}

function open(): Promise<IDBDatabase> {
	return new Promise((resolve, reject) => {
		const req = indexedDB.open(DB_NAME, 1);
		req.onupgradeneeded = () => req.result.createObjectStore(STORE);
		req.onsuccess = () => resolve(req.result);
		req.onerror = () => reject(req.error);
		req.onblocked = () => reject(new Error('blocked'));
	});
}

async function run<T>(mode: IDBTransactionMode, fn: (s: IDBObjectStore) => IDBRequest<T>): Promise<T> {
	const db = await open();
	try {
		return await new Promise<T>((resolve, reject) => {
			const tx = db.transaction(STORE, mode);
			const req = fn(tx.objectStore(STORE));
			tx.oncomplete = () => resolve(req.result);
			tx.onerror = () => reject(tx.error);
			tx.onabort = () => reject(tx.error);
		});
	} finally {
		db.close();
	}
}

export async function loadStoredRom(): Promise<StoredRom | null> {
	try {
		const v = await run<unknown>('readonly', (s) => s.get(KEY));
		const r = v as Partial<StoredRom> | undefined;
		if (r && r.bytes instanceof ArrayBuffer && typeof r.sha1 === 'string' && typeof r.name === 'string') {
			return { bytes: r.bytes, sha1: r.sha1, name: r.name, savedAt: Number(r.savedAt) || 0 };
		}
	} catch {
		/* storage unavailable */
	}
	return null;
}

/** Returns true if the ROM was kept. */
export async function saveStoredRom(rom: StoredRom): Promise<boolean> {
	try {
		await run('readwrite', (s) => s.put(rom, KEY));
		// Ask the browser not to evict it under storage pressure. Best effort.
		void navigator.storage?.persist?.().catch(() => {});
		return true;
	} catch {
		return false;
	}
}

export async function clearStoredRom(): Promise<void> {
	try {
		await run('readwrite', (s) => s.delete(KEY));
	} catch {
		/* ignore */
	}
}
