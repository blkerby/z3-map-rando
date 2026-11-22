<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		id,
		title,
		icon,
		badge,
		badgeIcon,
		badgeTooltip,
		open = $bindable(false),
		actions,
		children
	}: {
		id: string;
		title: string;
		icon?: string;
		/** Small label next to the title, e.g. "Locked". */
		badge?: string;
		badgeIcon?: string;
		badgeTooltip?: string;
		open?: boolean;
		/** Controls shown at the end of the header, so they stay visible while the section is collapsed. */
		actions?: Snippet;
		children: Snippet;
	} = $props();

	let headerHeight = $state(0);
</script>

<!-- The header height is exposed so the cards' headers can stick just beneath this one. -->
<section {id} class="section" style:--section-header-height="{headerHeight}px">
	<div class="section-top" bind:offsetHeight={headerHeight}>
		<h2 class="section-title">
			<button
				type="button"
				class="section-header"
				aria-expanded={open}
				aria-controls="{id}-body"
				onclick={() => (open = !open)}
			>
				<i class="bi bi-chevron-right chevron" class:open aria-hidden="true"></i>
				{#if icon}<i class="bi bi-{icon}" aria-hidden="true"></i>{/if}
				<span>{title}</span>
				{#if badge}
					<span class="badge rounded-pill text-bg-secondary ms-1 fw-normal" title={badgeTooltip}>
						{#if badgeIcon}<i class="bi bi-{badgeIcon}" aria-hidden="true"></i>{/if}
						{badge}
					</span>
				{/if}
			</button>
		</h2>
		{#if actions}
			<div class="section-actions">{@render actions()}</div>
		{/if}
	</div>
	<!-- Kept mounted (just hidden) so expanders and scroll state survive collapsing. -->
	<div id="{id}-body" class="section-body" hidden={!open}>
		{@render children()}
	</div>
</section>

<style>
	.section {
		margin-bottom: 1.25rem;
	}

	/* Stays in view while scrolling through the section's body. */
	.section-top {
		position: sticky;
		top: 0;
		z-index: 2;
		background: var(--bs-body-bg);
		display: flex;
		align-items: center;
		border-bottom: 1px solid var(--bs-border-color);
	}

	/* Never squeezed: when the header is out of room, the preset label is cut off instead. */
	.section-title {
		flex: 1 0 auto;
		margin: 0;
		font-size: 1.25rem;
		font-weight: 700;
	}

	/* Shrinkable, so a long label is cut off rather than overflowing the header. Inset like the cards'
	   controls, so they line up with the cards' below. */
	.section-actions {
		flex: 0 1 auto;
		min-width: 0;
		padding: 0.3rem 0.75rem;
	}

	.section-header {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		width: 100%;
		padding: 0.6rem 0.25rem;
		border: 0;
		background: none;
		color: var(--bs-emphasis-color);
		font: inherit;
		text-align: left;
	}

	.section-header:hover {
		background: var(--bs-tertiary-bg);
	}

	.chevron {
		font-size: 0.9rem;
		transition: transform 0.15s;
	}

	.chevron.open {
		transform: rotate(90deg);
	}

	.badge {
		font-size: 0.75rem;
	}

	.section-body {
		padding-top: 1rem;
	}
</style>
