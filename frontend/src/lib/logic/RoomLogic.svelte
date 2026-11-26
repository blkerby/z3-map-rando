<script lang="ts">
	import { onMount } from 'svelte';
	import { findDiagrams, selectDefaultDiagram } from './diagrams';
	import LogicNotes from './LogicNotes.svelte';
	import StratEntry from './StratEntry.svelte';
	import type { LogicRoom } from './types';

	/**
	 * A room's header, diagram, and strats; `room.strats` holds the strats to list. `showNodeFilter` adds
	 * controls to list only strats from or to a chosen node.
	 */
	let { room, showNodeFilter = false }: { room: LogicRoom; showNodeFilter?: boolean } = $props();

	const SHOW_DEV_NOTES_KEY = 'logic-show-dev-notes';

	const diagrams = $derived(findDiagrams(room.path, room.name));
	let selectedLabel = $state<string>();
	const diagram = $derived(
		diagrams.find((diagram) => diagram.label === selectedLabel) ?? selectDefaultDiagram(diagrams)
	);

	let fromNode = $state<number | null>(null);
	let toNode = $state<number | null>(null);
	const strats = $derived(
		room.strats.filter(
			(strat) =>
				(fromNode === null || strat.from.id === fromNode) &&
				(toNode === null || strat.to.id === toNode)
		)
	);

	let showDevNotes = $state(false);

	onMount(() => {
		try {
			showDevNotes = localStorage.getItem(SHOW_DEV_NOTES_KEY) === 'true';
		} catch {
			// Storage unavailable; keep dev notes hidden.
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

	{#if showNodeFilter}
		<div class="node-filter mt-3">
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
		</div>
	{/if}

	<h2 class="h5 mt-4">Strats ({strats.length})</h2>
	{#each strats as strat (strat.id)}
		<StratEntry {room} {strat} {showDevNotes} />
	{/each}
</main>

<style>
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
		grid-row: 1 / span 2;
	}
</style>
