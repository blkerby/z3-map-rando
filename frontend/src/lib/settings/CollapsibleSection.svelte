<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		id,
		title,
		icon,
		badge,
		badgeIcon,
		open = $bindable(false),
		children
	}: {
		id: string;
		title: string;
		icon?: string;
		/** Small label next to the title, e.g. "Locked". */
		badge?: string;
		badgeIcon?: string;
		open?: boolean;
		children: Snippet;
	} = $props();
</script>

<section {id} class="section">
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
				<span class="badge rounded-pill text-bg-secondary ms-1 fw-normal">
					{#if badgeIcon}<i class="bi bi-{badgeIcon}" aria-hidden="true"></i>{/if}
					{badge}
				</span>
			{/if}
		</button>
	</h2>
	<!-- Kept mounted (just hidden) so expanders and scroll state survive collapsing. -->
	<div id="{id}-body" class="section-body" hidden={!open}>
		{@render children()}
	</div>
</section>

<style>
	.section {
		margin-bottom: 1.25rem;
	}

	.section-title {
		margin: 0;
		font-size: 1.25rem;
		font-weight: 700;
		border-bottom: 1px solid var(--bs-border-color);
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
