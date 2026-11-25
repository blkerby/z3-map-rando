<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import linkHead from '$lib/assets/link_head.png';
	import { ITEM_LABELS, getItemLevelIcon, items } from '$lib/generation/data';
	import { useRomStore } from '$lib/rom/store.svelte';
	import Segmented from '$lib/settings/controls/Segmented.svelte';
	import {
		openSpoiler,
		type MapPoint,
		type RouteRow,
		type RouteState,
		type RouteTarget,
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
		/** Item label, including a Small Key's door or a dungeon item's dungeon. */
		item: string;
		location: string;
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
		target: RouteTarget;
		/** The target item, or null for a route to a discovery. */
		tile: Tile | null;
		title: string;
		subtitle: string;
		/** Null if no route was found. */
		steps: RouteRow[] | null;
	}

	const TARGET_KINDS: Record<RouteTarget['kind'], string> = {
		location: 'Item',
		flag: 'Flag',
		door: 'Door',
		ocarina: 'Ocarina'
	};

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
		for (const url of worldMaps ?? []) URL.revokeObjectURL(url);
	});

	const rom = useRomStore();
	/** PNG object URLs for the Light and Dark World maps, rendered once from the player's ROM. */
	let worldMaps = $state<string[] | null>(null);
	let mapError = $state('');
	/** Whether rendering has started; the map renders the first time a route is shown, if a ROM is saved. */
	let mapStarted = $state(false);
	let mapWorld = $state('light');
	const WORLD_OPTIONS = [
		{ value: 'light', label: 'Light World' },
		{ value: 'dark', label: 'Dark World' }
	];

	function renderMaps() {
		mapStarted = true;
		session!
			.loadWorldMaps(rom.bytes!)
			.then((urls) => {
				if (destroyed) {
					for (const url of urls) URL.revokeObjectURL(url);
					return;
				}
				worldMaps = urls;
				// Trace the shown route again, now with entrance points.
				if (route) route.steps = session!.traceRoute(route.target);
			})
			.catch((e) => (mapError = e instanceof Error ? e.message : String(e)));
	}

	/** Render the map from a saved ROM, without asking for one. */
	async function startMaps() {
		await rom.loadStored();
		if (rom.bytes && !mapStarted) renderMaps();
	}

	/** Ask for a ROM if none is saved, then render the map. */
	async function loadRomForMaps() {
		if ((await rom.require()) && !mapStarted) renderMaps();
	}

	/** Show a route in the dialog, replacing any route already shown. */
	function showRoute(target: RouteTarget, name: string) {
		if (!session || !routeDialog) return;
		const tile = target.kind === 'location' ? (tiles[target.index] ?? null) : null;
		route = {
			target,
			tile,
			title: `Route to ${tile ? tile.item : name}`,
			subtitle: tile ? tile.location : TARGET_KINDS[target.kind],
			steps: session.traceRoute(target)
		};
		if (routeDialog.open) routeDialog.scrollTop = 0;
		else routeDialog.showModal();
		if (!mapStarted) void startMaps();
	}

	interface MapLine {
		world: MapPoint['world'];
		/** Whether the line is a warp or interior travel rather than overworld travel. */
		dotted: boolean;
		/** SVG polyline points. */
		points: string;
	}

	/** The shown route as lines between its map points, split where the world or kind of travel changes. */
	const mapRoute = $derived.by(() => {
		const lines: MapLine[] = [];
		let start: MapPoint | null = null;
		let end: MapPoint | null = null;
		let indirect = false;
		for (const step of route?.steps ?? []) {
			if (step.indirect) indirect = true;
			for (const point of step.points) {
				// A repeated point, such as leaving through the door just entered, only changes the current world.
				if (end && (end.x !== point.x || end.y !== point.y)) {
					// A line takes the world of its starting point.
					const last = lines.at(-1);
					if (last && last.world === end.world && last.dotted === indirect) {
						last.points += ` ${point.x},${point.y}`;
					} else {
						lines.push({
							world: end.world,
							dotted: indirect,
							points: `${end.x},${end.y} ${point.x},${point.y}`
						});
					}
				} else if (!end) {
					start = point;
				}
				end = point;
				indirect = false;
			}
		}
		// A five-pointed star marking the end, as SVG polygon points.
		let star = '';
		if (end) {
			for (let corner = 0; corner < 10; corner++) {
				const radius = corner % 2 === 0 ? 4 : 1.7;
				const angle = (corner * Math.PI) / 5 - Math.PI / 2;
				star += `${end.x + radius * Math.cos(angle)},${end.y + radius * Math.sin(angle)} `;
			}
		}
		return { lines, start, end, star };
	});

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

	/** Unicode fraction characters, by the number of eighths. */
	const EIGHTHS = ['', '⅛', '¼', '⅜', '½', '⅝', '¾', '⅞'];

	/** Format a health amount in eighths of a heart as a mixed number of hearts, e.g. 10 as "1¼". */
	function formatHearts(eighths: number): string {
		const whole = Math.floor(eighths / 8);
		const fraction = EIGHTHS[eighths % 8];
		return whole === 0 && fraction ? fraction : `${whole}${fraction}`;
	}

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
						? formatHearts(state.health_used ?? 0)
						: `${formatHearts(state.health_remaining)} left`
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

	/** Tiles for every placed item, by location index, including items hidden from the rows. */
	const tiles = $derived.by(() => {
		const result: Record<number, Tile> = {};
		if (!spoiler) return result;
		// Progressive levels count copies in sphere order, then catalog order within a sphere.
		const levels: Record<string, number> = { ...startingCounts };
		for (const group of [...spoiler.spheres, spoiler.unreachable]) {
			for (const entry of group) {
				if (entry.item === null || entry.item === 'nothing') continue;
				const level = (levels[entry.item] ?? 0) + 1;
				levels[entry.item] = level;
				result[entry.location_idx] = {
					icon: getItemLevelIcon(entry.item, level),
					badge: getBadge(entry.item),
					item: describe(entry.item, entry),
					location: entry.location,
					locationIdx: entry.location_idx
				};
			}
		}
		return result;
	});

	const rows = $derived.by(() => {
		if (!spoiler) return [];
		const result: Row[] = [];
		const groups = [...spoiler.spheres];
		if (spoiler.unreachable.length) groups.push(spoiler.unreachable);
		for (const [index, group] of groups.entries()) {
			const ranked: { rank: number; tile: Tile }[] = [];
			for (const entry of group) {
				const tile = tiles[entry.location_idx];
				if (!tile || entry.item === null) continue;
				const tier = items.find((item) => item.name === entry.item)?.tier;
				if (importantOnly && tier !== 'major' && tier !== 'high') continue;
				ranked.push({ rank: getRank(entry.item), tile });
			}
			ranked.sort((a, b) => a.rank - b.rank);
			const rowTiles: Tile[] = [];
			for (const { tile } of ranked) rowTiles.push(tile);
			if (importantOnly && rowTiles.length === 0) continue;
			result.push({
				index,
				title: index < spoiler.spheres.length ? `${index + 1}` : 'Unreachable',
				tiles: rowTiles
			});
		}
		return result;
	});

	function describe(item: string, entry: SpoilerEntry): string {
		let label = ITEM_LABELS[item] ?? item;
		if (entry.door) label += ` (${entry.door})`;
		if (entry.dungeon) label += ` (${entry.dungeon})`;
		return label;
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
		a row to reveal it and the rows above it, and click an item to show a route to it.
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
								title={`${tile.item}\n${tile.location}`}
								aria-label="Show a route to {tile.item} at {tile.location}"
								onclick={(event) => {
									event.stopPropagation();
									showRoute({ kind: 'location', index: tile.locationIdx }, tile.item);
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
				{#if route.tile}
					<span class="tile">
						{#if route.tile.icon}
							<img src={route.tile.icon} alt="" />
						{:else}
							<span class="badge-text" style:color={route.tile.badge.color}
								>{route.tile.badge.text}</span
							>
						{/if}
					</span>
				{/if}
				<div class="me-auto">
					<h2 id="spoiler-route-title" class="h5 fw-bold mb-0">{route.title}</h2>
					<div class="small text-body-secondary">{route.subtitle}</div>
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
								<th scope="col">Location</th>
								<th scope="col">Strat</th>
								<th scope="col">State</th>
							</tr>
						</thead>
						<tbody>
							{#each route.steps as step, stepIndex (stepIndex)}
								<tr>
									<td>{step.location}</td>
									<td>
										{step.strat}
										{#each step.dependencies as dependency (dependency.name)}
											<div>
												{#if dependency.target}
													{@const target = dependency.target}
													<button
														type="button"
														class="btn btn-link btn-sm p-0 text-start dependency"
														onclick={() => showRoute(target, dependency.name)}
														>{dependency.name}</button
													>
												{:else}
													<span class="text-body-secondary">{dependency.name}</span>
												{/if}
											</div>
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
			<div class="px-4 pb-4">
				{#if mapError}
					<div class="alert alert-danger small py-2 px-3 mb-0" role="alert">
						Couldn't render the map: {mapError}
					</div>
				{:else if worldMaps}
					<div class="world-map">
						<img
							src={worldMaps[mapWorld === 'light' ? 0 : 1]}
							alt="{mapWorld === 'light' ? 'Light' : 'Dark'} World map"
						/>
						<!-- Travel in the shown world is solid, in the other world dashed and dimmed; warps and interiors
						     are dotted. Every line animates in the direction of travel. -->
						<svg viewBox="0 0 384 384" aria-hidden="true">
							{#each mapRoute.lines as line, lineIndex (lineIndex)}
								{@const shown = line.world === mapWorld}
								<polyline
									points={line.points}
									class="route-line"
									class:dotted={line.dotted}
									class:dashed={!line.dotted && !shown}
									class:other-world={!shown}
								/>
								{#if !line.dotted && shown}
									<polyline points={line.points} class="route-line flow" />
								{/if}
							{/each}
							{#if mapRoute.start}
								<image
									href={linkHead}
									x={mapRoute.start.x - 6}
									y={mapRoute.start.y - 6}
									width="12"
									height="12"
									class:other-world={mapRoute.start.world !== mapWorld}
								/>
							{/if}
							{#if mapRoute.end}
								<polygon
									points={mapRoute.star}
									class="route-end"
									class:other-world={mapRoute.end.world !== mapWorld}
								/>
							{/if}
						</svg>
					</div>
					<div class="mt-2 text-center">
						<Segmented
							options={WORLD_OPTIONS}
							value={mapWorld}
							label="World"
							onchange={(value) => (mapWorld = value)}
						/>
					</div>
				{:else if mapStarted}
					<p class="text-body-secondary mb-0" role="status">
						<span class="spinner-border spinner-border-sm me-1" aria-hidden="true"></span>
						Rendering the map…
					</p>
				{:else}
					<button type="button" class="btn btn-sm btn-outline-secondary" onclick={loadRomForMaps}>
						Load ROM to show the map
					</button>
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

	.dependency {
		font-size: inherit;
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

	/* Twice the PNG's 384px size, so pixels scale evenly. */
	.world-map {
		position: relative;
		width: 768px;
		max-width: 100%;
		margin: 0 auto;
	}

	.world-map img {
		display: block;
		width: 100%;
		image-rendering: pixelated;
	}

	.world-map svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		filter: drop-shadow(0 0 1px rgb(0 0 0 / 0.9));
	}

	.world-map image {
		image-rendering: pixelated;
	}

	/* Lengths are in map pixels, each two screen pixels at full size. */
	.route-line {
		fill: none;
		stroke: #ff3366;
		stroke-width: 1.5;
		stroke-linejoin: round;
	}

	.route-line.flow {
		stroke: rgb(255 255 255 / 0.85);
		stroke-width: 0.75;
		stroke-dasharray: 2 8;
		animation: march-flow 1s linear infinite;
	}

	.route-line.dashed {
		stroke-width: 1;
		stroke-dasharray: 4 2.5;
		animation: march-dashed 0.65s linear infinite;
	}

	.route-line.dotted {
		stroke-linecap: round;
		stroke-dasharray: 0 3;
		animation: march-dotted 0.3s linear infinite;
	}

	.route-end {
		fill: #fff;
		stroke: #333;
		stroke-width: 0.4;
		stroke-linejoin: round;
	}

	.other-world {
		opacity: 0.5;
	}

	/* A decreasing offset moves the pattern forward along the line. */
	@keyframes march-flow {
		to {
			stroke-dashoffset: -10;
		}
	}

	@keyframes march-dashed {
		to {
			stroke-dashoffset: -6.5;
		}
	}

	@keyframes march-dotted {
		to {
			stroke-dashoffset: -3;
		}
	}

	.badge-text {
		font-size: 0.7rem;
		font-weight: 700;
	}
</style>
