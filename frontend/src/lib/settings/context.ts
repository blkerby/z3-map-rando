import { getContext, setContext } from 'svelte';
import type { SettingsStore } from './store.svelte';

// Setting components find their store, card and layout through context, so pages can be written as plain markup.

const STORE = Symbol('settings-store');
const CARD = Symbol('settings-card');
const COMPACT = Symbol('settings-compact');

export function setStore(store: SettingsStore) {
	setContext(STORE, store);
}

export function getStore(): SettingsStore {
	return getContext(STORE);
}

export function setCardId(id: string) {
	setContext(CARD, id);
}

export function getCardId(): string | undefined {
	return getContext(CARD);
}

/** Rows in a compact group put the control beside the label. */
export function setCompact(compact: boolean) {
	setContext(COMPACT, compact);
}

export function getCompact(): boolean {
	return getContext(COMPACT) ?? false;
}
