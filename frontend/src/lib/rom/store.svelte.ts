import { getContext, setContext } from 'svelte';
import { checkRom, matchRom } from './validate';
import { clearStoredRom, loadStoredRom, saveStoredRom } from './storage';

const KEY = Symbol('rom-store');

export class RomStore {
	/** Why the last selected file was rejected (shown in the dialog). */
	error = $state<string | null>(null);
	busy = $state(false);
	dialogOpen = $state(false);

	// Raw bytes live outside $state so Svelte doesn't proxy 1 MB.
	#bytes: Uint8Array | null = null;
	#init: Promise<void> | null = null;
	#pending: { promise: Promise<boolean>; resolve: (ok: boolean) => void } | null = null;

	/** The headerless ROM, for patching. Null until a valid ROM is loaded. */
	get bytes(): Uint8Array | null {
		return this.#bytes;
	}

	/** Load a previously saved ROM. Safe to call more than once. */
	#loadStored(): Promise<void> {
		this.#init ??= (async () => {
			const stored = await loadStoredRom();
			// A saved ROM only counts if it still matches an accepted ROM (the list can change).
			if (stored && stored.bytes.byteLength > 0 && matchRom(stored.sha1)) {
				this.#bytes = new Uint8Array(stored.bytes);
			} else if (stored) {
				void clearStoredRom();
			}
		})();
		return this.#init;
	}

	/**
	 * Resolves true once a valid ROM is loaded, asking for one if needed.
	 * Resolves false if the player closes the dialog without choosing a ROM.
	 */
	async require(): Promise<boolean> {
		await this.#loadStored();
		if (this.#bytes) return true;
		if (this.#pending) return this.#pending.promise;
		let resolve!: (ok: boolean) => void;
		const promise = new Promise<boolean>((r) => (resolve = r));
		this.#pending = { promise, resolve };
		this.error = null;
		this.dialogOpen = true;
		return promise;
	}

	/** Validate and keep a file the player picked. */
	async select(file: File) {
		if (this.busy) return;
		this.busy = true;
		this.error = null;
		try {
			const result = await checkRom(await file.arrayBuffer());
			if (!result.ok) {
				this.error = result.error;
				return;
			}
			this.#bytes = result.rom.bytes;
			await saveStoredRom({ bytes: result.rom.bytes.slice().buffer, sha1: result.rom.sha1 });
			this.#settle(true);
		} catch {
			this.error = 'Couldn’t read that file. Please try again.';
		} finally {
			this.busy = false;
		}
	}

	cancel() {
		this.#settle(false);
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
