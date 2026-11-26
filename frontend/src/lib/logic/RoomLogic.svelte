<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { findDiagrams, selectDefaultDiagram } from './diagrams';
	import { checkRoomStrats } from './difficulty';
	import DifficultyAlerts from './DifficultyAlerts.svelte';
	import { DifficultyFilter } from './difficultyFilter.svelte';
	import LogicNotes from './LogicNotes.svelte';
	import StratEntry from './StratEntry.svelte';
	import type { LogicRoom, RoomConnection, RoomConnectionKind } from './types';

	/**
	 * A room's header, diagram, connections, and strats; `room.strats` holds the strats to list.
	 * `showNodeFilter` adds controls to list only strats from or to a chosen node, passing with chosen
	 * skill settings, and, in overworld rooms, usable in a chosen world.
	 */
	let { room, showNodeFilter = false }: { room: LogicRoom; showNodeFilter?: boolean } = $props();

	const connectionGroups: { kind: RoomConnectionKind; label: string }[] = [
		{ kind: 'scroll', label: 'Scrolling' },
		{ kind: 'entrance', label: 'Entrances' },
		{ kind: 'whirlpool', label: 'Whirlpools' },
		{ kind: 'teleport', label: 'Teleports' }
	];

	function targetRoomName(conn: RoomConnection): string {
		if (conn.targetRoom.darkWorldName) {
			if (conn.targetWorld === 'dark') {
				return conn.targetRoom.darkWorldName;
			}
			if (conn.targetWorld === 'both') {
				return `${conn.targetRoom.name} / ${conn.targetRoom.darkWorldName}`;
			}
		}
		return conn.targetRoom.name;
	}

	const SHOW_DEV_NOTES_KEY = 'logic-show-dev-notes';

	const diagrams = $derived(findDiagrams(room.path, room.name));
	let selectedLabel = $state<string>();
	const diagram = $derived(
		diagrams.find((diagram) => diagram.label === selectedLabel) ?? selectDefaultDiagram(diagrams)
	);

	let fromNode = $state<number | null>(null);
	let toNode = $state<number | null>(null);
	let world = $state<'light' | 'dark' | null>(null);
	const hasWorlds = $derived(room.strats.some((strat) => strat.world !== undefined));

	const difficultyFilter = new DifficultyFilter();
	/** IDs of the strats passing with the chosen difficulty, once checked. */
	let passingStrats = $state<Set<number> | null>(null);

	$effect(() => {
		const option = difficultyFilter.selected;
		const roomId = room.id;
		passingStrats = null;
		difficultyFilter.error = null;
		if (!option) return;
		let current = true;
		(async () => {
			const skill = await difficultyFilter.resolveSkill();
			if (!current || skill === null) return;
			const ids = await checkRoomStrats(roomId, skill);
			if (current) passingStrats = ids;
		})().catch((error) => {
			if (current) difficultyFilter.error = error.message;
		});
		return () => {
			current = false;
		};
	});

	const strats = $derived(
		room.strats.filter(
			(strat) =>
				(passingStrats === null || passingStrats.has(strat.id)) &&
				(fromNode === null || strat.from.id === fromNode) &&
				(toNode === null || strat.to.id === toNode) &&
				(world === null ||
					strat.world === world ||
					strat.world === 'any' ||
					strat.fromWorld === world ||
					strat.toWorld === world)
		)
	);

	let showDevNotes = $state(false);

	onMount(() => {
		try {
			showDevNotes = localStorage.getItem(SHOW_DEV_NOTES_KEY) === 'true';
		} catch {
			// Storage unavailable; keep dev notes hidden.
		}
		if (showNodeFilter) {
			difficultyFilter.load();
		}
	});

	function toggleDevNotes() {
		try {
			localStorage.setItem(SHOW_DEV_NOTES_KEY, String(showDevNotes));
		} catch {
			// Storage unavailable; the choice lasts for this page only.
		}
	}
</script>

<main class="container my-4">
	<div class="d-flex flex-wrap align-items-start justify-content-between gap-2">
		<div>
			<h1 class="h3 mb-1">{room.name}</h1>
			{#if room.darkWorldName}
				<div>Dark World: {room.darkWorldName}</div>
			{/if}
			<div class="text-body-secondary">Room ID: {room.id}</div>
		</div>
		<div class="form-check form-switch">
			<input
				class="form-check-input"
				type="checkbox"
				role="switch"
				id="show-dev-notes"
				bind:checked={showDevNotes}
				onchange={toggleDevNotes}
			/>
			<label class="form-check-label" for="show-dev-notes">Show dev notes</label>
		</div>
	</div>

	<div class="mt-3">
		<LogicNotes notes={room} {showDevNotes} />
	</div>

	{#if diagram}
		{#if diagrams.length > 1}
			<div class="btn-group btn-group-sm flex-wrap mb-2" role="group" aria-label="Diagram">
				{#each diagrams as option (option.label)}
					<button
						type="button"
						class="btn btn-outline-secondary"
						class:active={option === diagram}
						aria-pressed={option === diagram}
						onclick={() => (selectedLabel = option.label)}
					>
						{option.label}
					</button>
				{/each}
			</div>
		{/if}
		<img class="diagram d-block mb-3" src={diagram.url} alt="{room.name} {diagram.label} diagram" />
	{/if}

	{#if room.connections.length > 0}
		<h2 class="h5 mt-4">Connections</h2>
		{#each connectionGroups as group (group.kind)}
			{@const items = room.connections.filter((conn) => conn.kind === group.kind)}
			{#if items.length > 0}
				<h3 class="h6 text-body-secondary mt-2 mb-1">{group.label}</h3>
				<ul class="list-unstyled mb-2">
					{#each items as conn, i (i)}
						{@const roomName = targetRoomName(conn)}
						<li>
							<span class="text-body-secondary"
								>Node {conn.nodeId} ({conn.detail}){#if conn.name}:{/if}</span
							>
							{#if conn.name}
								{conn.name}
							{/if}
							<svg
								class="conn-arrow mx-1"
								viewBox="0 0 16 12"
								fill="none"
								stroke="currentColor"
								stroke-width="1.5"
								stroke-linecap="round"
								stroke-linejoin="round"
								aria-hidden="true"
							>
								<line x1="2" y1="6" x2="14" y2="6" />
								{#if conn.direction === 'in' || conn.direction === 'both'}
									<polyline points="5.5 2.5 2 6 5.5 9.5" />
								{/if}
								{#if conn.direction === 'out' || conn.direction === 'both'}
									<polyline points="10.5 2.5 14 6 10.5 9.5" />
								{/if}
							</svg>
							<a href={resolve('/logic/[roomId]', { roomId: String(conn.targetRoom.id) })}>
								{roomName}
							</a>
						</li>
					{/each}
				</ul>
			{/if}
		{/each}
	{/if}

	{#if showNodeFilter}
		<div class="node-filter mt-3">
			<label for="difficulty">Difficulty</label>
			<select
				id="difficulty"
				class="form-select"
				bind:value={difficultyFilter.selectedId}
				onchange={() => difficultyFilter.save()}
			>
				<option value="">Any</option>
				{#each difficultyFilter.options as option (option.id)}
					<option value={option.id}>{option.label}</option>
				{/each}
			</select>
			<label for="from-node">From node</label>
			<select id="from-node" class="form-select" bind:value={fromNode}>
				<option value={null}>Any</option>
				{#each room.nodes as node (node.id)}
					<option value={node.id}>{node.id}: {node.name}</option>
				{/each}
			</select>
			<label for="to-node">To node</label>
			<select id="to-node" class="form-select" bind:value={toNode}>
				<option value={null}>Any</option>
				{#each room.nodes as node (node.id)}
					<option value={node.id}>{node.id}: {node.name}</option>
				{/each}
			</select>
			<button
				type="button"
				class="swap btn btn-secondary"
				aria-label="Swap from and to nodes"
				title="Swap from and to nodes"
				onclick={() => ([fromNode, toNode] = [toNode, fromNode])}
			>
				<i class="bi bi-arrow-down-up"></i>
			</button>
			{#if hasWorlds}
				<label for="world">World</label>
				<select id="world" class="form-select" bind:value={world}>
					<option value={null}>Any</option>
					<option value="light">Light</option>
					<option value="dark">Dark</option>
				</select>
			{/if}
		</div>
		<DifficultyAlerts filter={difficultyFilter} />
	{/if}

	<h2 class="h5 mt-4">Strats ({strats.length})</h2>
	{#each strats as strat (strat.id)}
		<StratEntry {room} {strat} {showDevNotes} />
	{/each}
</main>

<style>
	.conn-arrow {
		width: 1em;
		height: 0.75em;
		vertical-align: -0.05em;
	}

	.diagram {
		max-width: 100%;
		height: auto;
	}

	.node-filter {
		display: grid;
		grid-template-columns: max-content minmax(0, 38rem) auto;
		justify-content: start;
		align-items: center;
		gap: 0.75rem 1rem;
	}

	.node-filter label {
		grid-column: 1;
	}

	.node-filter .swap {
		grid-column: 3;
		grid-row: 2 / span 2;
	}
</style>
