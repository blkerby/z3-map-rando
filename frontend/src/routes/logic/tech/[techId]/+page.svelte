<script lang="ts">
	import TechLogic from '$lib/logic/TechLogic.svelte';

	let { data } = $props();
</script>

<svelte:head>
	<title>{data.tech.name} - Logic</title>
</svelte:head>

<TechLogic tech={data.tech} />
