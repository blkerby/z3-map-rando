<script lang="ts">
	import { onMount } from 'svelte';
	import PresetManager from './PresetManager.svelte';
	import SettingRow from './SettingRow.svelte';
	import type { SettingsStore } from './store.svelte';

	// The store is owned by the page (so the side panel and the search box can share it).
	// Customization is personal: saved in this browser only, never part of the game link, never locked.
	let { store }: { store: SettingsStore } = $props();

	const STORAGE_KEY = 'alttp-maprando:customization';
	let ready = $state(false);

	onMount(() => {
		try {
			const code = localStorage.getItem(STORAGE_KEY);
			if (code) store.load(code);
		} catch {
			/* storage unavailable: start from defaults */
		}
		ready = true;
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
	<PresetManager {store} kind="customization" storageKey="alttp-maprando:presets:customization" selectId="custom-preset" />
</div>

{#each store.visibleGroups as { group, rows } (group.id)}
	<section id="custom-group-{group.id}" class="card mb-3">
		<header class="card-header">
			<h3 class="h6 fw-bold m-0">{group.title}</h3>
		</header>
		<div class="card-body py-1">
			{#each rows as s (s.id)}
				<SettingRow setting={s} {store} />
			{/each}
		</div>
	</section>
{:else}
	<p class="text-body-secondary">No customization options match “{store.query}”.</p>
{/each}
