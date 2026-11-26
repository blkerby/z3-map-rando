<script lang="ts">
	import { onMount } from 'svelte';
	import { generationModel } from '$lib/generation/data';
	import { SettingsStore } from '$lib/settings/store.svelte';
	import { findDiagrams, selectDefaultDiagram } from './diagrams';
	import {
		checkRoomStrats,
		findBuiltInOption,
		listBuiltInOptions,
		listUserOptions,
		loadUserPresets,
		resolvePresetSkill
	} from './difficulty';
	import LogicNotes from './LogicNotes.svelte';
	import StratEntry from './StratEntry.svelte';
	import type { LogicRoom } from './types';

	/**
	 * A room's header, diagram, and strats; `room.strats` holds the strats to list. `showNodeFilter` adds
	 * controls to list only strats from or to a chosen node, passing with chosen skill settings, and, in overworld
	 * rooms, usable in a chosen world.
	 */
	let { room, showNodeFilter = false }: { room: LogicRoom; showNodeFilter?: boolean } = $props();

	const SHOW_DEV_NOTES_KEY = 'logic-show-dev-notes';
	const DIFFICULTY_KEY = 'logic-difficulty';

	const diagrams = $derived(findDiagrams(room.path, room.name));
	let selectedLabel = $state<string>();
	const diagram = $derived(
		diagrams.find((diagram) => diagram.label === selectedLabel) ?? selectDefaultDiagram(diagrams)
	);

	let fromNode = $state<number | null>(null);
	let toNode = $state<number | null>(null);
	let world = $state<'light' | 'dark' | null>(null);
	const hasWorlds = $derived(room.strats.some((strat) => strat.world !== undefined));

	/** The player's generation presets, loaded once mounted. */
	const presets = new SettingsStore(generationModel, 'logic-');
	const difficultyOptions = $derived([...listBuiltInOptions(), ...listUserOptions(presets)]);
	/** An option ID, or '' for strats of any difficulty. */
	let difficultyId = $state('');
	const difficulty = $derived(difficultyOptions.find((option) => option.id === difficultyId));
	/** IDs of the strats passing with the chosen difficulty, once checked. */
	let passingStrats = $state<Set<number> | null>(null);
	let difficultyError = $state<string | null>(null);

	$effect(() => {
		const option = difficulty;
		const roomId = room.id;
		passingStrats = null;
		difficultyError = null;
		if (!option) return;
		let current = true;
		(async () => {
			const skill = option.skill ?? (await resolvePresetSkill(presets, option.id));
			const listed = listUserOptions(presets).some((o) => o.id === option.id);
			if (skill !== null && !option.skill && !listed && difficultyId === option.id) {
				// Processing removed the preset's skill tweaks, so it takes a built-in option's settings.
				difficultyId = findBuiltInOption(skill)?.id ?? '';
				saveDifficulty();
				return;
			}
			if (!current) return;
			if (skill === null) {
				difficultyError = `Preset “${option.label}” couldn't be read.`;
				return;
			}
			const ids = await checkRoomStrats(roomId, skill);
			if (current) passingStrats = ids;
		})().catch((error) => {
			if (current) difficultyError = error.message;
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
		if (!showNodeFilter) return;
		let stored = '';
		try {
			stored = localStorage.getItem(DIFFICULTY_KEY) ?? '';
		} catch {
			// Storage unavailable; list strats of any difficulty.
		}
		loadUserPresets(presets);
		if (difficultyOptions.some((option) => option.id === stored)) difficultyId = stored;
	});

	function saveDifficulty() {
		try {
			localStorage.setItem(DIFFICULTY_KEY, difficultyId);
		} catch {
			// Storage unavailable; the choice lasts for this page only.
		}
	}

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
			<label for="difficulty">Difficulty</label>
			<select
				id="difficulty"
				class="form-select"
				bind:value={difficultyId}
				onchange={saveDifficulty}
			>
				<option value="">Any</option>
				{#each difficultyOptions as option (option.id)}
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
		{#if difficultyError}
			<div class="text-danger mt-2">{difficultyError}</div>
		{/if}
		{#if presets.notices.length}
			<div class="alert alert-info small py-2 px-3 mt-3 d-flex align-items-start" role="status">
				<div class="me-auto">
					<div class="fw-semibold">
						Some saved settings were updated for this version of the randomizer.
					</div>
					{#each presets.notices as group, i (i)}
						<div class="mt-1">{group.source}:</div>
						<ul class="mb-0">
							{#each group.issues as issue, j (j)}
								<li>
									{issue.path.length ? `${presets.label(issue.path)}: ` : ''}{issue.message}
								</li>
							{/each}
						</ul>
					{/each}
				</div>
				<button
					type="button"
					class="btn-close btn-sm"
					aria-label="Dismiss"
					onclick={() => (presets.notices = [])}
				></button>
			</div>
		{/if}
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
		grid-row: 2 / span 2;
	}
</style>
