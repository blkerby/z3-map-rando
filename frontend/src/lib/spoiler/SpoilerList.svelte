<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { ITEM_LABELS, getItemLevelIcon, items } from '$lib/generation/data';
	import {
		openSpoiler,
		type RouteRow,
		type RouteState,
		type Spoiler,
		type SpoilerEntry,
		type SpoilerSession
	} from '$lib/seed';

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
		locationIdx: number;
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
	let session: SpoilerSession | null = null;
	let destroyed = false;
	let error = $state('');
	let started = false;

	interface Route {
		tile: Tile;
		item: string;
		location: string;
		/** Null if no route was found. */
		steps: RouteRow[] | null;
	}

	let route = $state<Route | null>(null);
	let routeDialog = $state<HTMLDialogElement>();
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
		openSpoiler(seed)
			.then((result) => {
				if (destroyed) {
					result.free();
					return;
				}
				session = result;
				spoiler = result.spoiler;
			})
			.catch((e) => (error = e instanceof Error ? e.message : String(e)));
	});

	onDestroy(() => {
		destroyed = true;
		session?.free();
	});

	function showRoute(tile: Tile) {
		if (!session) return;
		// A tile's description is the item label, then the location on its own line.
		const [item, location] = tile.description.split('\n');
		route = {
			tile,
			item,
			location,
			steps: session.traceRoute(tile.locationIdx)
		};
		routeDialog?.showModal();
	}

	interface StatePart {
		icon?: string | null;
		/** Alternative text for the icon. */
		label?: string;
		text: string;
	}

	/** State before any usage, which the first row is compared against. */
	const UNUSED_STATE: RouteState = {
		form: 'Link',
		health_used: 0,
		health_remaining: null,
		magic_used: 0,
		bombs_used: 0,
		arrows_used: 0,
		bottles_used: 0,
		followers: []
	};

	/** Describe the components that differ from the previous row, or on the first row from an unused state. */
	function describeState(state: RouteState, previous: RouteState = UNUSED_STATE): StatePart[] {
		const parts: StatePart[] = [];
		if (state.form !== previous.form) parts.push({ text: state.form });
		if (
			state.health_used !== previous.health_used ||
			state.health_remaining !== previous.health_remaining
		) {
			parts.push({
				icon: getItemLevelIcon('heart_container', 1),
				label: state.health_remaining === null ? 'Hearts used' : 'Hearts remaining',
				text:
					state.health_remaining === null
						? `${(state.health_used ?? 0) / 8}`
						: `${state.health_remaining / 8} left`
			});
		}
		const counters: [number, number, string, string][] = [
			[state.magic_used, previous.magic_used, 'half_magic', 'Magic used'],
			[state.bombs_used, previous.bombs_used, 'bomb', 'Bombs used'],
			[state.arrows_used, previous.arrows_used, 'bow', 'Arrows used'],
			[state.bottles_used, previous.bottles_used, 'bottle', 'Bottles used']
		];
		for (const [value, previousValue, icon, label] of counters) {
			if (value !== previousValue)
				parts.push({ icon: getItemLevelIcon(icon, 1), label, text: `${value}` });
		}
		const followers = state.followers.join(' or ');
		if (followers !== previous.followers.join(' or ')) {
			parts.push({ text: followers ? `Follower: ${followers}` : 'No follower' });
		}
		return parts;
	}

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
						description: describe(entry.item, entry),
						locationIdx: entry.location_idx
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
		a row to reveal it and the rows above it, and click a revealed item to show a route to it.
	</p>
	<div class="rows">
		{#each rows as row (row.index)}
			<!-- The row number is the keyboard-accessible reveal control; its clicks bubble here. -->
			<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
			<div class="row-button" onclick={() => (revealed = row.index + 1)}>
				<button
					type="button"
					class="row-label"
					aria-label="Reveal rows through {row.title} and hide the rest">{row.title}</button
				>
				<span class="tiles">
					{#if row.index < revealed}
						{#each row.tiles as tile, tileIndex (tileIndex)}
							<button
								type="button"
								class="tile"
								title={tile.description}
								aria-label="Show a route to {tile.description}"
								onclick={(event) => {
									event.stopPropagation();
									showRoute(tile);
								}}
							>
								{#if tile.icon}
									<img src={tile.icon} alt="" />
								{:else}
									<span class="badge-text" style:color={tile.badge.color}>{tile.badge.text}</span>
								{/if}
							</button>
						{/each}
					{:else}
						{#each row.tiles as tile (tile)}
							<span class="tile hidden-tile"
								><i class="bi bi-question-lg" aria-hidden="true"></i></span
							>
						{/each}
					{/if}
				</span>
			</div>
		{/each}
	</div>
	<!-- Escape, Close or a click on the backdrop closes it. `autofocus` makes `showModal()` focus the dialog
	     itself rather than its close button. -->
	<dialog
		bind:this={routeDialog}
		autofocus
		class="route-dialog"
		aria-labelledby="spoiler-route-title"
		onclick={(e) => {
			if (e.target === routeDialog) routeDialog.close();
		}}
		onclose={() => (route = null)}
	>
		{#if route}
			<div class="d-flex align-items-center gap-2 px-4 py-3 border-bottom">
				<span class="tile">
					{#if route.tile.icon}
						<img src={route.tile.icon} alt="" />
					{:else}
						<span class="badge-text" style:color={route.tile.badge.color}
							>{route.tile.badge.text}</span
						>
					{/if}
				</span>
				<div class="me-auto">
					<h2 id="spoiler-route-title" class="h5 fw-bold mb-0">Route to {route.item}</h2>
					<div class="small text-body-secondary">{route.location}</div>
				</div>
				<button
					type="button"
					class="btn-close"
					aria-label="Close"
					onclick={() => routeDialog?.close()}
				></button>
			</div>
			<div class="p-4">
				{#if route.steps}
					<table class="table table-sm small route mb-0">
						<thead>
							<tr>
								<th scope="col">Destination</th>
								<th scope="col">Strat</th>
								<th scope="col">State</th>
							</tr>
						</thead>
						<tbody>
							{#each route.steps as step, stepIndex (stepIndex)}
								<tr>
									<td>{step.destination}</td>
									<td>
										{step.strat}
										{#each step.dependencies as dependency (dependency)}
											<div class="text-body-secondary">{dependency}</div>
										{/each}
									</td>
									<td>
										{#each describeState(step.state, route.steps[stepIndex - 1]?.state) as part, partIndex (partIndex)}
											<span class="state-part">
												{#if part.icon}<img
														src={part.icon}
														alt={part.label}
														title={part.label}
													/>{/if}{part.text}
											</span>
										{/each}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				{:else}
					<p class="text-body-secondary mb-0">No route found.</p>
				{/if}
			</div>
		{/if}
	</dialog>
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
		padding: 0.375rem 0 0;
		border: none;
		background: none;
		color: inherit;
		font: inherit;
		text-align: right;
		font-weight: 600;
		cursor: pointer;
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
		padding: 0;
		border: none;
		color: inherit;
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

	button.tile {
		transition: transform 0.1s;
	}

	button.tile:hover {
		position: relative;
		z-index: 1;
		transform: scale(1.5);
	}

	.route-dialog {
		width: min(60rem, calc(100vw - 2rem));
		padding: 0;
		border: 1px solid var(--bs-border-color);
		border-radius: 0.6rem;
		background: var(--bs-body-bg);
		color: var(--bs-body-color);
	}

	.route-dialog::backdrop {
		background: rgb(0 0 0 / 0.55);
	}

	.state-part {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		margin-right: 0.5rem;
	}

	.state-part img {
		width: 16px;
		height: 16px;
		image-rendering: pixelated;
	}

	.route td {
		overflow-wrap: anywhere;
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
