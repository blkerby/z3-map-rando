<script lang="ts">
	import Cards from './controls/Cards.svelte';
	import Segmented from './controls/Segmented.svelte';
	import Swatches from './controls/Swatches.svelte';
	import SettingRow from './SettingRow.svelte';
	import { getStore } from './context';
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
		help?: string;
		notes?: string;
		options: ChoiceOption[];
		/** Default: segmented buttons, or a dropdown for more than 5 options. 'cards' suits options with real consequences. */
		display?: 'segmented' | 'cards' | 'dropdown' | 'swatches';
	} = $props();

	const store = getStore();
	const value = $derived(store.get(path) as string);
	const mode = $derived(display ?? (options.length > 5 ? 'dropdown' : 'segmented'));
	const selectedDesc = $derived(options.find((o) => o.value === value)?.desc);
	const format = (v: Value) => options.find((o) => o.value === v)?.label ?? String(v);
	const set = (v: string) => store.set(path, v);
</script>

<SettingRow {path} {label} {help} {notes} {format}>
	{#snippet control()}
		{#if mode === 'cards'}
			<Cards {options} {value} {label} disabled={store.locked} onchange={set} />
		{:else if mode === 'swatches'}
			<Swatches {options} {value} {label} disabled={store.locked} columns={Math.min(12, options.length)} onchange={set} />
		{:else if mode === 'dropdown'}
			<select class="form-select form-select-sm w-auto" disabled={store.locked} {value} onchange={(e) => set(e.currentTarget.value)}>
				{#each options as o (o.value)}
					<option value={o.value}>{o.label}</option>
				{/each}
			</select>
		{:else}
			<Segmented {options} {value} {label} disabled={store.locked} onchange={set} />
		{/if}
		{#if selectedDesc && mode !== 'cards'}
			<div class="desc">{selectedDesc}</div>
		{/if}
	{/snippet}
</SettingRow>

<style>
	.desc {
		margin-top: 0.3rem;
		font-size: 0.82rem;
		opacity: 0.7;
	}
</style>
