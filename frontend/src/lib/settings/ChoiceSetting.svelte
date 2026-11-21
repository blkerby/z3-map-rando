<script lang="ts">
	import type { Snippet } from 'svelte';
	import Segmented from './controls/Segmented.svelte';
	import Swatches from './controls/Swatches.svelte';
	import SettingRow from './SettingRow.svelte';
	import { getCompact, getStore } from './context';
	import type { Path, Value } from './tree';
	import type { ChoiceOption } from './types';

	let {
		path,
		label,
		help,
		notes,
		options,
		display
	}: {
		path: Path;
		label: string;
		help?: Snippet;
		notes?: string;
		options: ChoiceOption[];
		/** Default: segmented buttons, or a dropdown for more than 5 options. */
		display?: 'segmented' | 'dropdown' | 'swatches';
	} = $props();

	const store = getStore();
	const compact = getCompact();
	const value = $derived(store.get(path) as string);
	const mode = $derived(display ?? (options.length > 5 ? 'dropdown' : 'segmented'));
	const format = (v: Value) => options.find((o) => o.value === v)?.label ?? String(v);
	const set = (v: string) => store.set(path, v);
</script>

<SettingRow {path} {label} {help} {notes} {format}>
	{#snippet control()}
		{#if mode === 'swatches'}
			<Swatches {options} {value} {label} disabled={store.locked} columns={Math.min(10, options.length)} onchange={set} />
		{:else if mode === 'dropdown'}
			<select class="form-select form-select-sm" class:w-auto={!compact} class:compact disabled={store.locked} {value} onchange={(e) => set(e.currentTarget.value)}>
				{#each options as o (o.value)}
					<option value={o.value}>{o.label}</option>
				{/each}
			</select>
		{:else}
			<Segmented {options} {value} {label} disabled={store.locked} onchange={set} />
		{/if}
	{/snippet}
</SettingRow>

<style>
	/* One width for every dropdown in compact rows, so a column of them lines up. */
	.compact {
		width: 9.5rem;
	}
</style>
