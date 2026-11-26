<script lang="ts">
	import { resolve } from '$app/paths';
	import type { RoomCategory, TechSummary } from '$lib/logic/types';

	let { data } = $props();

	const sections: { category: RoomCategory; title: string }[] = [
		{ category: 'overworld', title: 'Overworld' },
		{ category: 'dungeon', title: 'Dungeons' },
		{ category: 'cave', title: 'Caves' },
		{ category: 'house', title: 'Houses' }
	];

	const techGroups = $derived.by(() => {
		const order = [...data.difficulties];
		for (const tech of data.techs) {
			if (!order.includes(tech.difficulty)) {
				order.push(tech.difficulty);
			}
		}
		const groups: { difficulty: string; techs: TechSummary[] }[] = [];
		for (const difficulty of order) {
			const techs = data.techs.filter((tech) => tech.difficulty === difficulty);
			if (techs.length > 0) {
				groups.push({ difficulty, techs });
			}
		}
		return groups;
	});
</script>

<svelte:head>
	<title>Logic</title>
</svelte:head>

<main class="container my-4">
	<h1 class="h2 mb-3">Logic</h1>
	<p>
		The randomizer's logic is based on the
		<a href="https://github.com/vg-json-data/z3-json-data" target="_blank" rel="noreferrer"
			>z3-json-data</a
		>
		project. Each room page lists the strats for moving through the room and their requirements, and each
		tech page lists the strats that depend on that tech.
	</p>
	<h2 class="h4 mt-4">Tech</h2>
	<div class="tiered-techs">
		{#each data.tieredTechs as tech (tech.id)}
			<div>
				<h3 class="h6 mt-3">{tech.name}:</h3>
				<ul class="list-unstyled">
					{#each tech.tiers as entry (entry.tier)}
						<li>
							<a
								href={resolve('/logic/tech/[techId]/[tier]', {
									techId: String(tech.id),
									tier: String(entry.tier)
								})}
							>
								{entry.tier}
							</a>
							<span class="text-body-secondary">({entry.difficulty})</span>
						</li>
					{/each}
				</ul>
			</div>
		{/each}
	</div>
	{#each techGroups as group (group.difficulty)}
		<h3 class="h6 mt-3">{group.difficulty}</h3>
		<ul class="room-list list-unstyled">
			{#each group.techs as tech (tech.id)}
				<li>
					<a href={resolve('/logic/tech/[techId]', { techId: String(tech.id) })}>
						{tech.name}
					</a>
					<span class="text-body-secondary">({tech.difficulty})</span>
				</li>
			{/each}
		</ul>
	{/each}
	{#each sections as section (section.category)}
		<h2 class="h4 mt-4">{section.title}</h2>
		<ul class="room-list list-unstyled">
			{#each data.rooms.filter((room) => room.category === section.category) as room (room.id)}
				<li>
					<a href={resolve('/logic/[roomId]', { roomId: String(room.id) })}>
						{room.name}
					</a>
					{#if room.darkWorldName}
						<span class="text-body-secondary">/ {room.darkWorldName}</span>
					{/if}
				</li>
			{/each}
		</ul>
	{/each}
</main>

<style>
	.room-list {
		columns: 18rem;
	}

	.tiered-techs {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
		gap: 0 1.5rem;
	}
</style>
