<script lang="ts">
	import type { Snippet } from 'svelte';
	import { tooltip } from '$lib/tooltip';
	import ScopePicker from './ScopePicker.svelte';
	import { getStore, setCardId } from './context';
	import type { Scope } from './store.svelte';

	// A card of settings. Registers itself with the store for page navigation, and its settings register
	// as being in it, for its change count, search and jumping to a setting.
	let {
		id,
		title,
		icon,
		collapsible = false,
		collapsed = false,
		scope,
		header,
		children
	}: {
		id: string;
		title: string;
		/** bootstrap-icons suffix, e.g. "map". */
		icon?: string;
		collapsible?: boolean;
		/** A collapsible card starts folded. */
		collapsed?: boolean;
		/** The child scope whose preset is picked in this card's header. */
		scope?: Scope;
		/** Shown at the top of the body, e.g. a status line. */
		header?: Snippet;
		children: Snippet;
	} = $props();

	const store = getStore();
	// svelte-ignore state_referenced_locally
	setCardId(id);
	// svelte-ignore state_referenced_locally
	const card = { id, title, icon, collapsed: collapsible && collapsed };
	$effect(() => store.registerCard(card));

	const open = $derived(store.isOpen(card));
	const changes = $derived(store.countCardChanges(id, scope));
	const hidden = $derived(store.query.trim() !== '' && store.countMatches(id) === 0);
</script>

<section id="{store.idPrefix}section-{id}" class="card mb-3" {hidden}>
	<!-- No padding on the header itself, so the controls on the right line up whether or not the card can fold. -->
	<header class="card-header d-flex align-items-center p-0" class:folded={!open}>
		<h3 class="h6 fw-bold m-0 flex-grow-1" class:card-title={!collapsible}>
			{#if collapsible}
				<button
					type="button"
					class="card-toggle"
					aria-expanded={open}
					aria-controls="{store.idPrefix}body-{id}"
					onclick={() => (store.opened[id] = !open)}
				>
					<i class="bi" class:bi-chevron-down={open} class:bi-chevron-right={!open}></i>
					<span>{title}</span>
					{#if !open && changes}<span class="changes">{changes} changed</span>{/if}
				</button>
			{:else}
				{title}
			{/if}
		</h3>
		{#if scope}
			<!-- In the header, so the preset can be seen and changed while the card is folded. -->
			<div class="card-presets">
				<ScopePicker {store} {scope} selectId="{store.idPrefix}preset-{id}" />
			</div>
		{/if}
		{#if changes}
			<button
				type="button"
				class="btn btn-link btn-sm p-0 card-reset"
				disabled={store.locked}
				use:tooltip={`Undo the changes in ${title}. The other sections are not touched.`}
				aria-label="Reset {title}"
				onclick={() => store.resetCard(id, scope)}
			>
				<i class="bi bi-arrow-counterclockwise"></i>
			</button>
		{/if}
	</header>
	<!-- Kept mounted while folded, so its settings stay known to the summary and search. -->
	<div class="card-body py-1" id="{store.idPrefix}body-{id}" hidden={!open}>
		{@render header?.()}
		{@render children()}
	</div>
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

	.changes {
		font-size: 0.75rem;
		font-weight: 400;
		opacity: 0.7;
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

	/* Same padding as the toggle of a card that can fold. */
	.card-title {
		padding: 0.5rem 1rem;
	}

	.card-header.folded {
		border-bottom: 0;
	}
</style>
