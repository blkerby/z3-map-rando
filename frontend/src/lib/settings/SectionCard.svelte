<script lang="ts">
	import { tooltip } from '$lib/tooltip';
	import ItemList from './ItemList.svelte';
	import PresetManager from './PresetManager.svelte';
	import type { SettingsStore } from './store.svelte';
	import type { Section } from './types';

	let { store, section, idPrefix }: { store: SettingsStore; section: Section; idPrefix: string } = $props();

	const open = $derived(store.isOpen(section));
	const pool = $derived(store.poolOf(section));
</script>

<section id="{idPrefix}{section.id}" class="card mb-3">
	<header class="card-header d-flex align-items-center" class:p-0={section.collapsible} class:folded={!open}>
		<h3 class="h6 fw-bold m-0 flex-grow-1">
			{#if section.collapsible}
				<button
					type="button"
					class="card-toggle"
					aria-expanded={open}
					aria-controls="{idPrefix}body-{section.id}"
					onclick={() => (store.opened[section.id] = !open)}
				>
					<i class="bi" class:bi-chevron-down={open} class:bi-chevron-right={!open}></i>
					<span>{section.title}</span>
				</button>
			{:else}
				{section.title}
			{/if}
		</h3>
		<!-- In the header, so the card's preset can be seen and changed while it is folded. -->
		<div class="card-presets">
			<PresetManager {store} {section} kind={section.id} selectId="{idPrefix}preset-{section.id}" />
		</div>
		{#if store.sectionChanged(section)}
			<button
				type="button"
				class="btn btn-link btn-sm p-0 card-reset"
				disabled={store.locked}
				use:tooltip={`Undo the changes in ${section.title}. The other sections are not touched.`}
				aria-label="Reset {section.title}"
				onclick={() => store.resetSection(section)}
			>
				<i class="bi bi-arrow-counterclockwise"></i>
			</button>
		{/if}
	</header>
	{#if open}
		<div class="card-body py-1" id="{idPrefix}body-{section.id}">
			{#if pool}
				<div class="pool-bar" class:text-danger={pool.used > pool.total}>
					{pool.used} of {pool.total} item locations filled{pool.used < pool.total ? ' (the rest are left empty)' : ''}.
				</div>
			{/if}
			<ItemList {store} items={section.items} {idPrefix} />
		</div>
	{/if}
</section>

<style>
	section {
		scroll-margin-top: 1rem;
	}

	.card-toggle {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.5rem 1rem;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
	}

	.card-reset {
		flex: none;
		margin-right: 0.75rem;
		text-decoration: none;
	}

	/* Shrinkable rather than capped, so the controls only wrap when the header is actually out of room. */
	.card-presets {
		flex: 0 1 auto;
		padding: 0.3rem 0.75rem 0.3rem 0;
		font-size: 0.9rem;
	}

	.card-header.folded {
		border-bottom: 0;
	}

	.pool-bar {
		padding: 0.5rem 0;
		font-size: 0.8rem;
		opacity: 0.9;
	}
</style>
