<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { checkRoomsStrats } from './difficulty';
	import DifficultyAlerts from './DifficultyAlerts.svelte';
	import { DifficultyFilter } from './difficultyFilter.svelte';
	import LogicNotes from './LogicNotes.svelte';
	import StratEntry from './StratEntry.svelte';
	import type { LogicTech } from './types';

	let { tech }: { tech: LogicTech } = $props();

	const SHOW_DEV_NOTES_KEY = 'logic-show-dev-notes';

	const difficultyFilter = new DifficultyFilter();
	/** Passing strat IDs by room ID for the chosen difficulty, once checked. */
	let passingStrats = $state<Map<number, Set<number>> | null>(null);

	const roomIds = $derived(Array.from(new Set(tech.strats.map((entry) => entry.room.id))));

	$effect(() => {
		const option = difficultyFilter.selected;
		const ids = roomIds;
		passingStrats = null;
		difficultyFilter.error = null;
		if (!option) return;
		let current = true;
		(async () => {
			const skill = await difficultyFilter.resolveSkill();
			if (!current || skill === null) return;
			const passing = await checkRoomsStrats(ids, skill);
			if (current) passingStrats = passing;
		})().catch((error) => {
			if (current) difficultyFilter.error = error.message;
		});
		return () => {
			current = false;
		};
	});

	const strats = $derived(
		tech.strats.filter(
			(entry) =>
				passingStrats === null || (passingStrats.get(entry.room.id)?.has(entry.strat.id) ?? false)
		)
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

<main class="container my-4">
	<div class="d-flex flex-wrap align-items-start justify-content-between gap-2">
		<div>
			<h1 class="h3 mb-1">
				{tech.tier !== undefined ? `${tech.name} ${tech.tier}` : tech.name}
				<span class="text-body-secondary">({tech.difficulty})</span>
			</h1>
			<div class="text-body-secondary">
				Tech ID: {tech.id}{#if tech.tier !== undefined}, Tier: {tech.tier}{/if}
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
		<LogicNotes notes={tech} {showDevNotes} />
	</div>

	{#if tech.techRequires.length > 0}
		<div class="mt-2">
			<span class="text-body-secondary">Requires tech:</span>
			{#each tech.techRequires as req, i (req.id)}
				{#if i > 0},
				{/if}<a href={resolve('/logic/tech/[techId]', { techId: String(req.id) })}>{req.name}</a>
				<span class="text-body-secondary">({req.difficulty})</span>
			{/each}
		</div>
	{/if}

	{#if tech.otherRequires}
		<div class="mt-2">
			<div class="text-body-secondary">Other requires:</div>
			<pre class="mb-0">{tech.otherRequires}</pre>
		</div>
	{/if}

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
	{#each strats as entry (`${entry.room.id}:${entry.strat.id}`)}
		<StratEntry room={entry.room} strat={entry.strat} {showDevNotes} />
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
