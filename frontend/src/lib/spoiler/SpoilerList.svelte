<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { MediaQuery } from 'svelte/reactivity';
	import { resolve } from '$app/paths';
	import logicDataJson from '$build/logic_data.json';
	import linkHead from '$lib/assets/link_head.png';
	import { ITEM_LABELS, getItemLevelIcon, items } from '$lib/generation/data';
	import { getDifficultyClass } from '$lib/logic/difficulty';
	import { useRomStore } from '$lib/rom/store.svelte';
	import type { Tree } from '$lib/settings/tree';
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
		seedBytes,
		active,
		startingCounts,
		logicKey,
		settings
	}: {
		seed: string;
		seedBytes: Uint8Array;
		/** Whether the list is visible; the replay runs the first time it is. */
		active: boolean;
		/** Starting copies of each item, so progressive items show the level each pickup reaches. */
		startingCounts: Record<string, number>;
		/** Changes whenever logic-affecting settings change. */
		logicKey: string;
		/** Override generation settings, or null to use the seed's recorded settings. */
		settings: Tree | null;
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

	/** Strat difficulties by room ID and strat ID, prepared by the catalog builder. */
	const logicData = logicDataJson as Record<string, Record<string, string>>;

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
	let routeList = $state<HTMLDivElement>();
	/** Number of rows shown, from the top; the rest show question marks until revealed. */
	let revealed = $state(0);
	/** Whether to show only major- and high-tier items, hiding rows left empty. Remembered across sessions. */
	let importantOnly = $state(true);
	const IMPORTANT_ONLY_KEY = 'spoilerImportantOnly';

	onMount(() => {
		importantOnly = localStorage.getItem(IMPORTANT_ONLY_KEY) !== 'false';
	});

	let appliedLogicKey = '';
	let wasActive = false;

	function replayCurrent() {
		if (!session) return;
		try {
			spoiler = session.replay(settings);
			error = '';
			appliedLogicKey = logicKey;
			if (revealed !== Infinity) revealed = 0;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	$effect(() => {
		if (!active || started) return;
		started = true;
		const initialKey = logicKey;
		const initialSettings = settings;
		openSpoiler(seed, seedBytes, initialSettings)
			.then((result) => {
				if (destroyed) {
					result.free();
					return;
				}
				session = result;
				appliedLogicKey = initialKey;
				spoiler = result.spoiler;
				if (logicKey !== appliedLogicKey) replayCurrent();
			})
			.catch((e) => (error = e instanceof Error ? e.message : String(e)));
	});

	$effect(() => {
		const isOpen = active;
		const currentKey = logicKey;
		const openedNow = isOpen && !wasActive;
		wasActive = isOpen;
		if (!isOpen || !session || currentKey === appliedLogicKey) return;
		if (openedNow) {
			replayCurrent();
			return;
		}
		const timer = setTimeout(() => {
			if (!destroyed && active && session && logicKey !== appliedLogicKey) {
				replayCurrent();
			}
		}, 150);
		return () => clearTimeout(timer);
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
	/**
	 * The route row whose progress the map shows: the last hovered or focused, which stays active so the mouse can move
	 * to the map, or the row passing a point clicked on the map. A heading's index is its group's first step. The
	 * progress ends at the clicked point, or else where the row's progress ends (see `findProgressEnd` and
	 * `findEntryEnd`).
	 */
	let activeRow = $state<{ index: number; heading: boolean; point: number | null } | null>(null);
	/** The index in `routePoints` of the point where the active progress ends. */
	const activeEnd = $derived(
		activeRow
			? (activeRow.point ??
					(activeRow.heading ? findEntryEnd(activeRow.index) : findProgressEnd(activeRow.index)))
			: null
	);
	/** The active step's world, or with none, the route target's. */
	const mapWorld = $derived(
		route?.steps?.[activeRow?.index ?? route.steps.length - 1]?.dark_world ? 'dark' : 'light'
	);

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
		activeRow = null;
		hoveredPoint = null;
		route = {
			target,
			tile,
			title: `Route to ${tile ? tile.item : name}`,
			subtitle: tile ? tile.location : TARGET_KINDS[target.kind],
			steps: session.traceRoute(target)
		};
		if (routeDialog.open) routeList?.scrollTo(0, 0);
		else routeDialog.showModal();
		if (!mapStarted) void startMaps();
	}

	/** Expand or collapse every group of the shown route. */
	function setGroupsOpen(open: boolean) {
		for (const group of routeList!.querySelectorAll('details')) group.open = open;
	}

	interface RouteGroup {
		/** The connection and room entered, such as "Cross West to Swamp Ruins". */
		title: string;
		/** Steps with their positions in the route, so each state is compared with the previous step's. */
		steps: { step: RouteRow; index: number }[];
	}

	/** The shown route's steps, grouped into runs within one room and world. */
	const routeGroups = $derived.by(() => {
		const groups: RouteGroup[] = [];
		for (const [index, step] of (route?.steps ?? []).entries()) {
			if (step.entered !== null) {
				groups.push({ title: step.entered, steps: [] });
			}
			groups.at(-1)?.steps.push({ step, index });
		}
		return groups;
	});

	interface RoutePoint {
		point: MapPoint;
		/** The step the point comes from. */
		step: number;
		/** Whether the travel to the point from the previous one is a warp or interior travel. */
		indirect: boolean;
	}

	/** The shown route's map points, in order. */
	const routePoints = $derived.by(() => {
		const points: RoutePoint[] = [];
		let indirect = false;
		for (const [index, step] of (route?.steps ?? []).entries()) {
			if (step.indirect) indirect = true;
			for (const point of step.points) {
				points.push({ point, step: index, indirect });
				indirect = false;
			}
		}
		return points;
	});

	/**
	 * The index of the point where the progress through a step ends: the first point after the step, or the route's
	 * last. So a step entering a room shows the travel through the room, up to the point where the route leaves it.
	 */
	function findProgressEnd(stepIndex: number): number {
		for (const [index, { step }] of routePoints.entries()) {
			if (step > stepIndex) return index;
		}
		return routePoints.length - 1;
	}

	/**
	 * The index of the point where the progress shown by a group's heading ends: where the route has just entered the
	 * group's room, at the last point of its first step, or with none, where the previous row's progress ends. Going
	 * down the rows, a crossing's near point thus ends the previous group, its far point the heading, and the point where
	 * the route leaves the room the crossing's own row.
	 */
	function findEntryEnd(stepIndex: number): number {
		let end = findProgressEnd(stepIndex - 1);
		for (const [index, { step }] of routePoints.entries()) {
			if (step === stepIndex) end = index;
		}
		return end;
	}

	interface MapLine {
		world: MapPoint['world'];
		/** Whether the line is a warp or interior travel rather than overworld travel. */
		dotted: boolean;
		/** Whether the line ends at or before the active progress's end point. */
		progress: boolean;
		/**
		 * The length before the line of its run of travel in one world and of one kind, in map pixels, which offsets its
		 * dash pattern. Each run starts its pattern afresh, and a line split from a run where the progress ends continues
		 * it, so the patterns stay aligned wherever the progress ends.
		 */
		offset: number;
		/** SVG polyline points. */
		points: string;
	}

	/**
	 * The shown route as lines between its map points, split where the world, the kind of travel, or the progress
	 * changes.
	 */
	const mapRoute = $derived.by(() => {
		const lines: MapLine[] = [];
		let start: MapPoint | null = null;
		let end: MapPoint | null = null;
		let runLength = 0;
		for (const [index, { point, indirect }] of routePoints.entries()) {
			const progress = activeEnd !== null && index <= activeEnd;
			// A repeated point, such as leaving through the door just entered, only changes the current world.
			if (end && (end.x !== point.x || end.y !== point.y)) {
				// A line takes the world of its starting point.
				const last = lines.at(-1);
				const sameRun = last && last.world === end.world && last.dotted === indirect;
				if (sameRun && last.progress === progress) {
					last.points += ` ${point.x},${point.y}`;
				} else {
					if (!sameRun) runLength = 0;
					lines.push({
						world: end.world,
						dotted: indirect,
						progress,
						offset: runLength,
						points: `${end.x},${end.y} ${point.x},${point.y}`
					});
				}
				runLength += Math.hypot(point.x - end.x, point.y - end.y);
			} else if (!end) {
				start = point;
			}
			end = point;
		}
		// A five-pointed star marking the end, as SVG polygon points.
		let star = '';
		if (end) {
			for (let corner = 0; corner < 10; corner++) {
				const radius = corner % 2 === 0 ? 4.75 : 2;
				const angle = (corner * Math.PI) / 5 - Math.PI / 2;
				star += `${end.x + radius * Math.cos(angle)},${end.y + radius * Math.sin(angle)} `;
			}
		}
		return { lines, start, end, star };
	});

	/** The progress end's dot pulses slower with a reduced-motion preference, which its SVG animation can't query itself. */
	const reducedMotion = new MediaQuery('prefers-reduced-motion: reduce');

	/** The index in `routePoints` of the point the mouse is over on the map. */
	let hoveredPoint = $state<number | null>(null);

	/** How far from a point, in map pixels, the mouse can be to hover or click it. */
	const POINT_REACH = 20;

	/**
	 * Hover the route point nearest the mouse within reach, preferring the shown world's where points coincide, as at a
	 * crossing between worlds.
	 */
	function hoverNearestPoint(event: MouseEvent) {
		const rect = (event.currentTarget as SVGSVGElement).getBoundingClientRect();
		const x = ((event.clientX - rect.left) * 320) / rect.width;
		const y = ((event.clientY - rect.top) * 320) / rect.height;
		let nearest: number | null = null;
		let nearestDistance = POINT_REACH;
		for (const [index, { point }] of routePoints.entries()) {
			const distance = Math.hypot(point.x - x, point.y - y);
			if (
				distance < nearestDistance ||
				(distance === nearestDistance && nearest !== null && point.world === mapWorld)
			) {
				nearest = index;
				nearestDistance = distance;
			}
		}
		hoveredPoint = nearest;
	}

	/**
	 * End the progress at a point clicked on the map, making active the row reaching it: the last whose own progress
	 * ends at the point, or with none, the first whose progress passes it. So a crossing's near point makes active the
	 * last step before the crossing, and its far point the crossing. Its group opens and the route scrolls to it.
	 */
	function selectPoint(pointIndex: number) {
		let index = -1;
		for (let step = 0; step < route!.steps!.length; step++) {
			const end = findProgressEnd(step);
			if (end === pointIndex) {
				index = step;
			} else if (end > pointIndex) {
				if (index < 0) index = step;
				break;
			}
		}
		activeRow = { index, heading: false, point: pointIndex };
		const row = routeList!.querySelector(`[data-step="${index}"]`)!;
		const group = row.closest('details')!;
		group.open = true;
		// Scroll as little as possible to show the most of these that fits below the toolbar: the group's heading through
		// the next group's heading, so the whole group is plainly in view; else the whole group; else its heading through
		// the row; else the row.
		const top =
			routeList!.getBoundingClientRect().top +
			routeList!.querySelector('.route-toolbar')!.clientHeight;
		const bottom = routeList!.getBoundingClientRect().bottom;
		const headingTop = group.querySelector('summary')!.getBoundingClientRect().top;
		const rowRect = row.getBoundingClientRect();
		const ranges = [
			[headingTop, group.getBoundingClientRect().bottom],
			[headingTop, rowRect.bottom],
			[rowRect.top, rowRect.bottom]
		];
		const nextHeading = group.nextElementSibling?.querySelector('summary');
		if (nextHeading) ranges.unshift([headingTop, nextHeading.getBoundingClientRect().bottom]);
		for (const [rangeTop, rangeBottom] of ranges) {
			if (rangeBottom - rangeTop > bottom - top) continue;
			if (rangeTop < top) routeList!.scrollBy(0, rangeTop - top);
			else if (rangeBottom > bottom) routeList!.scrollBy(0, rangeBottom - bottom);
			break;
		}
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

<!-- A step's line: the strat and the node reached. Strat names are tinted by difficulty; the connecting words and
     synthesized names (of steps without a strat page) are muted. -->
{#snippet stepText(step: RouteRow)}
	{#if !step.strat}
		<span class="connector">Start at</span> {step.node}
	{:else if step.stays}
		{@render stratName(step, step.strat)}
	{:else if step.strat === 'Base' || step.strat.startsWith('Base ')}
		<span class="connector">Go to</span> {step.node}
	{:else}
		{@render stratName(step, step.strat)} <span class="connector">to</span>
		{step.node}
	{/if}
{/snippet}

{#snippet stratName(step: RouteRow, strat: string)}
	{#if step.strat_link}
		{@const difficulty = logicData[step.strat_link[0]][step.strat_link[1]]}
		<span class="strat-name {getDifficultyClass(difficulty)}">{strat}</span>
	{:else}
		<span class="connector">{strat}</span>
	{/if}
{/snippet}

<!-- Route lines on the map: those in the other world first, so the shown world's lines are drawn on top. The route's
     progress through the active row is green. -->
{#snippet routeLines(lines: MapLine[])}
	{#each [false, true] as shown (shown)}
		{#each lines as line, lineIndex (lineIndex)}
			{#if (line.world === mapWorld) === shown}
				<!-- A black casing under the line, so it stands out from the map. -->
				<polyline
					points={line.points}
					class="route-line casing"
					class:dotted={line.dotted}
					class:dashed={!line.dotted}
					style:--offset="{line.offset}px"
				/>
				<polyline
					points={line.points}
					class="route-line"
					class:dotted={line.dotted}
					class:dashed={!line.dotted}
					class:progress={line.progress}
					class:other-world={!shown}
					style:--offset="{line.offset}px"
				/>
			{/if}
		{/each}
	{/each}
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
			<div class="route-body">
				<div class="route-list" bind:this={routeList}>
					{#if route.steps}
						<div class="route-toolbar">
							<button
								type="button"
								class="btn btn-sm btn-outline-secondary"
								onclick={() => setGroupsOpen(true)}
							>
								Expand all
							</button>
							<button
								type="button"
								class="btn btn-sm btn-outline-secondary"
								onclick={() => setGroupsOpen(false)}
							>
								Collapse all
							</button>
						</div>
						<!-- Recreated for each route shown, so its groups start collapsed. -->
						{#key route.target}
							{#each routeGroups as group, groupIndex (groupIndex)}
								{@const firstIndex = group.steps[0].index}
								<details>
									<summary
										class="route-group-title"
										class:active={activeRow?.heading && activeRow.index === firstIndex}
										onmouseenter={() =>
											(activeRow = { index: firstIndex, heading: true, point: null })}
										onfocus={() => (activeRow = { index: firstIndex, heading: true, point: null })}
									>
										{group.title}
									</summary>
									{#each group.steps as { step, index } (index)}
										{@const stateParts = describeState(step.state, route.steps[index - 1]?.state)}
										<!-- Hovering only previews what focusing the row's link also shows. -->
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div
											class="route-step"
											data-step={index}
											class:active={activeRow && !activeRow.heading && activeRow.index === index}
											onmouseenter={() => (activeRow = { index, heading: false, point: null })}
											onfocusin={() => (activeRow = { index, heading: false, point: null })}
										>
											{#if step.strat_link}
												<a
													class="step-line"
													href={resolve('/logic/[roomId]/[stratId]', {
														roomId: String(step.strat_link[0]),
														stratId: String(step.strat_link[1])
													})}
													target="_blank"
													rel="noopener">{@render stepText(step)}</a
												>
											{:else if step.strat}
												<!-- A synthesized step links to the logic page of the room reached. -->
												<a
													class="step-line"
													href={resolve('/logic/[roomId]', { roomId: String(step.room_id) })}
													target="_blank"
													rel="noopener">{@render stepText(step)}</a
												>
											{:else}
												<div class="step-line">{@render stepText(step)}</div>
											{/if}
											<div class="route-detail small text-body-secondary">
												{#each step.dependencies as dependency (dependency.name)}
													<div>
														{#if dependency.target}
															{@const target = dependency.target}
															<button
																type="button"
																class="btn btn-link btn-sm p-0 text-start dependency"
																onclick={() => showRoute(target, dependency.name)}
																><i class="bi bi-flag me-1" aria-hidden="true"
																></i>{dependency.name}</button
															>
														{:else}
															{dependency.name}
														{/if}
													</div>
												{/each}
												{#if stateParts.length}
													<div>
														{#each stateParts as part, partIndex (partIndex)}
															<span class="state-part">
																{#if part.icon}<img
																		src={part.icon}
																		alt={part.label}
																		title={part.label}
																	/>{/if}{part.text}
															</span>
														{/each}
													</div>
												{/if}
											</div>
										</div>
									{/each}
								</details>
							{/each}
						{/key}
					{:else}
						<p class="text-body-secondary mb-0">No route found.</p>
					{/if}
				</div>
				<div class="route-map">
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
							<!-- Overworld travel is dashed, and warps and interiors dotted; travel in the other world is dimmed.
							     Every line animates in the direction of travel. -->
							<!-- The point nearest the mouse within reach can be clicked to end the progress there. The route list
							     offers the same selection from the keyboard. -->
							<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
							<svg
								viewBox="0 0 320 320"
								aria-hidden="true"
								class:pointing={hoveredPoint !== null}
								onmousemove={hoverNearestPoint}
								onmouseleave={() => (hoveredPoint = null)}
								onclick={() => {
									if (hoveredPoint !== null) selectPoint(hoveredPoint);
								}}
							>
								{@render routeLines(mapRoute.lines)}
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
									<!-- Green once the progress reaches it, in place of the progress end's dot. -->
									<polygon
										points={mapRoute.star}
										class="route-end"
										class:reached={activeEnd === routePoints.length - 1}
										class:other-world={mapRoute.end.world !== mapWorld}
									/>
								{/if}
								<!-- Dots where the active progress ends, in green, and at the point the mouse is over, in white; none
								     at the first and last points, which Link's head and the star mark. -->
								{#each [activeEnd, hoveredPoint] as dotIndex, dotPosition (dotPosition)}
									{#if dotIndex !== null && dotIndex > 0 && dotIndex < routePoints.length - 1}
										{@const dot = routePoints[dotIndex].point}
										<circle
											cx={dot.x}
											cy={dot.y}
											r="2.5"
											class="route-dot"
											class:hovered={dotPosition === 1}
											class:other-world={dot.world !== mapWorld}
										>
											{#if dotPosition === 0}
												<!-- A gentle pulse of the radius, which keeps the dot centered on its point where a
												     scale transform would wobble as it is drawn to pixels. -->
												<animate
													attributeName="r"
													values="2.5;2.875;2.5"
													dur={reducedMotion.current ? '1.6s' : '0.8s'}
													calcMode="spline"
													keySplines="0.42 0 0.58 1; 0.42 0 0.58 1"
													repeatCount="indefinite"
												/>
											{/if}
										</circle>
									{/if}
								{/each}
							</svg>
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
		background: none;
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
		width: min(80rem, calc(100vw - 4rem));
		height: calc(100dvh - 2rem);
		max-height: none;
		padding: 0;
		border: 1px solid var(--bs-border-color);
		border-radius: 0.6rem;
		background: var(--bs-body-bg);
		color: var(--bs-body-color);
	}

	.route-dialog::backdrop {
		background: rgb(0 0 0 / 0.55);
	}

	.route-dialog[open] {
		display: flex;
		flex-direction: column;
	}

	/* The route scrolls beside the map, which stays in place. Sizes are relative to this container. */
	.route-body {
		flex: 1;
		min-height: 0;
		display: flex;
		container-type: size;
	}

	.route-list {
		flex: 1;
		min-width: 0;
		overflow-y: auto;
		padding: 1rem 1.5rem;
	}

	/* Stays at the top of the route as it scrolls, covering the rows beneath. Sticky offsets are measured inside the
	   list's padding, which the bar covers too. */
	.route-toolbar {
		position: sticky;
		top: -1rem;
		z-index: 1;
		display: flex;
		gap: 0.5rem;
		margin-top: -1rem;
		padding: 1rem 0 0.5rem;
		background: var(--bs-body-bg);
	}

	/* The map fills the height, while leaving the route at least 20rem. */
	.route-map {
		width: min(100cqh - 2rem, 100cqw - 20rem);
		box-sizing: content-box;
		padding: 1rem 1.5rem 1rem 0;
	}

	/* On narrow screens, the map sits on top and the route scrolls below it. */
	@media (max-width: 767.98px) {
		.route-body {
			flex-direction: column;
		}

		.route-map {
			order: -1;
			align-self: center;
			width: min(40cqh, 100cqw - 2rem);
			padding: 0.75rem 1rem 0;
		}
	}

	.route-group-title,
	.route-step {
		border-radius: 0.3rem;
	}

	/* The padding is on the line, so the whole highlighted line is the link. */
	.route-group-title,
	.step-line {
		padding: 0.25rem 0.5rem;
	}

	.route-group-title:hover,
	.route-step:hover,
	.route-group-title.active,
	.route-step.active {
		background: var(--bs-secondary-bg);
	}

	.route-group-title {
		font-weight: 600;
	}

	.route-step {
		margin-left: 1rem;
		overflow-wrap: anywhere;
	}

	.step-line {
		display: block;
		color: inherit;
		text-decoration: none;
	}

	a.step-line:hover {
		text-decoration: underline;
		text-decoration-color: var(--bs-secondary-color);
	}

	.connector {
		color: var(--bs-secondary-color);
	}

	.route-detail {
		padding: 0 0.5rem 0 1.5rem;
	}

	.route-detail > :last-child {
		padding-bottom: 0.25rem;
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

	.hidden-tile {
		color: var(--bs-secondary-color);
	}

	.row-button:hover {
		background: var(--bs-secondary-bg);
	}

	.world-map {
		position: relative;
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
		animation: march 3.9s linear infinite;
	}

	.world-map image {
		image-rendering: pixelated;
	}

	/* Lengths are in map pixels. */
	.route-line {
		stroke-dashoffset: calc(var(--offset) - var(--march));
		fill: none;
		stroke: #fff;
		stroke-width: 1.75;
		stroke-linejoin: round;
	}

	.route-line.dashed {
		stroke-width: 1.5;
		stroke-dasharray: 4 2.5;
	}

	.route-line.dotted {
		stroke-linecap: round;
		stroke-dasharray: 0 3;
	}

	/* After the dash styles, so its widths override theirs: a map pixel and a quarter wider than a dashed line, and for a
	   dotted line, just short of the 3-pixel spacing of the dots, so their casings stay apart. */
	.route-line.casing {
		stroke: #000;
		stroke-width: 2.875;
	}

	/* Each casing dash also reaches 0.625 past both ends of its dash, so the border goes all the way around. */
	.route-line.casing.dashed {
		stroke-width: 2.75;
		stroke-dasharray: 5.25 1.25;
		stroke-dashoffset: calc(var(--offset) - var(--march) + 0.625px);
	}

	.route-line.progress {
		stroke: #00ff00;
	}

	.route-dot {
		fill: #00ff00;
		stroke: #000;
		stroke-width: 0.75;
	}

	.route-dot.hovered {
		fill: #fff;
	}

	.world-map svg.pointing {
		cursor: pointer;
	}

	/* The border is drawn under the fill, so half its width shows outside, matching the lines' borders. */
	.route-end {
		fill: #fff;
		stroke: #000;
		stroke-width: 1.25;
		stroke-linejoin: round;
		paint-order: stroke;
	}

	.route-end.reached {
		fill: #00ff00;
	}

	.other-world {
		opacity: 0.65;
	}

	/*
	 * Every line's pattern is offset by one shared distance, animated on the map, which moves the patterns forward along
	 * their lines at 10 map pixels a second. Being shared keeps the patterns of lines created at different times in
	 * step. It loops at 39 pixels, a whole number of periods of both the dashes (6.5) and the dots (3).
	 */
	@property --march {
		syntax: '<length>';
		inherits: true;
		initial-value: 0px;
	}

	@keyframes march {
		to {
			--march: 39px;
		}
	}

	/* With reduced motion, the patterns move at a quarter speed; they still move, since the motion shows the direction of
	   travel. */
	@media (prefers-reduced-motion: reduce) {
		.world-map svg {
			animation-duration: 15.6s;
		}
	}

	.badge-text {
		font-size: 0.7rem;
		font-weight: 700;
	}
</style>
