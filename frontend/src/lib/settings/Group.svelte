<script lang="ts">
	import type { Snippet } from 'svelte';
	import { setCompact } from './context';

	// A run of settings under an optional heading. Hidden when the search box leaves none of its rows.
	let {
		heading,
		subheading = false,
		layout = 'list',
		children
	}: {
		heading?: string;
		/** A smaller heading, for a group inside another heading. */
		subheading?: boolean;
		/** One full row per line, compact rows in one column, or compact rows in columns. */
		layout?: 'list' | 'compact' | 'grid';
		children: Snippet;
	} = $props();

	// svelte-ignore state_referenced_locally
	setCompact(layout !== 'list');
</script>

<div class="group">
	{#if heading}
		{#if subheading}
			<h5 class="subsubhead">{heading}</h5>
		{:else}
			<h4 class="subhead">{heading}</h4>
		{/if}
	{/if}
	<div class="rows" class:grid={layout === 'grid'} class:compact={layout === 'compact'}>
		{@render children()}
	</div>
</div>

<style>
	.group:not(:has(:global(.setting-row:not([hidden])))) {
		display: none;
	}

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

	.group:first-child > .subhead {
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
	.rows.compact {
		max-width: 32rem;
	}

	.rows.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
		column-gap: 1.5rem;
		align-content: start;
	}
</style>
