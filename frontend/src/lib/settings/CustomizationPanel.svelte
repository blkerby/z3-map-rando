<script lang="ts">
	import type { SettingsStore } from './store.svelte';
	import type { Path } from './tree';

	let {
		store,
		onjump,
		onsave
	}: {
		store: SettingsStore;
		/** Scroll to a setting (the page also clears the search and opens its card). */
		onjump: (path: Path) => void;
		/** Save the customization as a preset (see PresetManager's `save`). */
		onsave: () => void;
	} = $props();

	/** The selected preset, if it is one of the player's own (saving goes into it). */
	const mine = $derived(store.userPreset(store.selectedId));
	const changed = $derived(store.changedPaths);
</script>

<aside id="customization-panel" class="card">
	<div class="card-body">
		<h2 class="h6 fw-bold mb-2">Customization settings</h2>

		{#if changed.length}
			<div class="small mb-2 text-body-secondary">
				{changed.length} change{changed.length === 1 ? '' : 's'} from <strong>{store.presetName}</strong>
			</div>
			<div class="chips mb-2">
				{#each changed as path (path.join('/'))}
					<button type="button" class="chip" onclick={() => onjump(path)}>
						{store.label(path)}: <strong>{store.display(path)}</strong>
					</button>
				{/each}
			</div>
			<button type="button" class="btn btn-link btn-sm p-0" onclick={onsave}>
				<!-- No space after the icon, so the underline starts at the text. -->
				<i class="bi bi-floppy me-2"></i>{mine ? `Save to “${mine.name}”` : 'Save as preset…'}
			</button>
		{:else}
			<div class="small text-body-secondary">
				Matches <strong>{store.presetName}</strong>
			</div>
		{/if}

	</div>
</aside>

<style>
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}

	.chip {
		border: 1px solid var(--bs-border-color);
		/* Buttons center their text; long chips wrap, so keep the lines left-aligned. */
		border-radius: 0.9rem;
		padding: 0.1rem 0.6rem;
		background: var(--bs-body-bg);
		color: var(--bs-body-color);
		font-size: 0.78rem;
		text-align: left;
	}

	.chip:hover {
		border-color: var(--bs-primary);
	}
</style>
