<script lang="ts">
	import SectionCard from './SectionCard.svelte';
	import SettingRow from './SettingRow.svelte';
	import type { SettingsStore } from './store.svelte';
	import type { Item, Section, Setting } from './types';

	let {
		store,
		items,
		idPrefix
	}: {
		store: SettingsStore;
		items: Item[];
		/** Prefix for the cards' element ids (the page's navigation scrolls to them). */
		idPrefix: string;
	} = $props();

	/** A run of settings under one heading, laid out as a list or a grid. */
	type Block = { kind: 'block'; heading: string; path: string[]; layout: 'list' | 'grid' | 'compact'; rows: Setting[] };

	// The items matching the search box, with consecutive settings under the same heading gathered into blocks.
	const entries = $derived.by(() => {
		const out: (Block | Section)[] = [];
		for (const item of items) {
			if (!store.matches(item)) continue;
			if (item.kind === 'section') {
				out.push(item);
				continue;
			}
			const last = out[out.length - 1];
			const heading = item.heading ?? '';
			if (last && last.kind === 'block' && last.heading === heading) {
				last.rows.push(item);
			} else {
				out.push({ kind: 'block', heading, path: item.headingPath ?? [], layout: item.layout ?? 'list', rows: [item] });
			}
		}
		return out;
	});
</script>

{#each entries as entry, i (entry.kind === 'section' ? `section|${entry.id}` : `block|${i}`)}
	{#if entry.kind === 'section'}
		<SectionCard {store} section={entry} {idPrefix} />
	{:else}
		<!-- Show only the heading levels that changed since the previous block. -->
		{@const before = entries[i - 1]}
		{@const prev = before?.kind === 'block' ? before.path : []}
		{@const from = entry.path.findIndex((part, j) => part !== prev[j])}
		{#if from >= 0}
			{#each entry.path.slice(from) as part, j (j)}
				{#if from + j === 0}
					<h4 class="subhead">{part}</h4>
				{:else}
					<h5 class="subsubhead">{part}</h5>
				{/if}
			{/each}
		{/if}
		<div class="block" class:grid={entry.layout === 'grid'} class:compact={entry.layout === 'compact'}>
			{#each entry.rows as s (s.id)}
				<SettingRow setting={s} {store} compact={entry.layout !== 'list'} />
			{/each}
		</div>
	{/if}
{/each}

<style>
	.subhead {
		margin: 1rem 0 0;
		padding-top: 0.6rem;
		border-top: 1px solid var(--bs-border-color-translucent);
		font-size: 0.8rem;
		font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		opacity: 0.65;
	}

	.subhead:first-child {
		margin-top: 0.4rem;
		border-top: 0;
	}

	.subsubhead {
		margin: 0.7rem 0 0.1rem;
		font-size: 0.75rem;
		font-weight: 600;
		opacity: 0.6;
	}

	/* One column of label-left rows; capped so the control stays near its label. */
	.block.compact {
		max-width: 32rem;
	}

	.block.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
		column-gap: 1.5rem;
		align-content: start;
	}
</style>
