<script lang="ts">
	import RoomLogic from '$lib/logic/RoomLogic.svelte';

	let { data } = $props();
</script>

<svelte:head>
	<title>{data.room.strats[0].name} - {data.room.name} - Logic</title>
</svelte:head>

<RoomLogic room={data.room} />
