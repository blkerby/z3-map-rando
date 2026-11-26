<script lang="ts">
	import NotableLogic from '$lib/logic/NotableLogic.svelte';

	let { data } = $props();
</script>

<svelte:head>
	<title>{data.notable.name} - Logic</title>
</svelte:head>

<NotableLogic notable={data.notable} />
