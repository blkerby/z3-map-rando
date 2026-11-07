<script lang="ts">
	import { onMount } from 'svelte';
	import PresetManager from './PresetManager.svelte';
	import ItemList from './ItemList.svelte';
	import type { SettingsStore } from './store.svelte';

	// The store is owned by the page (so the side panel and the search box can share it).
	// Customization is personal: saved in this browser only, never part of the game link, never locked.
	let { store }: { store: SettingsStore } = $props();

	const STORAGE_KEY = 'alttp-maprando:customization';
	const PRESETS_KEY = 'alttp-maprando:presets:customization';
	let ready = $state(false);

	onMount(() => {
		store.loadUserPresets(PRESETS_KEY, 1); // before the draft, which may name one of them
		try {
			const code = localStorage.getItem(STORAGE_KEY);
			if (code) store.load(code);
		} catch {
			/* storage unavailable: start from defaults */
		}
		ready = true;
	});

	$effect(() => {
		JSON.stringify(Object.values(store.lists).map((l) => l.userPresets)); // tracked: re-runs when any preset changes
		if (ready) store.saveUserPresets(PRESETS_KEY, 1);
	});

	$effect(() => {
		const code = store.serialize({ keepPreset: true }); // tracked: re-runs on any change
		if (!ready) return;
		try {
			localStorage.setItem(STORAGE_KEY, code);
		} catch {
			/* ignore */
		}
	});
</script>

<div class="mb-3">
	<PresetManager {store} kind="customization" selectId="custom-preset" />
</div>

<ItemList {store} items={store.items} idPrefix="custom-section-" />
{#if !store.countMatches()}
	<p class="text-body-secondary">No customization options match “{store.query}”.</p>
{/if}
