<script lang="ts">
	import { resolve } from '$app/paths';
	import LogicNotes from './LogicNotes.svelte';
	import type { LogicRoom, LogicStrat } from './types';

	let { room, strat, showDevNotes }: { room: LogicRoom; strat: LogicStrat; showDevNotes: boolean } =
		$props();

	const roomParams = $derived({ roomId: String(room.id) });
</script>

<div class="strat border-top py-3">
	<div class="row mb-2">
		<div class="col-sm-6">
			<a
				href={resolve('/logic/[roomId]/[stratId]', {
					...roomParams,
					stratId: String(strat.id)
				})}
			>
				{strat.name}
			</a>
			<br />
			<a href={resolve('/logic/[roomId]', roomParams)}>{room.name}</a>
		</div>
		<div class="col-6 col-sm-3">From: {strat.from.id}<br />{strat.from.name}</div>
		<div class="col-6 col-sm-3">To: {strat.to.id}<br />{strat.to.name}</div>
	</div>
	<LogicNotes notes={strat} {showDevNotes} />
	{#if strat.requires}
		<div class="label">Requires:</div>
		<pre>{strat.requires}</pre>
	{/if}
	{#each strat.properties as property (property.label)}
		<div class="label">{property.label}:</div>
		<pre>{property.text}</pre>
	{/each}
</div>

<style>
	.label {
		color: var(--bs-secondary-color);
	}

	pre {
		margin-bottom: 0.75rem;
	}

	.strat :global(p:last-child),
	.strat pre:last-child {
		margin-bottom: 0;
	}
</style>
