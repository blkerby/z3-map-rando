<script lang="ts">
	import type { Snippet } from 'svelte';
	import { setStore } from './context';
	import type { SettingsStore } from './store.svelte';

	// Makes a store available to the setting components inside.
	let { store, children }: { store: SettingsStore; children: Snippet } = $props();

	// svelte-ignore state_referenced_locally
	setStore(store);
</script>

{@render children()}
