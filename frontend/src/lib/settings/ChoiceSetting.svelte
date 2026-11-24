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
		icon,
		options,
		display
	}: {
		path: Path;
		label: string;
		help?: Snippet;
		notes?: string;
		/** See `SettingRow`. */
		icon?: string | null;
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

<SettingRow {path} {label} {help} {notes} {icon} {format} wrap={mode === 'swatches'}>
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
	/* One width for every dropdown in compact rows, so a column of them lines up. The arrow sits closer to
	   the edge than Bootstrap's default, leaving more room for the label beside the dropdown. The width just
	   fits the longest option, "Green Potion". */
	.compact {
		width: 7.75rem;
		padding-right: 1.5rem;
		background-position: right 0.375rem center;
	}
</style>
