<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { checkRoomStrats } from './difficulty';
	import DifficultyAlerts from './DifficultyAlerts.svelte';
	import { DifficultyFilter } from './difficultyFilter.svelte';
	import LogicNotes from './LogicNotes.svelte';
	import StratEntry from './StratEntry.svelte';
	import type { LogicNotable } from './types';

	let { notable }: { notable: LogicNotable } = $props();

	const SHOW_DEV_NOTES_KEY = 'logic-show-dev-notes';

	const difficultyFilter = new DifficultyFilter();
	/** Passing strat IDs for the chosen difficulty, once checked. */
	let passingStrats = $state<Set<number> | null>(null);

	$effect(() => {
		const option = difficultyFilter.selected;
		const roomId = notable.room.id;
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
		notable.strats.filter((strat) => passingStrats === null || passingStrats.has(strat.id))
	);

	let showDevNotes = $state(false);

	onMount(() => {
		try {
			showDevNotes = localStorage.getItem(SHOW_DEV_NOTES_KEY) === 'true';
		} catch {
			// Storage unavailable; keep dev notes hidden.
		}
		difficultyFilter.load();
	});

	function toggleDevNotes() {
		try {
			localStorage.setItem(SHOW_DEV_NOTES_KEY, String(showDevNotes));
		} catch {
			// Storage unavailable; the choice lasts for this page only.
		}
	}
</script>

<main class="container content-width my-4">
	<div class="d-flex flex-wrap align-items-start justify-content-between gap-2">
		<div>
			<h1 class="h3 mb-1">
				{notable.name}
				<span class="text-body-secondary">({notable.difficulty})</span>
			</h1>
			<div class="text-body-secondary">
				Room:
				<a href={resolve('/logic/[roomId]', { roomId: String(notable.room.id) })}>
					{notable.room.name}
				</a>, Notable ID: {notable.id}
			</div>
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
		<LogicNotes notes={notable} {showDevNotes} />
	</div>

	<div class="difficulty-filter mt-3">
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
	</div>
	<DifficultyAlerts filter={difficultyFilter} />

	<h2 class="h5 mt-4">Strats ({strats.length})</h2>
	{#each strats as strat (strat.id)}
		<StratEntry room={notable.room} {strat} {showDevNotes} />
	{/each}
</main>

<style>
	.difficulty-filter {
		display: grid;
		grid-template-columns: max-content minmax(0, 38rem);
		justify-content: start;
		align-items: center;
		gap: 0.75rem 1rem;
	}
</style>
