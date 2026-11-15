<script lang="ts">
	import GamePage from '$lib/GamePage.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<GamePage seed={data.seed} settings={data.settings} />
