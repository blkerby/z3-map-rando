<script lang="ts">
	import { ITEM_LABELS } from '$lib/generation/data';
	import { computeSpoiler, type Spoiler, type SpoilerEntry } from '$lib/seed';

	let {
		seed,
		active
	}: {
		seed: string;
		/** Whether the list is visible; the replay runs the first time it is. */
		active: boolean;
	} = $props();

	let spoiler = $state<Spoiler | null>(null);
	let error = $state('');
	let started = false;

	$effect(() => {
		if (!active || started) return;
		started = true;
		computeSpoiler(seed)
			.then((result) => (spoiler = result))
			.catch((e) => (error = e instanceof Error ? e.message : String(e)));
	});

	function getItemLabel(entry: SpoilerEntry): string {
		if (entry.item === null) return '—';
		if (entry.item === 'nothing') return 'Nothing';
		const label = ITEM_LABELS[entry.item] ?? entry.item;
		return entry.door ? `${label} (${entry.door})` : label;
	}
</script>

{#snippet entries(list: SpoilerEntry[])}
	<ul class="list-unstyled mb-0 entries">
		{#each list as entry, index (index)}
			<li>
				<span class="item">{getItemLabel(entry)}</span>
				<span class="location text-body-secondary">{entry.location}</span>
			</li>
		{/each}
	</ul>
{/snippet}

{#if error}
	<div class="alert alert-danger small py-2 px-3" role="alert">
		Couldn't compute the spoiler: {error}
	</div>
{:else if !spoiler}
	<p class="text-body-secondary" role="status">
		<span class="spinner-border spinner-border-sm me-1" aria-hidden="true"></span>
		Computing spheres…
	</p>
{:else}
	{#each spoiler.spheres as sphere, index (index)}
		<details class="sphere">
			<summary>
				<span class="fw-semibold">Sphere {index + 1}</span>
				<span class="text-body-secondary"
					>· {sphere.length} {sphere.length === 1 ? 'item' : 'items'}</span
				>
			</summary>
			{@render entries(sphere)}
		</details>
	{/each}
	{#if spoiler.unreachable.length}
		<details class="sphere">
			<summary>
				<span class="fw-semibold">Unreachable</span>
				<span class="text-body-secondary"
					>· {spoiler.unreachable.length}
					{spoiler.unreachable.length === 1 ? 'item' : 'items'}</span
				>
			</summary>
			{@render entries(spoiler.unreachable)}
		</details>
	{/if}
{/if}

<style>
	.sphere {
		border-bottom: 1px solid var(--bs-border-color);
		padding: 0.5rem 0;
	}

	summary {
		cursor: pointer;
	}

	.entries {
		padding: 0.25rem 0 0.25rem 1.25rem;
	}

	.entries li {
		display: flex;
		flex-wrap: wrap;
		column-gap: 0.75rem;
		padding: 0.125rem 0;
	}

	.item {
		min-width: 12rem;
	}
</style>
