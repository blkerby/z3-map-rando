<script lang="ts">
	import { onMount } from 'svelte';
	import { ITEM_LABELS, getItemLevelIcon, items } from '$lib/generation/data';
	import { computeSpoiler, type Spoiler, type SpoilerEntry } from '$lib/seed';

	let {
		seed,
		active,
		startingCounts
	}: {
		seed: string;
		/** Whether the list is visible; the replay runs the first time it is. */
		active: boolean;
		/** Starting copies of each item, so progressive items show the level each pickup reaches. */
		startingCounts: Record<string, number>;
	} = $props();

	interface Tile {
		/** Null for an item without an icon yet, shown with its fallback badge. */
		icon: string | null;
		badge: Badge;
		/** Item and location, for the tooltip and screen readers. */
		description: string;
	}

	interface Row {
		/** Position among all rows, so revealing is unaffected by which rows are shown. */
		index: number;
		title: string;
		tiles: Tile[];
	}

	interface Badge {
		text: string;
		color?: string;
	}

	/**
	 * Stand-ins for items whose icons haven't been drawn yet. An icon added to `assets/items` under the item's
	 * name replaces its badge automatically; any other item without an icon uses its label's initials.
	 */
	const FALLBACK_BADGES: Record<string, Badge> = {
		one_hundred_rupees: { text: '100' },
		arrow: { text: '1A' },
		ten_arrows: { text: '10A' },
		pendant_of_courage: { text: 'P', color: '#3c3' },
		pendant_of_wisdom: { text: 'P', color: '#e44' },
		pendant_of_power: { text: 'P', color: '#48f' },
		crystal1: { text: 'C1' },
		crystal2: { text: 'C2' },
		crystal3: { text: 'C3' },
		crystal4: { text: 'C4' },
		crystal5: { text: 'C5' },
		crystal6: { text: 'C6' },
		crystal7: { text: 'C7' }
	};

	/** Display order within a row: the order item kinds appear in items.json. */
	function getRank(name: string): number {
		const rank = items.findIndex((item) => item.name === name);
		return rank < 0 ? items.length : rank;
	}

	function getBadge(name: string): Badge {
		const badge = FALLBACK_BADGES[name];
		if (badge) return badge;
		const label = ITEM_LABELS[name] ?? name;
		let text = '';
		for (const word of label.split(' ')) text += word[0];
		return { text: text.slice(0, 3) };
	}

	let spoiler = $state<Spoiler | null>(null);
	let error = $state('');
	let started = false;
	/** Number of rows shown, from the top; the rest show question marks until revealed. */
	let revealed = $state(0);
	/** Whether to show only major- and high-tier items, hiding rows left empty. Remembered across sessions. */
	let importantOnly = $state(true);
	const IMPORTANT_ONLY_KEY = 'spoilerImportantOnly';

	onMount(() => {
		importantOnly = localStorage.getItem(IMPORTANT_ONLY_KEY) !== 'false';
	});

	$effect(() => {
		if (!active || started) return;
		started = true;
		computeSpoiler(seed)
			.then((result) => (spoiler = result))
			.catch((e) => (error = e instanceof Error ? e.message : String(e)));
	});

	const rows = $derived.by(() => {
		if (!spoiler) return [];
		// Progressive levels count copies in sphere order, then catalog order within a sphere.
		const levels: Record<string, number> = { ...startingCounts };
		const result: Row[] = [];
		const groups = [...spoiler.spheres];
		if (spoiler.unreachable.length) groups.push(spoiler.unreachable);
		for (const [index, group] of groups.entries()) {
			const ranked: { rank: number; tile: Tile }[] = [];
			for (const entry of group) {
				if (entry.item === null || entry.item === 'nothing') continue;
				const level = (levels[entry.item] ?? 0) + 1;
				levels[entry.item] = level;
				const tier = items.find((item) => item.name === entry.item)?.tier;
				if (importantOnly && tier !== 'major' && tier !== 'high') continue;
				ranked.push({
					rank: getRank(entry.item),
					tile: {
						icon: getItemLevelIcon(entry.item, level),
						badge: getBadge(entry.item),
						description: describe(entry.item, entry)
					}
				});
			}
			ranked.sort((a, b) => a.rank - b.rank);
			const tiles: Tile[] = [];
			for (const { tile } of ranked) tiles.push(tile);
			if (importantOnly && tiles.length === 0) continue;
			result.push({
				index,
				title: index < spoiler.spheres.length ? `${index + 1}` : 'Unreachable',
				tiles
			});
		}
		return result;
	});

	function describe(item: string, entry: SpoilerEntry): string {
		let label = ITEM_LABELS[item] ?? item;
		if (entry.door) label += ` (${entry.door})`;
		if (entry.dungeon) label += ` (${entry.dungeon})`;
		return `${label}\n${entry.location}`;
	}
</script>

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
	<div class="d-flex gap-2 mb-2">
		<button
			type="button"
			class="btn btn-sm btn-outline-secondary"
			onclick={() => (revealed = Infinity)}
		>
			Reveal all
		</button>
		<button type="button" class="btn btn-sm btn-outline-secondary" onclick={() => (revealed = 0)}>
			Hide all
		</button>
		<div class="form-check form-switch ms-auto mb-0 align-self-center">
			<input
				class="form-check-input"
				type="checkbox"
				role="switch"
				id="spoiler-important-only"
				bind:checked={importantOnly}
				onchange={() => localStorage.setItem(IMPORTANT_ONLY_KEY, String(importantOnly))}
			/>
			<label class="form-check-label" for="spoiler-important-only">
				Show only important items
			</label>
		</div>
	</div>
	<p class="small text-body-secondary">
		Each row is a sphere: the items collectible once all items in earlier rows are collected. Click
		a row to reveal it and the rows above it.
	</p>
	<div class="rows">
		{#each rows as row (row.index)}
			<button
				type="button"
				class="row-button"
				aria-label="Reveal rows through {row.title} and hide the rest"
				onclick={() => (revealed = row.index + 1)}
			>
				<span class="row-label">{row.title}</span>
				<span class="tiles">
					{#if row.index < revealed}
						{#each row.tiles as tile, tileIndex (tileIndex)}
							<span class="tile" title={tile.description}>
								{#if tile.icon}
									<img src={tile.icon} alt="" />
								{:else}
									<span class="badge-text" style:color={tile.badge.color}>{tile.badge.text}</span>
								{/if}
							</span>
						{/each}
					{:else}
						{#each row.tiles as tile (tile)}
							<span class="tile hidden-tile"
								><i class="bi bi-question-lg" aria-hidden="true"></i></span
							>
						{/each}
					{/if}
				</span>
			</button>
		{/each}
	</div>
{/if}

<style>
	.rows {
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: start;
		column-gap: 0.75rem;
	}

	.row-label {
		min-width: 1.5rem;
		padding-top: 0.375rem;
		text-align: right;
		font-weight: 600;
	}

	.tiles {
		display: flex;
		flex-wrap: wrap;
		gap: 2px;
	}

	/* Spans both columns of .rows, so the number and the gap beside it are part of the button. */
	.row-button {
		grid-column: 1 / -1;
		display: grid;
		grid-template-columns: subgrid;
		align-items: start;
		padding: 0.1875rem 0.375rem;
		border: none;
		border-radius: 4px;
		background: none;
		color: inherit;
		font: inherit;
		cursor: pointer;
		text-align: start;
	}

	.tile {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 32px;
		border-radius: 4px;
		background: var(--bs-tertiary-bg);
	}

	.tile img {
		width: 32px;
		height: 32px;
		image-rendering: pixelated;
	}

	.hidden-tile {
		color: var(--bs-secondary-color);
	}

	.row-button:hover {
		background: var(--bs-secondary-bg);
	}

	.row-button:hover .tile {
		background: none;
	}

	.badge-text {
		font-size: 0.7rem;
		font-weight: 700;
	}
</style>
