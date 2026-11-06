import { getContext, setContext } from 'svelte';
import { checkRom, matchRom } from './validate';
import { clearStoredRom, loadStoredRom, saveStoredRom } from './storage';

export type RomStatus = 'loading' | 'missing' | 'ready';

const KEY = Symbol('rom-store');

export class RomStore {
	status = $state<RomStatus>('loading');
	/** File name of the loaded ROM, for display. */
	name = $state<string | null>(null);
	/** Why the last selected file was rejected (shown in the dialog). */
	error = $state<string | null>(null);
	/** True when the ROM couldn't be saved, so it only lasts until the page closes. */
	notRemembered = $state(false);
	busy = $state(false);
	dialogOpen = $state(false);

	// Raw bytes live outside $state so Svelte doesn't proxy 1 MB.
	#bytes: Uint8Array | null = null;
	#init: Promise<void> | null = null;
	#pending: { promise: Promise<boolean>; resolve: (ok: boolean) => void } | null = null;

	get ready(): boolean {
		return this.status === 'ready';
	}

	/** The headerless ROM, for patching. Null until a valid ROM is loaded. */
	get bytes(): Uint8Array | null {
		return this.#bytes;
	}

	/** Load a previously saved ROM. Safe to call more than once. */
	init(): Promise<void> {
		this.#init ??= (async () => {
			const stored = await loadStoredRom();
			// A saved ROM only counts if it still matches an accepted ROM (the list can change).
			if (stored && stored.bytes.byteLength > 0 && matchRom(stored.sha1)) {
				this.#bytes = new Uint8Array(stored.bytes);
				this.name = stored.name;
				this.status = 'ready';
			} else {
				if (stored) void clearStoredRom();
				this.status = 'missing';
			}
		})();
		return this.#init;
	}

	/**
	 * Resolves true once a valid ROM is loaded, asking for one if needed.
	 * Resolves false if the player closes the dialog without choosing a ROM.
	 */
	async require(): Promise<boolean> {
		await this.init();
		if (this.ready) return true;
		return this.choose();
	}

	/**
	 * Open the picker even if a ROM is already loaded (the "Change ROM" button).
	 * Resolves true once a valid ROM is chosen. Cancelling resolves false and keeps the current ROM.
	 */
	async choose(): Promise<boolean> {
		await this.init();
		if (this.#pending) return this.#pending.promise;
		let resolve!: (ok: boolean) => void;
		const promise = new Promise<boolean>((r) => (resolve = r));
		this.#pending = { promise, resolve };
		this.error = null;
		this.dialogOpen = true;
		return promise;
	}

	/** Validate and keep a file the player picked. Returns true on success. */
	async select(file: File): Promise<boolean> {
		if (this.busy) return false;
		this.busy = true;
		this.error = null;
		try {
			const result = await checkRom(await file.arrayBuffer());
			if (!result.ok) {
				this.error = result.error;
				return false;
			}
			this.#bytes = result.rom.bytes;
			this.name = file.name;
			this.status = 'ready';
			const kept = await saveStoredRom({
				bytes: result.rom.bytes.slice().buffer,
				sha1: result.rom.sha1,
				name: file.name,
				savedAt: Date.now()
			});
			this.notRemembered = !kept;
			this.#settle(true);
			return true;
		} catch {
			this.error = 'Couldn’t read that file. Please try again.';
			return false;
		} finally {
			this.busy = false;
		}
	}

	cancel() {
		this.#settle(false);
	}

	async clear() {
		this.#bytes = null;
		this.name = null;
		this.status = 'missing';
		this.notRemembered = false;
		await clearStoredRom();
	}

	#settle(ok: boolean) {
		this.dialogOpen = false;
		this.#pending?.resolve(ok);
		this.#pending = null;
	}
}

/** Call once, from the root layout. */
export function provideRomStore(): RomStore {
	const store = new RomStore();
	setContext(KEY, store);
	return store;
}

export function useRomStore(): RomStore {
	return getContext<RomStore>(KEY);
}
