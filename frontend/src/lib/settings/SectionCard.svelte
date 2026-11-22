<script lang="ts">
	import type { Snippet } from 'svelte';
	import ScopePicker from './ScopePicker.svelte';
	import { getStore, setCardId } from './context';
	import { narrow } from './narrow';
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
					<!-- A scope's preset dropdown already shows its unsaved changes. -->
					{#if !open && changes && !scope}<span class="changes">{changes} changed</span>{/if}
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
		<!-- Left out on narrow screens for room: choosing the inherited preset in the dropdown does the same. -->
		{#if changes && !narrow.current}
			<!-- Bootstrap's disabled buttons ignore the pointer, so the marker goes on a wrapper. -->
			<span data-lockable>
				<button
					type="button"
					class="btn btn-link btn-sm p-0 card-reset"
					disabled={store.locked}
					title={`Undo the changes in ${title}. The other sections are not touched.`}
					aria-label="Reset {title}"
					onclick={() => store.resetCard(id, scope)}
				>
					<i class="bi bi-arrow-counterclockwise"></i>
				</button>
			</span>
		{/if}
	</header>
	<!-- Kept mounted while folded, so its settings stay known to the summary and search. -->
	<div class="card-body py-1" id="{store.idPrefix}body-{id}" hidden={!open}>
		{@render children()}
	</div>
</section>

<style>
	/* Lands beneath the stuck section header. */
	section {
		scroll-margin-top: calc(var(--section-header-height, 0px) + 0.5rem);
	}

	/* Stays in view while scrolling through the card, beneath the stuck section header. The cap color is
	   translucent, so it is layered over the card's own background. */
	.card-header {
		position: sticky;
		top: var(--section-header-height, 0px);
		z-index: 1;
		background: linear-gradient(var(--bs-card-cap-bg), var(--bs-card-cap-bg)) var(--bs-card-bg);
	}

	/* Never squeezed: when the header is out of room, the preset label is cut off instead. */
	h3 {
		flex: 1 0 auto;
	}

	.card-toggle {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.5rem 1rem 0.5rem 0.5rem;
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

	/* Shrinkable, so a long label is cut off rather than overflowing the card. */
	.card-presets {
		flex: 0 1 auto;
		min-width: 0;
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
