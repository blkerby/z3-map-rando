<script lang="ts">
	import type { Notes } from './types';

	let { notes, showDevNotes }: { notes: Notes; showDevNotes: boolean } = $props();
</script>

{#each notes.note as paragraph, i (i)}
	<p>{paragraph}</p>
{/each}
{#each notes.detailNote as paragraph, i (i)}
	<p>{paragraph}</p>
{/each}
{#if showDevNotes}
	{#each notes.devNote as paragraph, i (i)}
		<p class="text-body-secondary">Dev note: {paragraph}</p>
	{/each}
{/if}
