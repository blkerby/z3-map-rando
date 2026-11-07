<script lang="ts">
	import { tooltip } from '$lib/tooltip';
	import type { SettingsStore } from './store.svelte';
	import { downloadRom } from '$lib/game';

	let {
		store,
		generated = false,
		seed = null,
		version = '',
		onjump,
		onsave
	}: {
		store: SettingsStore;
		/** True once a game exists, which is when there is a ROM to download. */
		generated?: boolean;
		seed?: string | null;
		version?: string;
		/** Scroll to a setting (the page also clears the search and opens its section). */
		onjump: (id: string) => void;
		/** Save the customization as a preset (see PresetManager's `save`). */
		onsave: () => void;
	} = $props();

	/** The selected top-level preset, if it is one of the player's own (saving goes into it). */
	const mine = $derived(store.listFor(store.root).isUser(store.rootPreset.id) ? store.rootPreset : undefined);

	const changed = $derived(
		store.changedIds.map((id) => ({
			id,
			label: store.byId[id]?.label ?? id,
			value: store.display(id)
		}))
	);

	// ---- Download ROM (stub: see downloadRom in $lib/game) ---------------------
	let downloading = $state(false);
	let romError = $state<string | null>(null);

	async function download() {
		if (!seed || downloading) return;
		downloading = true;
		romError = null;
		try {
			await downloadRom({ seed, version, customization: $state.snapshot(store.values) });
		} catch (e) {
			romError = e instanceof Error ? e.message : 'Could not download the ROM.';
		} finally {
			downloading = false;
		}
	}
</script>

<aside id="customization-panel" class="card">
	<div class="card-body">
		<h2 class="h6 fw-bold mb-2">Your customization</h2>

		{#if changed.length}
			<div class="small mb-2 text-body-secondary">
				{changed.length} change{changed.length === 1 ? '' : 's'} from <strong>{store.presetSummary}</strong>
			</div>
			<div class="chips mb-2">
				{#each changed as c (c.id)}
					<button type="button" class="chip" onclick={() => onjump(c.id)}>
						{c.label}: <strong>{c.value}</strong>
					</button>
				{/each}
			</div>
			<button type="button" class="btn btn-link btn-sm p-0" onclick={onsave}>
				<!-- No space after the icon, so the underline starts at the text. -->
				<i class="bi bi-floppy me-2"></i>{mine ? `Save to “${mine.name}”` : 'Save as preset…'}
			</button>
		{:else}
			<div class="small text-body-secondary">
				Everything matches <strong>{store.presetSummary}</strong>.
			</div>
		{/if}

		{#if romError}
			<div class="alert alert-danger small py-2 px-3 mt-3 mb-0" role="alert">{romError}</div>
		{/if}

		<div class="d-grid gap-2 mt-3" use:tooltip={generated ? undefined : 'Generate a game first, before a ROM can be saved.'}>
			<button type="button" class="btn btn-primary" disabled={!generated || downloading} onclick={download}>
				{#if downloading}
					<span class="spinner-border spinner-border-sm me-1" aria-hidden="true"></span>
					Preparing…
				{:else}
					<i class="bi bi-download"></i> Save ROM
				{/if}
			</button>
		</div>
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
