<script lang="ts">
	import type { Snippet } from 'svelte';
	import Level from './controls/Level.svelte';
	import SettingRow from './SettingRow.svelte';
	import { getStore } from './context';
	import type { Path, Value } from './tree';
	import type { LevelOption } from './types';

	let {
		path,
		label,
		help,
		notes,
		levels
	}: {
		path: Path;
		label: string;
		help?: Snippet;
		notes?: string;
		/** The named steps of the slider, lowest first. */
		levels: LevelOption[];
	} = $props();

	const store = getStore();
	const format = (v: Value) => levels.find((l) => l.value === v)?.label ?? String(v);
</script>

<SettingRow {path} {label} {help} {notes} {format} wrap>
	{#snippet control()}
		<Level {levels} value={store.get(path) as number} {label} disabled={store.locked} onchange={(v) => store.set(path, v)} />
	{/snippet}
</SettingRow>
