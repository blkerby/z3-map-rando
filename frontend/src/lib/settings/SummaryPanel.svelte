<script lang="ts">
	import { resolve } from '$app/paths';
	import type { SettingsStore } from './store.svelte';
	import type { Section } from './types';

	let {
		store,
		generated = false,
		generating = false,
		error = null,
		seed = null,
		shareHash = null,
		ongenerate,
		onnew,
		onjump,
		onjumpsection,
		onsave
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
		/** Scroll to a section's card. */
		onjumpsection: (id: string) => void;
		/** Save the settings as a preset (see PresetManager's `save`). */
		onsave: () => void;
	} = $props();

	let copied = $state(false);

	const presetName = $derived(store.rootPreset.name);
	/** The selected top-level preset, if it is one of the player's own (saving goes into it). */
	const mine = $derived(store.listFor(store.root).isUser(store.rootPreset.id) ? store.rootPreset : undefined);

	const changed = $derived(
		store.changedIds.map((id) => ({
			id,
			label: store.byId[id]?.label ?? id,
			value: store.display(id)
		}))
	);
	const picked = $derived(store.changedSections);
	const changeCount = $derived(changed.length + picked.length);

	/** The selected presets' tweaks, by the section they are in. */
	const tweakGroups = $derived.by(() => {
		const groups: { section: Section; rows: { id: string; label: string; value: string }[] }[] = [];
		for (const id of store.tweakedIds) {
			if (store.isChanged(id)) continue; // listed as a change instead
			const section = store.ownerOf[id];
			let group = groups.find((g) => g.section === section);
			if (!group) {
				group = { section, rows: [] };
				groups.push(group);
			}
			group.rows.push({ id, label: store.byId[id]?.label ?? id, value: store.display(id) });
		}
		return groups;
	});
	const tweakCount = $derived(tweakGroups.reduce((n, g) => n + g.rows.length, 0));
	/** Folded or unfolded by hand; otherwise open only while there are no unsaved changes, so those stand out. */
	let tweaksOpenByHand = $state<boolean | null>(null);
	const tweaksOpen = $derived(tweaksOpenByHand ?? changeCount === 0);

	async function copyLink() {
		if (!shareHash) return;
		await navigator.clipboard.writeText(`${location.origin}${resolve('/generate')}#${shareHash}`);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<aside id="summary-panel" class="summary card">
	<div class="card-body">
		{#if generated}
			{#if seed}<div class="small mb-2"><strong>Seed</strong> <code class="seed">{seed}</code></div>{/if}
		{:else}
			<h2 class="h6 fw-bold mb-2">Generation settings</h2>
		{/if}

		<div class="small mb-2 text-body-secondary">
			{#if changeCount}
				{changeCount} change{changeCount === 1 ? '' : 's'} from <strong>{presetName}</strong>
			{:else}
				Matches <strong>{presetName}</strong>
			{/if}
		</div>

		{#if changeCount}
			<div class="chips mb-2">
				{#each picked as sec (sec.id)}
					<button type="button" class="chip" onclick={() => onjumpsection(sec.id)}>
						{sec.title} preset: <strong>{store.selectedPreset(sec).name}</strong>
					</button>
				{/each}
				{#each changed as c (c.id)}
					<button type="button" class="chip" onclick={() => onjump(c.id)}>
						{c.label}: <strong>{c.value}</strong>
					</button>
				{/each}
			</div>
			<button type="button" class="btn btn-link btn-sm p-0 mb-3" onclick={onsave}>
				<!-- No space after the icon, so the underline starts at the text. -->
				<i class="bi bi-floppy me-2"></i>{mine ? `Save to “${mine.name}”` : 'Save as preset…'}
			</button>
		{/if}

		{#if tweakCount}
			<button
				type="button"
				class="tweaks-toggle"
				aria-expanded={tweaksOpen}
				onclick={() => (tweaksOpenByHand = !tweaksOpen)}
			>
				<i class="bi" class:bi-chevron-down={tweaksOpen} class:bi-chevron-right={!tweaksOpen}></i>
				{tweakCount} tweak{tweakCount === 1 ? '' : 's'} in <strong>{presetName}</strong>
			</button>
			{#if tweaksOpen}
				{#each tweakGroups as g (g.section.id)}
					<div class="tweak-group">
						{g.section === store.root
							? presetName
							: `${g.section.title}: ${store.selectionLabel(g.section)}`}
					</div>
					<div class="chips mb-2">
						{#each g.rows as c (c.id)}
							<button type="button" class="chip tweak" onclick={() => onjump(c.id)}>
								{c.label}: <strong>{c.value}</strong>
							</button>
						{/each}
					</div>
				{/each}
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

	/* Fainter than a change, like the tweaked rows. */
	.chip.tweak {
		opacity: 0.8;
		border-style: dashed;
	}

	.tweaks-toggle {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		margin-bottom: 0.4rem;
		padding: 0;
		border: 0;
		background: none;
		color: var(--bs-secondary-color);
		font-size: 0.875rem;
		text-align: left;
	}

	.tweak-group {
		margin-bottom: 0.25rem;
		font-size: 0.78rem;
		color: var(--bs-secondary-color);
	}

	.seed {
		font-size: 0.9rem;
		letter-spacing: 0.05em;
		user-select: all;
	}
</style>
