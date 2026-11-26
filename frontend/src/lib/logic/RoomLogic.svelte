<script lang="ts">
	import { onMount } from 'svelte';
	import { findDiagrams, selectDefaultDiagram } from './diagrams';
	import LogicNotes from './LogicNotes.svelte';
	import StratEntry from './StratEntry.svelte';
	import type { LogicRoom } from './types';

	/** A room's header, diagram, and strats; `room.strats` holds the strats to list. */
	let { room }: { room: LogicRoom } = $props();

	const SHOW_DEV_NOTES_KEY = 'logic-show-dev-notes';

	const diagrams = $derived(findDiagrams(room.path, room.name));
	let selectedLabel = $state<string>();
	const diagram = $derived(
		diagrams.find((diagram) => diagram.label === selectedLabel) ?? selectDefaultDiagram(diagrams)
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

	<h2 class="h5 mt-4">Strats ({room.strats.length})</h2>
	{#each room.strats as strat (strat.id)}
		<StratEntry {room} {strat} {showDevNotes} />
	{/each}
</main>

<style>
	.diagram {
		max-width: 100%;
		height: auto;
	}
</style>
