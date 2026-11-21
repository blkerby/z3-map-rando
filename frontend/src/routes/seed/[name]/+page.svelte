<script lang="ts">
	import SeedPage from '$lib/SeedPage.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<SeedPage seed={data.seed} />
