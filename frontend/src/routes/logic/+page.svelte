<script lang="ts">
	import { resolve } from '$app/paths';
	import overworldMap from '$lib/assets/overworld_map.png';
	import type { RoomCategory, RoomSummary, TechSummary } from '$lib/logic/types';

	let { data } = $props();

	let hoveredRoom = $state<RoomSummary | null>(null);
	const mapRooms = $derived(data.rooms.filter((room) => room.mapRect !== undefined));

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
	<div class="overworld-map my-3">
		<img src={overworldMap} alt="Light World overworld map" />
		<svg viewBox="0 0 256 256" aria-label="Overworld rooms">
			{#each mapRooms as room (room.id)}
				{@const rect = room.mapRect!}
				{@const label = room.darkWorldName ? `${room.name} / ${room.darkWorldName}` : room.name}
				<a
					href={resolve('/logic/[roomId]', { roomId: String(room.id) })}
					class="room-link"
					aria-label={label}
					onmouseenter={() => (hoveredRoom = room)}
					onmouseleave={() => (hoveredRoom = null)}
					onfocus={() => (hoveredRoom = room)}
					onblur={() => (hoveredRoom = null)}
				>
					<rect x={rect.x * 32} y={rect.y * 32} width={rect.width * 32} height={rect.height * 32} />
					<title>{label}</title>
				</a>
			{/each}
			{#if hoveredRoom?.mapRect}
				{@const rect = hoveredRoom.mapRect}
				<rect
					class="room-highlight"
					x={rect.x * 32}
					y={rect.y * 32}
					width={rect.width * 32}
					height={rect.height * 32}
				/>
			{/if}
		</svg>
	</div>
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
	/* Twice the PNG's 256px size, so pixels scale evenly. */
	.overworld-map {
		position: relative;
		width: 512px;
		max-width: 100%;
		margin: 0 auto;
	}

	.overworld-map img {
		display: block;
		width: 100%;
		image-rendering: pixelated;
	}

	.overworld-map svg {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
	}

	.room-link {
		outline: none;
	}

	.room-link rect {
		fill: transparent;
		stroke: rgb(180 180 180 / 0.75);
		stroke-width: 1;
		vector-effect: non-scaling-stroke;
	}

	.room-highlight {
		fill: rgb(255 255 255 / 0.2);
		stroke: #fff;
		stroke-width: 1.5;
		vector-effect: non-scaling-stroke;
		pointer-events: none;
	}

	.room-list {
		columns: 18rem;
	}

	.tiered-techs {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(14rem, 1fr));
		gap: 0 1.5rem;
	}
</style>
