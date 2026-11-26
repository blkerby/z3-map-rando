<script lang="ts">
	import { resolve } from '$app/paths';
	import type { RoomCategory } from '$lib/logic/types';

	let { data } = $props();

	const sections: { category: RoomCategory; title: string }[] = [
		{ category: 'overworld', title: 'Overworld' },
		{ category: 'dungeon', title: 'Dungeons' },
		{ category: 'cave', title: 'Caves' },
		{ category: 'house', title: 'Houses' }
	];
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
		project. Each room page lists the strats for moving through the room and their requirements.
	</p>
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
</style>
