<script lang="ts">
	import { resolve } from '$app/paths';
	import type { SettingsStore } from './store.svelte';

	let {
		store,
		generated = false,
		generating = false,
		error = null,
		seed = null,
		shareHash = null,
		ongenerate,
		onnew,
		onjump
	}: {
		store: SettingsStore;
		/** True once a game exists for these settings (the settings are locked). */
		generated?: boolean;
		generating?: boolean;
		error?: string | null;
		seed?: string | null;
		/** Hash for the share link (version.seed.settings); null until a game exists. */
		shareHash?: string | null;
		ongenerate: () => void;
		onnew: () => void;
		/** Scroll to a setting (the page also clears the search and opens its section). */
		onjump: (id: string) => void;
	} = $props();

	let copied = $state(false);

	const presetName = $derived(store.presetName);

	const changed = $derived(
		store.changedIds.map((id) => ({
			id,
			label: store.byId[id]?.label ?? id,
			value: store.display(id)
		}))
	);

	async function copyLink() {
		if (!shareHash) return;
		await navigator.clipboard.writeText(`${location.origin}${resolve('/generate')}#${shareHash}`);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<aside id="summary-panel" class="summary card">
	<div class="card-body">
		<h2 class="h6 fw-bold mb-2">{generated ? 'Your game' : 'Your settings'}</h2>

		{#if generated && seed}
			<div class="small mb-2">Seed <code class="seed">{seed}</code></div>
		{/if}

		<div class="small mb-2 text-body-secondary">
			{#if changed.length}
				{changed.length} change{changed.length === 1 ? '' : 's'} from <strong>{presetName}</strong>
			{:else}
				Matches <strong>{presetName}</strong>
			{/if}
		</div>

		{#if changed.length}
			<div class="chips mb-2">
				{#each changed as c (c.id)}
					<button type="button" class="chip" onclick={() => onjump(c.id)}>
						{c.label}: <strong>{c.value}</strong>
					</button>
				{/each}
			</div>
			{#if !generated}
				<button type="button" class="btn btn-link btn-sm p-0 mb-3" onclick={() => store.resetAll()}>
					Reset all
				</button>
			{/if}
		{/if}

		{#if error}
			<div class="alert alert-danger small py-2 px-3" role="alert">{error}</div>
		{/if}
		{#each store.problems as problem (problem)}
			<div class="alert alert-warning small py-2 px-3" role="alert">{problem}</div>
		{/each}

		<div class="d-grid gap-2 mt-3">
			{#if generated}
				<button type="button" class="btn btn-primary" onclick={copyLink}>
					<i class="bi bi-link-45deg"></i>
					{copied ? 'Copied!' : 'Copy game link'}
				</button>
				<button type="button" class="btn btn-outline-secondary btn-sm" onclick={onnew}>
					<i class="bi bi-arrow-repeat"></i>
					Generate new game
				</button>
			{:else}
				<button
					type="button"
					class="btn btn-primary"
					disabled={generating || store.problems.length > 0}
					onclick={ongenerate}
				>
					{#if generating}
						<span class="spinner-border spinner-border-sm me-1" aria-hidden="true"></span>
						Generating…
					{:else}
						Generate game
					{/if}
				</button>
			{/if}
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

	.seed {
		font-size: 0.9rem;
		letter-spacing: 0.05em;
		user-select: all;
	}
</style>
