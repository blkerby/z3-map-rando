<script lang="ts">
	import RoomLogic from '$lib/logic/RoomLogic.svelte';

	let { data } = $props();
</script>

<svelte:head>
	<title>{data.room.name} - Logic</title>
</svelte:head>

<RoomLogic room={data.room} showNodeFilter />
