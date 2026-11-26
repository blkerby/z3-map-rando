<script lang="ts">
	import type { Notes } from './types';

	let { notes, showDevNotes }: { notes: Notes; showDevNotes: boolean } = $props();
</script>

{#if notes.note}
	<p>{notes.note}</p>
{/if}
{#if notes.detailNote}
	<p>{notes.detailNote}</p>
{/if}
{#if showDevNotes && notes.devNote}
	<p class="text-body-secondary">Dev note: {notes.devNote}</p>
{/if}
