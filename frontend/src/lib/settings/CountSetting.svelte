<script lang="ts">
	import NumberInput from './controls/NumberInput.svelte';
	import SettingRow from './SettingRow.svelte';
	import { getStore } from './context';
	import type { Path } from './tree';

	let {
		path,
		label,
		help,
		notes,
		min,
		max
	}: { path: Path; label: string; help?: string; notes?: string; min: number; max: number } = $props();

	const store = getStore();
</script>

<SettingRow {path} {label} {help} {notes} format={String}>
	{#snippet control()}
		<NumberInput value={store.get(path) as number} {min} {max} {label} disabled={store.locked} onchange={(v) => store.set(path, v)} />
	{/snippet}
</SettingRow>
